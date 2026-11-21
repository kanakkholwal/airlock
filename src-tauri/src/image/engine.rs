use crate::llm::download::{fetch_verified, DownloadEvent, Fetch};
use crate::llm::engine::Backend;
use serde::Serialize;
use specta::Type;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tauri::ipc::Channel;
use tauri::{AppHandle, State};

use crate::data::{self, BuildSource};

/// The pinned stable-diffusion.cpp release and its per-platform assets are in
/// `catalog/engines.json`.
fn builds() -> &'static [BuildSource] {
    data::engine("sd").map_or(&[], |e| e.builds)
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct ImageEngineStatus {
    pub backend: Backend,
    pub label: String,
    pub size_bytes: u64,
    pub installed: bool,
    pub recommended: bool,
}

fn exe_name() -> &'static str {
    if cfg!(windows) {
        "sd-cli.exe"
    } else {
        "sd-cli"
    }
}

fn dir_name(backend: Backend) -> String {
    format!("sd-{backend:?}").to_lowercase()
}

fn install_dir(engines: &Path, backend: Backend) -> PathBuf {
    engines.join(dir_name(backend))
}

/// The installed engine to run, preferring the order in the catalog (GPU first).
pub fn find_cli(app: &AppHandle) -> Result<PathBuf, String> {
    let engines = crate::llm::paths::engines_dir(app)?;
    builds()
        .iter()
        .map(|b| install_dir(&engines, b.backend).join(exe_name()))
        .find(|p| p.exists())
        .ok_or_else(|| "The image engine is not installed yet".to_string())
}

#[tauri::command]
#[specta::specta]
pub fn image_engines(app: AppHandle) -> Result<Vec<ImageEngineStatus>, String> {
    let engines = crate::llm::paths::engines_dir(&app)?;
    Ok(builds()
        .iter()
        .map(|b| ImageEngineStatus {
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
pub async fn image_install_engine(
    app: AppHandle,
    state: State<'_, super::ImageState>,
    backend: Backend,
    on_event: Channel<DownloadEvent>,
) -> Result<(), String> {
    let source = data::engine("sd").ok_or("This engine is not available on this system")?;
    let build = source
        .builds
        .iter()
        .find(|b| b.backend == backend)
        .ok_or("This engine is not available on this system")?;
    let asset = build.assets.first().ok_or("This engine has no download")?;
    let engines = crate::llm::paths::engines_dir(&app)?;
    let downloads = engines.join(".downloads");
    std::fs::create_dir_all(&downloads).map_err(|e| e.to_string())?;
    let staging = engines.join(format!(".{}.staging", dir_name(backend)));
    let _ = std::fs::remove_dir_all(&staging);

    let key = format!("engine:{}", dir_name(backend));
    let cancel = Arc::new(AtomicBool::new(false));
    state
        .cancels
        .lock()
        .unwrap()
        .insert(key.clone(), cancel.clone());

    let target = downloads.join(asset.name);
    let fetch = Fetch {
        url: source.asset_url(asset),
        target: &target,
        sha256: asset.sha256,
    };
    let fetched = fetch_verified(&fetch, 0, asset.size, &cancel, &on_event).await;
    state.cancels.lock().unwrap().remove(&key);

    let result = match fetched {
        Ok(true) => {
            let (archive, dest) = (target.clone(), staging.clone());
            tokio::task::spawn_blocking(move || crate::llm::engines::extract(&archive, &dest))
                .await
                .map_err(|e| e.to_string())
                .and_then(|r| r)
                .and_then(|_| {
                    if !staging.join(exe_name()).exists() {
                        return Err("The archive did not contain sd-cli".into());
                    }
                    let dest = install_dir(&engines, backend);
                    let _ = std::fs::remove_dir_all(&dest);
                    std::fs::rename(&staging, &dest).map_err(|e| e.to_string())
                })
        }
        Ok(false) => {
            let _ = std::fs::remove_dir_all(&staging);
            let _ = on_event.send(DownloadEvent::Cancelled);
            return Ok(());
        }
        Err(e) => Err(e),
    };
    let _ = std::fs::remove_file(&target);
    match result {
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
pub fn image_remove_engine(app: AppHandle, backend: Backend) -> Result<(), String> {
    let dir = install_dir(&crate::llm::paths::engines_dir(&app)?, backend);
    if dir.exists() {
        std::fs::remove_dir_all(dir).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_folders_do_not_clash_with_llama() {
        assert_eq!(dir_name(Backend::Vulkan), "sd-vulkan");
    }

    #[test]
    fn every_build_has_a_valid_checksum() {
        for b in builds() {
            for a in b.assets {
                assert_eq!(a.sha256.len(), 64, "{}", a.name);
            }
        }
    }
}
