use super::permission::{Mode, Rule};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::path::PathBuf;
use tauri::AppHandle;

/// What the assistant may do. Decision (product): privacy first, so no folder is shared until
/// the person adds one, and every tool can be switched off.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct AgentConfig {
    pub enabled: bool,
    /// Folders the file tools may read. Always stored as full, existing paths.
    pub folders: Vec<String>,
    pub disabled_tools: Vec<String>,
    #[serde(default)]
    pub mode: Mode,
    /// Rules the person wrote. `deny` beats `ask` beats `allow`.
    #[serde(default)]
    pub rules: Vec<Rule>,
    /// "Always allow" answers remembered from approval cards; the person can remove them.
    #[serde(default)]
    pub grants: Vec<Rule>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            folders: vec![],
            disabled_tools: vec![],
            mode: Mode::Ask,
            rules: vec![],
            grants: vec![],
        }
    }
}

fn file(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = crate::portable::app_data_dir(app).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("agent.json"))
}

pub fn load(app: &AppHandle) -> AgentConfig {
    file(app)
        .ok()
        .and_then(|f| std::fs::read_to_string(f).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// Keeps only folders that exist, as canonical paths, without duplicates.
fn clean_folders(folders: Vec<String>) -> Vec<String> {
    let mut out: Vec<String> = vec![];
    for f in folders {
        if let Ok(canon) = std::fs::canonicalize(&f) {
            if canon.is_dir() {
                let text = canon.to_string_lossy().into_owned();
                if !out.contains(&text) {
                    out.push(text);
                }
            }
        }
    }
    out
}

pub fn save(app: &AppHandle, config: &AgentConfig) -> Result<(), String> {
    let json = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    std::fs::write(file(app)?, json).map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub fn agent_get_config(app: AppHandle) -> AgentConfig {
    load(&app)
}

#[tauri::command]
#[specta::specta]
pub fn agent_set_config(app: AppHandle, config: AgentConfig) -> Result<AgentConfig, String> {
    let clean = AgentConfig {
        folders: clean_folders(config.folders),
        ..config
    };
    save(&app, &clean)?;
    Ok(clean)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folders_are_checked_and_deduplicated() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().to_string_lossy().into_owned();
        let kept = clean_folders(vec![dir.clone(), dir, "/definitely/not/here".into()]);
        assert_eq!(kept.len(), 1);
    }

    #[test]
    fn nothing_is_shared_by_default() {
        assert!(AgentConfig::default().folders.is_empty());
    }
}
