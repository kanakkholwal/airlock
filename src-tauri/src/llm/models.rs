use super::catalog::{self, CatalogEntry, CatalogFile, CATALOG};
use super::download::{fetch_verified, DownloadEvent, Fetch};
use serde::Serialize;
use specta::Type;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, State};

const IMPORT_PREFIX: &str = "import:";

#[derive(Debug, Clone, Serialize, Type)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum FileState {
    NotDownloaded,
    Partial { downloaded: u64 },
    Downloaded,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct ModelStatus {
    pub id: String,
    pub name: String,
    pub description: String,
    pub size_bytes: u64,
    pub license: String,
    pub license_url: Option<String>,
    pub recommended: bool,
    pub vision: bool,
    pub imported: bool,
    pub file: FileState,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct MemoryCheck {
    pub required_bytes: u64,
    pub free_bytes: u64,
    pub fits: bool,
}

#[derive(Default)]
pub struct LlmModels {
    pub(super) cancels: Mutex<HashMap<String, Arc<AtomicBool>>>,
}

pub fn models_dir(app: &AppHandle) -> Result<PathBuf, String> {
    super::paths::models_dir(app)
}

fn final_path(dir: &Path, f: &CatalogFile) -> PathBuf {
    dir.join(f.filename)
}

fn partial_path(dir: &Path, f: &CatalogFile) -> PathBuf {
    dir.join(format!("{}.partial", f.filename))
}

fn file_state(dir: &Path, entry: &CatalogEntry) -> FileState {
    let mut partial = 0;
    let mut complete = 0;
    for f in entry.files() {
        if final_path(dir, f).exists() {
            complete += 1;
        } else if let Ok(m) = std::fs::metadata(partial_path(dir, f)) {
            partial += m.len();
        }
    }
    if complete == entry.files().count() {
        FileState::Downloaded
    } else if partial > 0 || complete > 0 {
        FileState::Partial {
            downloaded: partial
                + entry
                    .files()
                    .filter(|f| final_path(dir, f).exists())
                    .map(|f| f.size_bytes)
                    .sum::<u64>(),
        }
    } else {
        FileState::NotDownloaded
    }
}

fn free_disk_bytes(dir: &Path) -> Option<u64> {
    let dir = dir.canonicalize().ok()?;
    let disks = sysinfo::Disks::new_with_refreshed_list().ok()?;
    disks
        .iter()
        .filter(|d| dir.starts_with(d.mount_point()))
        .max_by_key(|d| d.mount_point().as_os_str().len())
        .map(|d| d.available_space())
}

fn free_memory_bytes() -> u64 {
    let Ok(mut sys) = sysinfo::System::new() else {
        return 0;
    };
    sys.refresh_memory();
    sys.available_memory()
}

fn imported_models(dir: &Path) -> Vec<ModelStatus> {
    let Ok(read) = std::fs::read_dir(dir.join("imported")) else {
        return vec![];
    };
    read.flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "gguf"))
        .map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            ModelStatus {
                id: format!("{IMPORT_PREFIX}{name}"),
                name: name.trim_end_matches(".gguf").to_string(),
                description: "Imported from a file.".into(),
                size_bytes: e.metadata().map(|m| m.len()).unwrap_or(0),
                license: "Unknown, check the source".into(),
                license_url: None,
                recommended: false,
                vision: false,
                imported: true,
                file: FileState::Downloaded,
            }
        })
        .collect()
}

/// Resolves a model id to its weights file and optional projector, if fully on disk.
pub fn resolve(dir: &Path, id: &str) -> Option<(PathBuf, Option<PathBuf>, u32)> {
    if let Some(name) = id.strip_prefix(IMPORT_PREFIX) {
        let path = dir.join("imported").join(name);
        return path.exists().then_some((path, None, 4096));
    }
    let entry = catalog::find(id)?;
    let model = final_path(dir, &entry.model);
    let projector = entry.projector.map(|p| final_path(dir, &p));
    let complete = model.exists() && projector.as_ref().is_none_or(|p| p.exists());
    complete.then_some((model, projector, entry.context))
}

#[tauri::command]
#[specta::specta]
pub fn llm_catalog(app: AppHandle) -> Result<Vec<ModelStatus>, String> {
    let dir = models_dir(&app)?;
    let mut list: Vec<ModelStatus> = CATALOG
        .iter()
        .map(|e| ModelStatus {
            id: e.id.into(),
            name: e.name.into(),
            description: e.description.into(),
            size_bytes: e.total_bytes(),
            license: e.license.into(),
            license_url: Some(e.license_url.into()),
            recommended: e.recommended,
            vision: e.projector.is_some(),
            imported: false,
            file: file_state(&dir, e),
        })
        .collect();
    list.extend(imported_models(&dir));
    Ok(list)
}

