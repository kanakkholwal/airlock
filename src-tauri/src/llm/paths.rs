use serde::{Deserialize, Serialize};
use specta::Type;
use std::path::{Path, PathBuf};
use tauri::AppHandle;

/// Decision (product): model and engine folders are user choices, stored in their own file so
/// the large shared settings struct stays untouched. Changing a folder does not move files.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct Stored {
    models_dir: Option<PathBuf>,
    engines_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct StoragePaths {
    pub models_dir: String,
    pub engines_dir: String,
    pub default_models_dir: String,
    pub default_engines_dir: String,
    pub models_custom: bool,
    pub engines_custom: bool,
}

fn data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = crate::portable::app_data_dir(app).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

fn file(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(data_dir(app)?.join("storage.json"))
}

fn load(app: &AppHandle) -> Stored {
    file(app)
        .ok()
        .and_then(|f| std::fs::read_to_string(f).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save(app: &AppHandle, stored: &Stored) -> Result<(), String> {
    let json = serde_json::to_string_pretty(stored).map_err(|e| e.to_string())?;
    std::fs::write(file(app)?, json).map_err(|e| e.to_string())
}

fn default_models(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(data_dir(app)?.join("models").join("llm"))
}

fn default_engines(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(data_dir(app)?.join("engines"))
}

pub fn models_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = match load(app).models_dir {
        Some(d) => d,
        None => default_models(app)?,
    };
    std::fs::create_dir_all(&dir).map_err(|e| format!("Models folder unavailable: {e}"))?;
    Ok(dir)
}

pub fn engines_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = match load(app).engines_dir {
        Some(d) => d,
        None => default_engines(app)?,
    };
    std::fs::create_dir_all(&dir).map_err(|e| format!("Engines folder unavailable: {e}"))?;
    Ok(dir)
}

/// Rejects relative paths and folders we cannot write to, before anything is saved.
fn validate(path: &Path) -> Result<PathBuf, String> {
    if !path.is_absolute() {
        return Err("Choose a full folder path".into());
    }
    std::fs::create_dir_all(path).map_err(|e| format!("Cannot use that folder: {e}"))?;
    let probe = path.join(".airlock-write-test");
    std::fs::write(&probe, b"").map_err(|e| format!("That folder is not writable: {e}"))?;
    let _ = std::fs::remove_file(probe);
    Ok(path.to_path_buf())
}

fn describe(app: &AppHandle) -> Result<StoragePaths, String> {
    let stored = load(app);
    let (dm, de) = (default_models(app)?, default_engines(app)?);
    Ok(StoragePaths {
        models_dir: stored
            .models_dir
            .clone()
            .unwrap_or_else(|| dm.clone())
            .to_string_lossy()
            .into_owned(),
        engines_dir: stored
            .engines_dir
            .clone()
            .unwrap_or_else(|| de.clone())
            .to_string_lossy()
            .into_owned(),
        default_models_dir: dm.to_string_lossy().into_owned(),
        default_engines_dir: de.to_string_lossy().into_owned(),
        models_custom: stored.models_dir.is_some(),
        engines_custom: stored.engines_dir.is_some(),
    })
}

#[tauri::command]
#[specta::specta]
pub fn llm_storage_paths(app: AppHandle) -> Result<StoragePaths, String> {
    describe(&app)
}

/// `None` resets that folder to its default.
#[tauri::command]
#[specta::specta]
pub fn llm_set_storage_paths(
    app: AppHandle,
    models_dir: Option<String>,
    engines_dir: Option<String>,
    change_models: bool,
    change_engines: bool,
) -> Result<StoragePaths, String> {
    let mut stored = load(&app);
    if change_models {
        stored.models_dir = models_dir.map(|p| validate(Path::new(&p))).transpose()?;
    }
    if change_engines {
        stored.engines_dir = engines_dir.map(|p| validate(Path::new(&p))).transpose()?;
    }
    save(&app, &stored)?;
    describe(&app)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_relative_paths() {
        assert!(validate(Path::new("models")).is_err());
    }

    #[test]
    fn accepts_a_writable_folder_and_creates_it() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("a").join("b");
        assert_eq!(validate(&target).unwrap(), target);
        assert!(target.is_dir());
        assert!(!target.join(".airlock-write-test").exists());
    }
}
