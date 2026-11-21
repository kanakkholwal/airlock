use super::catalog::{self, ImageModel, MODELS};
use crate::llm::download::{fetch_verified, DownloadEvent, Fetch};
use serde::Serialize;
use specta::Type;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::ipc::Channel;
use tauri::{AppHandle, State};

#[derive(Debug, Clone, Serialize, Type)]
pub struct ImageModelStatus {
    pub id: String,
    pub name: String,
    pub description: String,
    pub size_bytes: u64,
    /// Bytes still to download, shared parts already on disk excluded.
    pub missing_bytes: u64,
    pub license: String,
    pub recommended: bool,
    pub downloaded: bool,
}

fn missing_bytes(dir: &Path, m: &ImageModel) -> u64 {
    m.files
        .iter()
        .filter(|f| !dir.join(f.name).exists())
        .map(|f| f.size_bytes)
        .sum()
}

pub fn is_downloaded(dir: &Path, m: &ImageModel) -> bool {
    missing_bytes(dir, m) == 0
}

#[tauri::command]
#[specta::specta]
pub fn image_models(app: AppHandle) -> Result<Vec<ImageModelStatus>, String> {
    let dir = super::models_dir(&app)?;
    Ok(MODELS
        .iter()
        .map(|m| {
            let missing = missing_bytes(&dir, m);
            ImageModelStatus {
                id: m.id.into(),
                name: m.name.into(),
                description: m.description.into(),
                size_bytes: m.total_bytes(),
                missing_bytes: missing,
                license: m.license.into(),
                recommended: m.recommended,
                downloaded: missing == 0,
            }
        })
        .collect())
}

#[tauri::command]
#[specta::specta]
pub async fn image_download(
    app: AppHandle,
    state: State<'_, super::ImageState>,
    id: String,
    on_event: Channel<DownloadEvent>,
) -> Result<(), String> {
    let model = catalog::find(&id).ok_or("Unknown image model")?;
    let dir = super::models_dir(&app)?;
    let key = format!("image:{id}");
    let cancel = Arc::new(AtomicBool::new(false));
    {
        let mut cancels = state.cancels.lock().unwrap();
        if cancels.contains_key(&key) {
            return Err("This model is already downloading".into());
        }
        cancels.insert(key.clone(), cancel.clone());
    }

    let total = missing_bytes(&dir, model);
    let free = crate::llm::models::free_disk_bytes(&dir).unwrap_or(u64::MAX);
    let outcome = if free < total {
        Err(format!(
            "Not enough disk space: {:.1} GB needed, {:.1} GB free",
            total as f64 / 1e9,
            free as f64 / 1e9
        ))
    } else {
        let mut base = 0;
        let mut result = Ok(true);
        for f in model.files.iter().filter(|f| !dir.join(f.name).exists()) {
            let target = dir.join(f.name);
            let fetch = Fetch {
                url: catalog::file_url(f),
                target: &target,
                sha256: f.sha256,
            };
            match fetch_verified(&fetch, base, total, &cancel, &on_event).await {
                Ok(true) => base += f.size_bytes,
                other => {
                    result = other;
                    break;
                }
            }
        }
        result
    };
    state.cancels.lock().unwrap().remove(&key);
    match outcome {
        Ok(true) => {
            let _ = on_event.send(DownloadEvent::Done);
            Ok(())
        }
        Ok(false) => {
            let _ = on_event.send(DownloadEvent::Cancelled);
            Ok(())
        }
        Err(message) => {
            let _ = on_event.send(DownloadEvent::Error {
                message: message.clone(),
            });
            Err(message)
        }
    }
}

#[tauri::command]
#[specta::specta]
pub fn image_download_cancel(state: State<'_, super::ImageState>, id: String) {
    let cancels = state.cancels.lock().unwrap();
    for key in [format!("image:{id}"), id] {
        if let Some(flag) = cancels.get(&key) {
            flag.store(true, Ordering::SeqCst);
        }
    }
}

/// Removes this model's files, keeping any part another downloaded model still uses.
#[tauri::command]
#[specta::specta]
pub fn image_delete_model(app: AppHandle, id: String) -> Result<(), String> {
    let model = catalog::find(&id).ok_or("Unknown image model")?;
    let dir = super::models_dir(&app)?;
    for f in model.files {
        let shared = MODELS
            .iter()
            .filter(|m| m.id != model.id && is_downloaded(&dir, m))
            .any(|m| m.files.iter().any(|g| g.name == f.name));
        if !shared {
            let _ = std::fs::remove_file(dir.join(f.name));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_bytes_ignores_files_already_present() {
        let tmp = tempfile::tempdir().unwrap();
        let m = &MODELS[0];
        let full = m.total_bytes();
        assert_eq!(missing_bytes(tmp.path(), m), full);
        let vae = m.files.iter().find(|f| f.name == "ae.safetensors").unwrap();
        std::fs::write(tmp.path().join(vae.name), b"x").unwrap();
        assert_eq!(missing_bytes(tmp.path(), m), full - vae.size_bytes);
    }
}