#[tauri::command]
#[specta::specta]
pub async fn llm_download(
    app: AppHandle,
    models: State<'_, LlmModels>,
    id: String,
    on_event: Channel<DownloadEvent>,
) -> Result<(), String> {
    let entry = catalog::find(&id).ok_or("Unknown model")?;
    let dir = models_dir(&app)?;
    if let FileState::Downloaded = file_state(&dir, entry) {
        let _ = on_event.send(DownloadEvent::Done);
        return Ok(());
    }
    let missing = entry.total_bytes();
    if let Some(free) = free_disk_bytes(&dir) {
        if free < missing {
            let message = format!(
                "Needs {} MB of disk, {} MB free",
                missing / 1_048_576,
                free / 1_048_576
            );
            let _ = on_event.send(DownloadEvent::Error {
                message: message.clone(),
            });
            return Err(message);
        }
    }

    let cancel = Arc::new(AtomicBool::new(false));
    models
        .cancels
        .lock()
        .unwrap()
        .insert(id.clone(), cancel.clone());

    let mut base = 0;
    let total = entry.total_bytes();
    let mut outcome: Result<bool, String> = Ok(true);
    for f in entry.files() {
        let fetch = Fetch {
            url: format!(
                "https://huggingface.co/{}/resolve/main/{}",
                f.repo, f.filename
            ),
            target: &final_path(&dir, f),
            sha256: f.sha256,
        };
        match fetch_verified(&fetch, base, total, &cancel, &on_event).await {
            Ok(true) => base += f.size_bytes,
            other => {
                outcome = other;
                break;
            }
        }
    }
    models.cancels.lock().unwrap().remove(&id);
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
pub fn llm_download_cancel(models: State<'_, LlmModels>, id: String) {
    if let Some(flag) = models.cancels.lock().unwrap().get(&id) {
        flag.store(true, Ordering::SeqCst);
    }
}

#[tauri::command]
#[specta::specta]
pub fn llm_delete_model(app: AppHandle, id: String) -> Result<(), String> {
    let dir = models_dir(&app)?;
    if let Some(name) = id.strip_prefix(IMPORT_PREFIX) {
        let name = Path::new(name).file_name().ok_or("Invalid model name")?;
        return std::fs::remove_file(dir.join("imported").join(name)).map_err(|e| e.to_string());
    }
    let entry = catalog::find(&id).ok_or("Unknown model")?;
    for f in entry.files() {
        let _ = std::fs::remove_file(final_path(&dir, f));
        let _ = std::fs::remove_file(partial_path(&dir, f));
    }
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn llm_import_model(app: AppHandle, path: String) -> Result<String, String> {
    let src = PathBuf::from(path);
    if src.extension().is_none_or(|e| e != "gguf") {
        return Err("Only .gguf files can be imported".into());
    }
    let name = src.file_name().ok_or("Invalid file")?.to_owned();
    let dest_dir = models_dir(&app)?.join("imported");
    std::fs::create_dir_all(&dest_dir).map_err(|e| e.to_string())?;
    let dest = dest_dir.join(&name);
    tokio::task::spawn_blocking(move || std::fs::copy(&src, &dest).map(|_| ()))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    Ok(format!("{IMPORT_PREFIX}{}", name.to_string_lossy()))
}

/// Compares estimated resident memory with free RAM. VRAM is not checked yet.
#[tauri::command]
#[specta::specta]
pub fn llm_memory_check(app: AppHandle, id: String) -> Result<MemoryCheck, String> {
    let required = if let Some(entry) = catalog::find(&id) {
        catalog::estimated_memory_bytes(entry)
    } else {
        let dir = models_dir(&app)?;
        let (path, _, _) = resolve(&dir, &id).ok_or("Model is not on disk")?;
        std::fs::metadata(path).map_err(|e| e.to_string())?.len() + 256 * 1024 * 1024
    };
    let free = free_memory_bytes();
    Ok(MemoryCheck {
        required_bytes: required,
        free_bytes: free,
        fits: required <= free,
    })
}

pub fn manage(app: &AppHandle) {
    app.manage(LlmModels::default());
}
