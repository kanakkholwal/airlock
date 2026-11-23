use super::download::{fetch_verified, DownloadEvent, Fetch};
use super::engine::Backend;
use super::models::LlmModels;
use serde::Serialize;
use specta::Type;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tauri::ipc::Channel;
use tauri::{AppHandle, State};

use crate::data::{self, BuildSource as EngineBuild};

/// Decision (architecture): engines download on first use, verified against the checksums
/// GitHub publishes for each release asset. The pinned release and its assets live in
/// `catalog/engines.json`, one list per platform.
fn builds() -> &'static [EngineBuild] {
    data::engine("llama").map_or(&[], |e| e.builds)
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct EngineStatus {
    pub backend: Backend,
    pub label: String,
    pub size_bytes: u64,
    pub installed: bool,
    pub recommended: bool,
}

fn exe_name() -> &'static str {
    if cfg!(windows) {
        "llama-server.exe"
    } else {
        "llama-server"
    }
}

fn install_dir(engines: &Path, backend: Backend) -> PathBuf {
    engines.join(backend.dir_name())
}

fn engines_dir(app: &AppHandle) -> Result<PathBuf, String> {
    super::paths::engines_dir(app)
}

pub(crate) fn extract(archive: &Path, dest: &Path) -> Result<(), String> {
    let file = std::fs::File::open(archive).map_err(|e| e.to_string())?;
    let name = archive.to_string_lossy();
    if name.ends_with(".zip") {
        zip::ZipArchive::new(file)
            .map_err(|e| e.to_string())?
            .extract(dest)
            .map_err(|e| e.to_string())
    } else if name.ends_with(".tar.bz2") {
        tar::Archive::new(bzip2::read::BzDecoder::new(file))
            .unpack(dest)
            .map_err(|e| e.to_string())
    } else {
        tar::Archive::new(flate2::read::GzDecoder::new(file))
            .unpack(dest)
            .map_err(|e| e.to_string())
    }
}

/// Release tarballs wrap everything in one folder; move its contents up a level.
fn flatten_single_dir(dest: &Path) -> Result<(), String> {
    if dest.join(exe_name()).exists() {
        return Ok(());
    }
    let entries: Vec<_> = std::fs::read_dir(dest)
        .map_err(|e| e.to_string())?
        .flatten()
        .collect();
    if let [only] = entries.as_slice() {
        if only.path().is_dir() {
            for child in std::fs::read_dir(only.path())
                .map_err(|e| e.to_string())?
                .flatten()
            {
                std::fs::rename(child.path(), dest.join(child.file_name()))
                    .map_err(|e| e.to_string())?;
            }
            let _ = std::fs::remove_dir(only.path());
        }
    }
    Ok(())
}

pub fn is_installed(engines: &Path, backend: Backend) -> bool {
    install_dir(engines, backend).join(exe_name()).exists()
}

#[tauri::command]
#[specta::specta]
pub fn llm_engines(app: AppHandle) -> Result<Vec<EngineStatus>, String> {
    let engines = engines_dir(&app)?;
    Ok(builds()
        .iter()
        .map(|b| EngineStatus {
            backend: b.backend,
            label: b.label.into(),
            size_bytes: b.assets.iter().map(|a| a.size).sum(),
            installed: install_dir(&engines, b.backend).join(exe_name()).exists(),
            recommended: b.recommended,
        })
        .collect())
}

#[tauri::command]
#[specta::specta]
pub async fn llm_install_engine(
    app: AppHandle,
    models: State<'_, LlmModels>,
    backend: Backend,
    on_event: Channel<DownloadEvent>,
) -> Result<(), String> {
    let source = data::engine("llama").ok_or("This engine is not available on this system")?;
    let build = source
        .builds
        .iter()
        .find(|b| b.backend == backend)
        .ok_or("This engine is not available on this system")?;
    let engines = engines_dir(&app)?;
    let downloads = engines.join(".downloads");
    std::fs::create_dir_all(&downloads).map_err(|e| e.to_string())?;
    let staging = engines.join(format!(".{}.staging", backend.dir_name()));
    let _ = std::fs::remove_dir_all(&staging);

    let key = format!("engine:{}", backend.dir_name());
    let cancel = Arc::new(AtomicBool::new(false));
    models
        .cancels
        .lock()
        .unwrap()
        .insert(key.clone(), cancel.clone());

    let total: u64 = build.assets.iter().map(|a| a.size).sum();
    let mut base = 0;
    let mut outcome: Result<bool, String> = Ok(true);
    for asset in build.assets {
        let target = downloads.join(asset.name);
        let fetch = Fetch {
            url: source.asset_url(asset),
            target: &target,
            sha256: asset.sha256,
        };
        match fetch_verified(&fetch, base, total, &cancel, &on_event).await {
            Ok(true) => {
                base += asset.size;
                let (archive, dest) = (target.clone(), staging.clone());
                let extracted = tokio::task::spawn_blocking(move || extract(&archive, &dest))
                    .await
                    .map_err(|e| e.to_string())
                    .and_then(|r| r);
                if let Err(e) = extracted {
                    outcome = Err(e);
                    break;
                }
                let _ = std::fs::remove_file(&target);
            }
            other => {
                outcome = other;
                break;
            }
        }
    }
    models.cancels.lock().unwrap().remove(&key);

    let finish = match outcome {
        Ok(true) => flatten_single_dir(&staging).and_then(|_| {
            if !staging.join(exe_name()).exists() {
                return Err("The archive did not contain llama-server".into());
            }
            let dest = install_dir(&engines, backend);
            let _ = std::fs::remove_dir_all(&dest);
            std::fs::rename(&staging, &dest).map_err(|e| e.to_string())
        }),
        Ok(false) => {
            let _ = std::fs::remove_dir_all(&staging);
            let _ = on_event.send(DownloadEvent::Cancelled);
            return Ok(());
        }
        Err(e) => Err(e),
    };
    match finish {
        Ok(()) => {
            let _ = on_event.send(DownloadEvent::Done);
            Ok(())
        }
        Err(message) => {
            let _ = std::fs::remove_dir_all(&staging);
            let _ = on_event.send(DownloadEvent::Error {
                message: message.clone(),
            });
            Err(message)
        }
    }
}

#[tauri::command]
#[specta::specta]
pub fn llm_remove_engine(app: AppHandle, backend: Backend) -> Result<(), String> {
    let dir = install_dir(&engines_dir(&app)?, backend);
    if dir.exists() {
        std::fs::remove_dir_all(dir).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flatten_moves_a_single_wrapper_folder_up() {
        let tmp = tempfile::tempdir().unwrap();
        let inner = tmp.path().join("llama-b1");
        std::fs::create_dir_all(&inner).unwrap();
        std::fs::write(inner.join(exe_name()), b"x").unwrap();
        flatten_single_dir(tmp.path()).unwrap();
        assert!(tmp.path().join(exe_name()).exists());
        assert!(!inner.exists());
    }

    #[test]
    fn flatten_leaves_a_flat_archive_alone() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join(exe_name()), b"x").unwrap();
        std::fs::create_dir(tmp.path().join("sub")).unwrap();
        flatten_single_dir(tmp.path()).unwrap();
        assert!(tmp.path().join("sub").exists());
    }

    #[test]
    fn every_build_has_valid_checksums() {
        assert!(!builds().is_empty() || data::engine("llama").is_none());
        for b in builds() {
            for a in b.assets {
                assert_eq!(a.sha256.len(), 64, "{}", a.name);
            }
        }
    }
}
