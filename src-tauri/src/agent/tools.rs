use super::config::{self, AgentConfig};
use serde::Serialize;
use serde_json::{json, Value};
use specta::Type;
use std::path::{Path, PathBuf};
use futures_util::future::BoxFuture;
use futures_util::FutureExt;
use tauri::{AppHandle, Manager};

const MAX_OUTPUT_CHARS: usize = 8_000;
const MAX_FILE_BYTES: u64 = 256 * 1024;

/// How much a call needs the person's say. Decision (product): reading public facts is free,
/// reading private text asks first, and anything that writes asks first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum Risk {
    Read,
    Ask,
    Write,
}

/// A tool runs on its own task, so a slow one (transcribing, summarizing) never blocks the app.
pub type RunFuture = BoxFuture<'static, Result<String, String>>;

pub struct ToolDef {
    pub name: &'static str,
    pub description: &'static str,
    pub risk: Risk,
    /// Only offered once at least one folder is shared.
    pub needs_folder: bool,
    pub parameters: fn() -> Value,
    /// What the call does in words, shown in chat and in the approval card.
    pub describe: fn(&Value) -> String,
    pub run: fn(AppHandle, AgentConfig, Value) -> RunFuture,
}

const AUDIO_EXTENSIONS: &[&str] = &["mp3", "wav", "m4a", "aac", "flac", "ogg", "opus", "webm", "mp4", "mov", "mkv", "aiff"];

pub const TOOLS: &[ToolDef] = &[
    ToolDef {
        name: "get_time",
        description: "Get the current local date, time and weekday.",
        risk: Risk::Read,
        needs_folder: false,
        parameters: || json!({ "type": "object", "properties": {} }),
        describe: |_| "Checking the time".into(),
        run: |_, _, _| async { Ok(chrono::Local::now().format("%A, %d %B %Y, %H:%M (%:z)").to_string()) }.boxed(),
    },
    ToolDef {
        name: "list_folder",
        description: "List the files and folders inside a folder the user has shared. Omit the path to list the first shared folder.",
        risk: Risk::Read,
        needs_folder: true,
        parameters: || {
            json!({
                "type": "object",
                "properties": { "path": { "type": "string", "description": "Folder path, full or relative to a shared folder." } }
            })
        },
        describe: |a| format!("Listing {}", a["path"].as_str().unwrap_or("the shared folder")),
        run: |app, config, args| async move { list_folder(&app, &config, &args) }.boxed(),
    },
    ToolDef {
        name: "read_text_file",
        description: "Read a plain text file from a folder the user has shared (notes, code, markdown). Large files are cut short.",
        risk: Risk::Read,
        needs_folder: true,
        parameters: || {
            json!({
                "type": "object",
                "properties": { "path": { "type": "string", "description": "File path, full or relative to a shared folder." } },
                "required": ["path"]
            })
        },
        describe: |a| format!("Reading {}", file_label(a["path"].as_str().unwrap_or("a file"))),
        run: |app, config, args| async move { read_text_file(&app, &config, &args) }.boxed(),
    },
    ToolDef {
        name: "search_library",
        description: "Search the user's library of recordings, transcribed files, notes, summaries and images by words in the title or start of the text. Returns ids with short previews only. To read one in full, use read_item with its id.",
        risk: Risk::Read,
        needs_folder: false,
        parameters: || {
            json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Words to look for. Leave empty to list the newest." },
                    "kind": { "type": "string", "enum": ["recording", "transcript", "note", "summary", "image"], "description": "Only this kind." },
                    "limit": { "type": "integer", "description": "How many results, at most 10." }
                }
            })
        },
        describe: |a| match a["query"].as_str().filter(|q| !q.is_empty()) {
            Some(q) => format!("Searching the library for \"{q}\""),
            None => "Looking through the library".into(),
        },
        run: |app, _, args| async move { search_library(&app, &args) }.boxed(),
    },
    ToolDef {
        name: "read_item",
        description: "Read the full text of one library item by its id, such as recording:12, transcript:30, note:plan or summary:4. The user is asked before it is shared.",
        risk: Risk::Ask,
        needs_folder: false,
        parameters: || {
            json!({
                "type": "object",
                "properties": { "id": { "type": "string", "description": "The library id, from search_library or an attached item." } },
                "required": ["id"]
            })
        },
        describe: |a| format!("Reading the full text of {}", a["id"].as_str().unwrap_or("an item")),
        run: |app, _, args| async move { read_item(&app, &args) }.boxed(),
    },
    ToolDef {
        name: "summarize_item",
        description: "Write a short summary of one library item with the local model, then return it. The summary is saved in the library. The user is asked first.",
        risk: Risk::Ask,
        needs_folder: false,
        parameters: || {
            json!({
                "type": "object",
                "properties": { "id": { "type": "string", "description": "The library id, such as recording:12." } },
                "required": ["id"]
            })
        },
        describe: |a| format!("Summarizing {}", a["id"].as_str().unwrap_or("an item")),
        run: |app, _, args| async move { summarize_item(&app, &args).await }.boxed(),
    },
    ToolDef {
        name: "transcribe_file",
        description: "Turn an audio or video file from a shared folder into text on this computer. The text is added to the library. The user approves each file.",
        risk: Risk::Write,
        needs_folder: true,
        parameters: || {
            json!({
                "type": "object",
                "properties": { "path": { "type": "string", "description": "Audio or video file, full or relative to a shared folder." } },
                "required": ["path"]
            })
        },
        describe: |a| format!("Transcribing {}", file_label(a["path"].as_str().unwrap_or("a file"))),
        run: |app, config, args| async move { transcribe_file(&app, &config, &args).await }.boxed(),
    },
    ToolDef {
        name: "save_note",
        description: "Save a note as a markdown file in the user's Airlock notes folder. The user approves each save.",
        risk: Risk::Write,
        needs_folder: false,
        parameters: || {
            json!({
                "type": "object",
                "properties": {
                    "title": { "type": "string", "description": "Short title, used for the file name." },
                    "text": { "type": "string", "description": "The note, in markdown." }
                },
                "required": ["title", "text"]
            })
        },
        describe: |a| format!("Saving the note \"{}\"", a["title"].as_str().unwrap_or("Untitled")),
        run: |app, config, args| async move { save_note(&app, &config, &args) }.boxed(),
    },
];

