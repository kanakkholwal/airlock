pub mod catalog;
pub mod engine;
pub mod generate;
pub mod models;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager};

#[derive(Default)]
pub struct ImageState {
    pub(crate) cancels: Mutex<HashMap<String, Arc<AtomicBool>>>,
    pub(crate) generating: AtomicBool,
    pub(crate) cancel_generation: AtomicBool,
}

pub fn manage(app: &AppHandle) {
    app.manage(ImageState::default());
}

/// Image weights sit beside the chat models, so one folder choice moves both.
pub fn models_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = crate::llm::paths::models_dir(app)?.join("image");
    std::fs::create_dir_all(&dir).map_err(|e| format!("Models folder unavailable: {e}"))?;
    Ok(dir)
}

pub fn gallery_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = crate::portable::app_data_dir(app)
        .map_err(|e| e.to_string())?
        .join("images");
    std::fs::create_dir_all(&dir).map_err(|e| format!("Image folder unavailable: {e}"))?;
    Ok(dir)
}
