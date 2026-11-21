use crate::llm::download::{fetch_verified, DownloadEvent, Fetch};
use crate::llm::engine::Backend;
use serde::Serialize;
use specta::Type;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tauri::ipc::Channel;
use tauri::{AppHandle, State};

/// Pinned stable-diffusion.cpp release; digests are the ones GitHub publishes per asset.
const BUILD: &str = "master-1106-532d2c1";

#[derive(Clone, Copy)]
struct Build {
    backend: Backend,
    label: &'static str,
    recommended: bool,
    asset: &'static str,
    sha256: &'static str,
    size: u64,
}

#[cfg(all(windows, target_arch = "x86_64"))]
const BUILDS: &[Build] = &[
    Build {
        backend: Backend::Vulkan,
        label: "Vulkan (any GPU)",
        recommended: true,
        asset: "sd-master-532d2c1-bin-win-vulkan-x64.zip",
        sha256: "3e66222c87ca27074e1fd8ce79706eef17a8aaf39949906aa1bee7e060f22ecc",
        size: 30_312_081,
    },
    Build {
        backend: Backend::Cpu,
        label: "CPU only",
        recommended: false,
        asset: "sd-master-532d2c1-bin-win-cpu-x64.zip",
        sha256: "abef332cfca7861cc567910e79bedb70bfb2920bb4eedac97fcbb6db477f1596",
        size: 18_358_541,
    },
];

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
const BUILDS: &[Build] = &[
    Build {
        backend: Backend::Vulkan,
        label: "Vulkan (any GPU)",
        recommended: true,
        asset: "sd-master-532d2c1-bin-Linux-Ubuntu-26.04-x86_64-vulkan.zip",
        sha256: "ca30fe17f1735cdfb4c8561b75799070c25cc86a9e5438f4c0e1b163baae4e12",
        size: 42_212_329,
    },
    Build {
        backend: Backend::Cpu,
        label: "CPU only",
        recommended: false,
        asset: "sd-master-532d2c1-bin-Linux-Ubuntu-26.04-x86_64.zip",
        sha256: "0a02f7f51feb0691128f60cf6ff1c686ec23089561cfe7c3b72645f446646e13",
        size: 29_124_086,
    },
];

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
const BUILDS: &[Build] = &[Build {
    backend: Backend::Metal,
    label: "Metal (Apple GPU)",
    recommended: true,
    asset: "sd-master-532d2c1-bin-Darwin-macOS-26.7.2-arm64.zip",
    sha256: "8146d6ffc75f8ff8f64b62a80fffee8fb714ab65e368e202084135fa07cc6f36",
    size: 38_331_254,
}];

#[cfg(not(any(
    all(windows, target_arch = "x86_64"),
    all(target_os = "linux", target_arch = "x86_64"),
    all(target_os = "macos", target_arch = "aarch64"),
)))]
const BUILDS: &[Build] = &[];

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

/// The installed engine to run, preferring the order of BUILDS (GPU first).
pub fn find_cli(app: &AppHandle) -> Result<PathBuf, String> {
    let engines = crate::llm::paths::engines_dir(app)?;
    BUILDS
        .iter()
        .map(|b| install_dir(&engines, b.backend).join(exe_name()))
        .find(|p| p.exists())
        .ok_or_else(|| "The image engine is not installed yet".to_string())
}

#[tauri::command]
#[specta::specta]
pub fn image_engines(app: AppHandle) -> Result<Vec<ImageEngineStatus>, String> {
    let engines = crate::llm::paths::engines_dir(&app)?;
    Ok(BUILDS
        .iter()
        .map(|b| ImageEngineStatus {
            backend: b.backend,
            label: b.label.into(),
            size_bytes: b.size,
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
    let build = BUILDS
        .iter()
        .find(|b| b.backend == backend)
        .ok_or("This engine is not available on this system")?;
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

    let target = downloads.join(build.asset);
    let fetch = Fetch {
        url: format!(
            "https://github.com/leejet/stable-diffusion.cpp/releases/download/{BUILD}/{}",
            build.asset
        ),
        target: &target,
        sha256: build.sha256,
    };
    let fetched = fetch_verified(&fetch, 0, build.size, &cancel, &on_event).await;
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
        for b in BUILDS {
            assert_eq!(b.sha256.len(), 64, "{}", b.asset);
        }
    }
}