pub fn find(name: &str) -> Option<&'static ToolDef> {
    TOOLS.iter().find(|t| t.name == name)
}

/// The tools a model is offered right now.
pub fn active(config: &AgentConfig) -> Vec<&'static ToolDef> {
    if !config.enabled {
        return vec![];
    }
    TOOLS
        .iter()
        .filter(|t| !config.disabled_tools.iter().any(|d| d == t.name))
        .filter(|t| !t.needs_folder || !config.folders.is_empty())
        .collect()
}

/// The OpenAI-style `tools` array for a request.
pub fn specs(tools: &[&ToolDef]) -> Vec<Value> {
    tools
        .iter()
        .map(|t| {
            json!({
                "type": "function",
                "function": { "name": t.name, "description": t.description, "parameters": (t.parameters)() }
            })
        })
        .collect()
}

/// Cuts long output at a character boundary and says so, so a model never mistakes it for all.
pub fn clip(text: String) -> String {
    if text.chars().count() <= MAX_OUTPUT_CHARS {
        return text;
    }
    let cut: String = text.chars().take(MAX_OUTPUT_CHARS).collect();
    format!("{cut}\n[Cut short: the rest is not shown.]")
}

fn file_label(path: &str) -> String {
    path.rsplit(['/', '\\']).next().unwrap_or(path).to_string()
}

/// Turns a requested path into a real one, only if it is inside a shared folder. Relative paths
/// start from the first shared folder; `..` and links that leave the folders are refused.
pub fn resolve_allowed(roots: &[PathBuf], requested: &str) -> Result<PathBuf, String> {
    let first = roots
        .first()
        .ok_or("No folder is shared yet. Add one in Settings, Assistant.")?;
    let wanted = Path::new(requested);
    let joined = if requested.trim().is_empty() {
        first.clone()
    } else if wanted.is_absolute() {
        wanted.to_path_buf()
    } else {
        first.join(wanted)
    };
    let real = std::fs::canonicalize(&joined).map_err(|_| format!("Nothing found at {requested}"))?;
    let inside = roots
        .iter()
        .filter_map(|r| std::fs::canonicalize(r).ok())
        .any(|r| real.starts_with(&r));
    if inside {
        Ok(real)
    } else {
        Err("That is outside the folders you shared.".into())
    }
}

fn roots(config: &AgentConfig) -> Vec<PathBuf> {
    config.folders.iter().map(PathBuf::from).collect()
}

