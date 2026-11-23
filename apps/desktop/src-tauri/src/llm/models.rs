use super::arch::{self, ModelArch, MIN_CONTEXT};
use super::catalog::{self, CatalogEntry, CatalogFile};
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
    pub tags: Vec<String>,
    pub vision: bool,
    pub imported: bool,
    pub speed: Option<u8>,
    pub intelligence: Option<u8>,
    pub max_context: Option<u32>,
    /// Estimated memory to run it at the current settings, rounded up to whole GB.
    pub needs_gb: Option<u32>,
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

pub(crate) fn free_disk_bytes(dir: &Path) -> Option<u64> {
    let dir = dir.canonicalize().ok()?;
    let disks = sysinfo::Disks::new_with_refreshed_list().ok()?;
    disks
        .iter()
        .filter(|d| dir.starts_with(d.mount_point()))
        .max_by_key(|d| d.mount_point().as_os_str().len())
        .map(|d| d.available_space())
}

/// Imported files that carry no readable architecture: the old flat cache guess.
pub const UNKNOWN_ARCH: ModelArch = ModelArch {
    max_context: MIN_CONTEXT,
    kv_kib_per_token: 128,
    kv_fixed_mib: 0,
};

/// Chat length the memory figures are quoted at: the user's override, else the smallest useful.
fn effective_context(tuning: &super::engine::Tuning) -> u32 {
    tuning.context.unwrap_or(MIN_CONTEXT)
}

/// Weights, architecture and the model's own context for a downloaded model id.
pub fn profile(dir: &Path, id: &str) -> Option<(u64, ModelArch)> {
    let (path, projector, _) = resolve(dir, id)?;
    let bytes = |p: &Path| std::fs::metadata(p).map(|m| m.len()).unwrap_or(0);
    let weights = bytes(&path) + projector.as_deref().map_or(0, bytes);
    let found = match catalog::find(id) {
        Some(entry) => entry.arch(),
        None => arch::from_gguf(&path).unwrap_or(UNKNOWN_ARCH),
    };
    Some((weights, found))
}

pub(super) fn usable_memory(app: &AppHandle) -> u64 {
    usable_memory_bytes(app)
}

fn gb_up(bytes: u64) -> u32 {
    bytes.div_ceil(1_073_741_824) as u32
}

fn free_memory_bytes() -> u64 {
    let Ok(mut sys) = sysinfo::System::new() else {
        return 0;
    };
    sys.refresh_memory();
    sys.available_memory()
}

fn imported_models(dir: &Path, tuning: &super::engine::Tuning) -> Vec<ModelStatus> {
    let Ok(read) = std::fs::read_dir(dir.join("imported")) else {
        return vec![];
    };
    read.flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "gguf"))
        .filter(|e| {
            !e.file_name()
                .to_string_lossy()
                .to_lowercase()
                .starts_with("mmproj")
        })
        .map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            // A note written by the Hugging Face download, when there is one.
            let note: serde_json::Value =
                std::fs::read_to_string(dir.join("imported").join(format!("{name}.json")))
                    .ok()
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or_default();
            let repo = note["repo"].as_str();
            let file_bytes = e.metadata().map(|m| m.len()).unwrap_or(0);
            let found = arch::from_gguf(&e.path());
            ModelStatus {
                id: format!("{IMPORT_PREFIX}{name}"),
                name: name.trim_end_matches(".gguf").to_string(),
                description: repo
                    .map(|r| format!("From huggingface.co/{r}"))
                    .unwrap_or_else(|| "Imported from a file.".into()),
                size_bytes: file_bytes,
                license: note["license"]
                    .as_str()
                    .map(str::to_string)
                    .unwrap_or_else(|| "Unknown, check the source".into()),
                license_url: repo.map(|r| format!("https://huggingface.co/{r}")),
                recommended: false,
                tags: vec![],
                vision: note["projector"].is_string(),
                imported: true,
                speed: None,
                intelligence: None,
                max_context: found.map(|a| a.max_context),
                needs_gb: Some(gb_up(found.unwrap_or(UNKNOWN_ARCH).memory_bytes(
                    file_bytes,
                    effective_context(tuning),
                    tuning,
                ))),
                file: FileState::Downloaded,
            }
        })
        .collect()
}

/// Resolves a model id to its weights file and optional projector, if fully on disk.
/// The projector file recorded next to an imported model, if it is on disk.
fn projector_of(dir: &Path, model_file: &str) -> Option<PathBuf> {
    let note: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(dir.join("imported").join(format!("{model_file}.json"))).ok()?,
    )
    .ok()?;
    let name = note["projector"].as_str()?;
    // Same rule as ids: a bare file name, never a path.
    if Path::new(name).file_name() != Some(std::ffi::OsStr::new(name)) {
        return None;
    }
    let path = dir.join("imported").join(name);
    path.exists().then_some(path)
}

