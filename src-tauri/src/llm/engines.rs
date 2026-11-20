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

/// Pinned official llama.cpp release. Decision (architecture): engines download on first use,
/// verified against the checksums GitHub publishes for each release asset.
const BUILD: &str = "b12576";

#[derive(Clone, Copy)]
struct Asset {
    name: &'static str,
    sha256: &'static str,
    size: u64,
}

#[derive(Clone, Copy)]
struct EngineBuild {
    backend: Backend,
    label: &'static str,
    recommended: bool,
    assets: &'static [Asset],
}

#[cfg(all(windows, target_arch = "x86_64"))]
const BUILDS: &[EngineBuild] = &[
    EngineBuild {
        backend: Backend::Vulkan,
        label: "Vulkan (any GPU)",
        recommended: true,
        assets: &[Asset {
            name: "llama-b12576-bin-win-vulkan-x64.zip",
            sha256: "ea6a55397d6afebc39abd74a3c2628b4548ec4346d15d21000a38a7802187101",
            size: 38_713_184,
        }],
    },
    EngineBuild {
        backend: Backend::Cuda,
        label: "CUDA (NVIDIA only, slightly faster)",
        recommended: false,
        assets: &[
            Asset {
                name: "llama-b12576-bin-win-cuda-13.5-x64.zip",
                sha256: "f9748d9db4c5cf97326a5919905c379f9d2dd44a08ef7e1729675517cac519f6",
                size: 225_467_178,
            },
            Asset {
                name: "cudart-llama-bin-win-cuda-13.5-x64.zip",
                sha256: "c2bf4cc202bfbaaa7a0b75f5082457c3811326947bb216059b72839d844e9cba",
                size: 426_314_199,
            },
        ],
    },
    EngineBuild {
        backend: Backend::Cpu,
        label: "CPU only",
        recommended: false,
        assets: &[Asset {
            name: "llama-b12576-bin-win-cpu-x64.zip",
            sha256: "c8bda771161ab75e259568ea178122b7567fb2d51c1a5ac755d308897e9b1c20",
            size: 24_875_631,
        }],
    },
];

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
const BUILDS: &[EngineBuild] = &[
    EngineBuild {
        backend: Backend::Vulkan,
        label: "Vulkan (any GPU)",
        recommended: true,
        assets: &[Asset {
            name: "llama-b12576-bin-ubuntu-vulkan-x64.tar.gz",
            sha256: "06a6bf56e34204f1f107d1f7a76347846fb5783f9f52cb2b6a8bb0ead65d8908",
            size: 36_738_274,
        }],
    },
    EngineBuild {
        backend: Backend::Cpu,
        label: "CPU only",
        recommended: false,
        assets: &[Asset {
            name: "llama-b12576-bin-ubuntu-x64.tar.gz",
            sha256: "b3e1ea8ff8723a17d312562625e9e531b10bed69222c49c53741d737d0214e31",
            size: 23_060_191,
        }],
    },
];

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
const BUILDS: &[EngineBuild] = &[EngineBuild {
    backend: Backend::Metal,
    label: "Metal (Apple GPU)",
    recommended: true,
    assets: &[Asset {
        name: "llama-b12576-bin-macos-arm64.tar.gz",
        sha256: "69da1ce8b38945939f4ad040637f762495f7b00f6d22ad88e3c874f4f66b8c20",
        size: 16_451_935,
    }],
}];

#[cfg(all(target_os = "macos", target_arch = "x86_64"))]
const BUILDS: &[EngineBuild] = &[EngineBuild {
    backend: Backend::Cpu,
    label: "CPU only",
    recommended: true,
    assets: &[Asset {
        name: "llama-b12576-bin-macos-x64.tar.gz",
        sha256: "d7ed21b59e6587159582bbcfc43f078c9aa40cf396b84f011977798ced914213",
        size: 12_230_057,
    }],
}];

#[cfg(not(any(
    all(windows, target_arch = "x86_64"),
    all(target_os = "linux", target_arch = "x86_64"),
    all(
        target_os = "macos",
        any(target_arch = "aarch64", target_arch = "x86_64")
    ),
)))]
const BUILDS: &[EngineBuild] = &[];

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

fn extract(archive: &Path, dest: &Path) -> Result<(), String> {
    let file = std::fs::File::open(archive).map_err(|e| e.to_string())?;
    let name = archive.to_string_lossy();
    if name.ends_with(".zip") {
        zip::ZipArchive::new(file)
            .map_err(|e| e.to_string())?
            .extract(dest)
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

#[tauri::command]
#[specta::specta]
pub fn llm_engines(app: AppHandle) -> Result<Vec<EngineStatus>, String> {
    let engines = engines_dir(&app)?;
    Ok(BUILDS
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
    let build = BUILDS
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
            url: format!(
                "https://github.com/ggml-org/llama.cpp/releases/download/{BUILD}/{}",
                asset.name
            ),
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
        for b in BUILDS {
            for a in b.assets {
                assert_eq!(a.sha256.len(), 64, "{}", a.name);
            }
        }
    }
}