fn list_folder(_: &AppHandle, config: &AgentConfig, args: &Value) -> Result<String, String> {
    let dir = resolve_allowed(&roots(config), args["path"].as_str().unwrap_or(""))?;
    if !dir.is_dir() {
        return Err("That is a file, not a folder.".into());
    }
    let mut lines = vec![];
    for entry in std::fs::read_dir(&dir).map_err(|e| e.to_string())?.flatten().take(200) {
        let meta = entry.metadata().ok();
        let name = entry.file_name().to_string_lossy().into_owned();
        match meta {
            Some(m) if m.is_dir() => lines.push(format!("{name}/")),
            Some(m) => lines.push(format!("{name} ({} bytes)", m.len())),
            None => lines.push(name),
        }
    }
    lines.sort();
    Ok(clip(format!("{}\n{}", dir.display(), lines.join("\n"))))
}

fn read_text_file(_: &AppHandle, config: &AgentConfig, args: &Value) -> Result<String, String> {
    let path = resolve_allowed(&roots(config), args["path"].as_str().ok_or("Give a file path")?)?;
    let meta = std::fs::metadata(&path).map_err(|e| e.to_string())?;
    if !meta.is_file() {
        return Err("That is a folder, not a file.".into());
    }
    if meta.len() > MAX_FILE_BYTES {
        return Err(format!("That file is too large to read ({} KB).", meta.len() / 1024));
    }
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    let text = String::from_utf8(bytes).map_err(|_| "That is not a plain text file.".to_string())?;
    Ok(clip(text))
}

fn search_library(app: &AppHandle, args: &Value) -> Result<String, String> {
    let query = args["query"].as_str().unwrap_or("").trim();
    let kind = args["kind"].as_str().unwrap_or("");
    let limit = args["limit"].as_i64().unwrap_or(5).clamp(1, 10) as usize;
    let found = crate::library::search(app, kind, query, limit)?;
    if found.is_empty() {
        return Ok("Nothing in the library matched.".into());
    }
    Ok(found
        .iter()
        .map(|a| format!("{} · {} · {} · {}", a.id, a.kind, a.title, a.preview))
        .collect::<Vec<_>>()
        .join("\n"))
}

fn read_item(app: &AppHandle, args: &Value) -> Result<String, String> {
    use crate::library::privacy::{visible_text, Grant, Provider};
    let id = args["id"].as_str().ok_or("Give a library id")?;
    // Every read of a person's text goes through the privacy gateway, which applies their flags
    // and writes the privacy log.
    crate::library::refresh(app)?;
    Ok(clip(visible_text(app, id, Provider::Local, Grant::Full)?))
}

async fn summarize_item(app: &AppHandle, args: &Value) -> Result<String, String> {
    let id = args["id"].as_str().ok_or("Give a library id")?;
    crate::library::refresh(app)?;
    let (summary_id, text) = crate::library::jobs::summarize_and_wait(app, id).await?;
    Ok(clip(format!("Summary saved as {summary_id}:\n{text}")))
}

async fn transcribe_file(app: &AppHandle, config: &AgentConfig, args: &Value) -> Result<String, String> {
    use crate::managers::history::HistoryManager;
    use crate::managers::transcription_jobs::TranscriptionJobManager;
    use std::sync::Arc;

    let path = resolve_allowed(&roots(config), args["path"].as_str().ok_or("Give a file path")?)?;
    let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    if !path.is_file() || !AUDIO_EXTENSIONS.contains(&ext.as_str()) {
        return Err("That is not an audio or video file Airlock can transcribe.".into());
    }
    let jobs = Arc::clone(&app.state::<Arc<TranscriptionJobManager>>());
    let entry = tauri::async_runtime::spawn_blocking(move || jobs.transcribe_file(path))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;

    let history = Arc::clone(&app.state::<Arc<HistoryManager>>());
    // Long recordings take a while; wait a few minutes, then hand back the id either way.
    for _ in 0..240 {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        let current = history.get_entry_by_id(entry.id).await.map_err(|e| e.to_string())?;
        if let Some(done) = current.filter(|e| !e.transcription_text.trim().is_empty()) {
            let _ = crate::library::refresh(app);
            return Ok(format!(
                "Transcribed. Library id: transcript:{}\nStarts: {}\nUse read_item or summarize_item with that id.",
                entry.id,
                preview(&done.transcription_text)
            ));
        }
    }
    Ok(format!(
        "Still transcribing in the background. It will appear in the library as transcript:{} when it is done.",
        entry.id
    ))
}

fn preview(text: &str) -> String {
    crate::library::preview_of(text)
}