pub fn resolve(dir: &Path, id: &str) -> Option<(PathBuf, Option<PathBuf>, u32)> {
    if let Some(name) = id.strip_prefix(IMPORT_PREFIX) {
        // Decision (security): an imported id is a bare file name; anything with a separator or
        // `..` could point outside the models folder.
        if Path::new(name).file_name() != Some(std::ffi::OsStr::new(name)) {
            return None;
        }
        let path = dir.join("imported").join(name);
        // A vision model downloaded from Hugging Face has its projector named in its note.
        let projector = projector_of(dir, name);
        return path.exists().then_some((path, projector, 4096));
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
    let tuning = super::paths::tuning(&app);
    let mut list: Vec<ModelStatus> = catalog::all()
        .iter()
        .map(|e| ModelStatus {
            id: e.id.into(),
            name: e.name.into(),
            description: e.description.into(),
            size_bytes: e.total_bytes(),
            license: e.license.into(),
            license_url: Some(e.license_url.into()),
            recommended: e.recommended,
            tags: e.tags.iter().map(|t| t.to_string()).collect(),
            vision: e.projector.is_some(),
            imported: false,
            speed: e.speed,
            intelligence: e.intelligence,
            max_context: e.max_context,
            needs_gb: Some(gb_up(e.arch().memory_bytes(
                e.total_bytes(),
                effective_context(&tuning),
                &tuning,
            ))),
            file: file_state(&dir, e),
        })
        .collect();
    list.extend(imported_models(&dir, &tuning));
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
        // Deleting a vision model takes its projector with it.
        if let Some(p) = projector_of(&dir, &name.to_string_lossy()) {
            let _ = std::fs::remove_file(p);
        }
        let _ = std::fs::remove_file(
            dir.join("imported")
                .join(format!("{}.json", name.to_string_lossy())),
        );
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

#[derive(Debug, Clone, Serialize, Type)]
pub struct SystemMemory {
    pub total_bytes: u64,
    pub free_bytes: u64,
    /// Free RAM plus most of the GPU's memory when a GPU engine will run the model.
    pub usable_bytes: u64,
}

/// Lets Explore rate a download against this machine before it starts.
#[tauri::command]
#[specta::specta]
pub fn llm_system_memory(app: AppHandle) -> SystemMemory {
    let total = sysinfo::System::new()
        .map(|mut s| {
            s.refresh_memory();
            s.total_memory()
        })
        .unwrap_or(0);
    SystemMemory {
        total_bytes: total,
        free_bytes: free_memory_bytes(),
        usable_bytes: usable_memory_bytes(&app),
    }
}

/// Share of a GPU's total memory counted as usable: the desktop and other apps hold the rest.
/// Free VRAM is not reported by the device probe, so this is a fixed share of the total.
const GPU_USABLE_SHARE: f64 = 0.8;

/// Free RAM plus the largest GPU's usable memory. CPU-only runs count RAM alone, since weights
/// then never reach the GPU.
fn usable_memory_bytes(app: &AppHandle) -> u64 {
    let ram = free_memory_bytes();
    if super::paths::engine_pref(app) == super::engine::Backend::Cpu {
        return ram;
    }
    let gpu = app
        .try_state::<std::sync::Arc<crate::managers::transcription::TranscriptionManager>>()
        .map(|tm| crate::managers::transcription::get_available_accelerators(&tm))
        .and_then(|a| a.gpu_devices.iter().map(|g| g.total_vram_mb).max())
        .unwrap_or(0) as u64
        * 1024
        * 1024;
    ram + (gpu as f64 * GPU_USABLE_SHARE) as u64
}

/// Compares estimated memory at the current settings with free RAM plus usable GPU memory.
#[tauri::command]
#[specta::specta]
pub fn llm_memory_check(app: AppHandle, id: String) -> Result<MemoryCheck, String> {
    let tuning = super::paths::tuning(&app);
    let dir = models_dir(&app)?;
    let (weights, found) = profile(&dir, &id).ok_or("Model is not on disk")?;
    let required = found.memory_bytes(weights, effective_context(&tuning), &tuning);
    let free = usable_memory_bytes(&app);
    Ok(MemoryCheck {
        required_bytes: required,
        free_bytes: free,
        fits: required <= free,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imported_ids_cannot_escape_the_models_folder() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("secret.gguf"), b"").unwrap();
        assert!(resolve(tmp.path(), "import:../secret.gguf").is_none());
        assert!(resolve(tmp.path(), r"import:..\secret.gguf").is_none());
        assert!(resolve(tmp.path(), "import:/etc/passwd").is_none());
    }

    #[test]
    fn imported_id_resolves_a_plain_file_name() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir(tmp.path().join("imported")).unwrap();
        std::fs::write(tmp.path().join("imported").join("a.gguf"), b"").unwrap();
        assert!(resolve(tmp.path(), "import:a.gguf").is_some());
    }
}

pub fn manage(app: &AppHandle) {
    app.manage(LlmModels::default());
}