/// A file name made only of safe characters.
pub fn slug(title: &str) -> String {
    let s: String = title
        .chars()
        .map(|c| if c.is_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
        .collect();
    let s = s.split('-').filter(|p| !p.is_empty()).collect::<Vec<_>>().join("-");
    let s: String = s.chars().take(48).collect();
    if s.is_empty() {
        "note".into()
    } else {
        s
    }
}

fn save_note(app: &AppHandle, _: &AgentConfig, args: &Value) -> Result<String, String> {
    let title = args["title"].as_str().unwrap_or("Untitled").trim();
    let text = args["text"].as_str().ok_or("Give the note text")?;
    let dir = crate::portable::app_data_dir(app)
        .map_err(|e| e.to_string())?
        .join("notes");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let name = format!("{}-{}.md", slug(title), chrono::Local::now().format("%Y%m%d-%H%M%S"));
    let path = dir.join(name);
    std::fs::write(&path, format!("# {title}\n\n{text}\n")).map_err(|e| e.to_string())?;
    Ok(format!("Saved to {}", path.display()))
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    pub risk: Risk,
    pub needs_folder: bool,
    pub enabled: bool,
}

#[tauri::command]
#[specta::specta]
pub fn agent_tools(app: AppHandle) -> Vec<ToolInfo> {
    let config = config::load(&app);
    TOOLS
        .iter()
        .map(|t| ToolInfo {
            name: t.name.into(),
            description: t.description.into(),
            risk: t.risk,
            needs_folder: t.needs_folder,
            enabled: !config.disabled_tools.iter().any(|d| d == t.name),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_must_stay_inside_the_shared_folders() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("shared");
        std::fs::create_dir_all(root.join("sub")).unwrap();
        std::fs::write(root.join("a.txt"), "hi").unwrap();
        std::fs::write(tmp.path().join("secret.txt"), "no").unwrap();
        let roots = vec![root.clone()];

        assert!(resolve_allowed(&roots, "a.txt").is_ok());
        assert!(resolve_allowed(&roots, "sub").is_ok());
        assert!(resolve_allowed(&roots, "").is_ok());
        assert!(resolve_allowed(&roots, "../secret.txt").is_err());
        assert!(resolve_allowed(&roots, &tmp.path().join("secret.txt").to_string_lossy()).is_err());
        assert!(resolve_allowed(&roots, "missing.txt").is_err());
        assert!(resolve_allowed(&[], "a.txt").is_err());
    }

    #[test]
    fn nothing_is_offered_when_the_assistant_is_off_or_a_tool_is_disabled() {
        let mut config = AgentConfig::default();
        assert!(active(&config).iter().all(|t| !t.needs_folder), "no folder, no file tools");
        config.enabled = false;
        assert!(active(&config).is_empty());
        config.enabled = true;
        config.disabled_tools = vec!["get_time".into()];
        assert!(active(&config).iter().all(|t| t.name != "get_time"));
    }

    #[test]
    fn file_tools_appear_once_a_folder_is_shared() {
        let config = AgentConfig {
            folders: vec!["/x".into()],
            ..AgentConfig::default()
        };
        assert!(active(&config).iter().any(|t| t.name == "read_text_file"));
    }

    #[test]
    fn private_and_writing_tools_ask_first() {
        assert_eq!(find("read_item").unwrap().risk, Risk::Ask);
        assert_eq!(find("summarize_item").unwrap().risk, Risk::Ask);
        assert_eq!(find("transcribe_file").unwrap().risk, Risk::Write);
        assert_eq!(find("save_note").unwrap().risk, Risk::Write);
        assert_eq!(find("search_library").unwrap().risk, Risk::Read);
    }

    #[test]
    fn tool_names_are_unique_and_specs_are_functions() {
        for (i, t) in TOOLS.iter().enumerate() {
            assert!(TOOLS[i + 1..].iter().all(|o| o.name != t.name));
        }
        let all: Vec<&ToolDef> = TOOLS.iter().collect();
        for spec in specs(&all) {
            assert_eq!(spec["type"], "function");
            assert!(spec["function"]["parameters"]["type"] == "object");
        }
    }

    #[test]
    fn long_output_is_cut_and_says_so() {
        let long = "x".repeat(MAX_OUTPUT_CHARS + 50);
        assert!(clip(long).contains("Cut short"));
        assert_eq!(clip("short".into()), "short");
    }

    #[test]
    fn note_names_are_safe() {
        assert_eq!(slug("Plan: Q3 / launch!"), "plan-q3-launch");
        assert_eq!(slug("../../evil"), "evil");
        assert_eq!(slug("???"), "note");
    }
}
