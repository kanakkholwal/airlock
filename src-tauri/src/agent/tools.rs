use super::config::{self, AgentConfig};
use serde::Serialize;
use serde_json::{json, Value};
use specta::Type;
use std::path::{Path, PathBuf};
use tauri::AppHandle;

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

pub struct ToolDef {
    pub name: &'static str,
    pub description: &'static str,
    pub risk: Risk,
    /// Only offered once at least one folder is shared.
    pub needs_folder: bool,
    pub parameters: fn() -> Value,
    /// What the call does in words, shown in chat and in the approval card.
    pub describe: fn(&Value) -> String,
    pub run: fn(&AppHandle, &AgentConfig, &Value) -> Result<String, String>,
}

pub const TOOLS: &[ToolDef] = &[
    ToolDef {
        name: "get_time",
        description: "Get the current local date, time and weekday.",
        risk: Risk::Read,
        needs_folder: false,
        parameters: || json!({ "type": "object", "properties": {} }),
        describe: |_| "Checking the time".into(),
        run: |_, _, _| Ok(chrono::Local::now().format("%A, %d %B %Y, %H:%M (%:z)").to_string()),
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
        run: list_folder,
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
        run: read_text_file,
    },
    ToolDef {
        name: "search_recordings",
        description: "Search the user's dictated recordings and transcribed files by words. Returns only short previews with an id; ask to read one in full with read_recording.",
        risk: Risk::Read,
        needs_folder: false,
        parameters: || {
            json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Words to look for." },
                    "limit": { "type": "integer", "description": "How many results, at most 10." }
                },
                "required": ["query"]
            })
        },
        describe: |a| format!("Searching recordings for \"{}\"", a["query"].as_str().unwrap_or("")),
        run: search_recordings,
    },
    ToolDef {
        name: "read_recording",
        description: "Read the full text of one recording by its id from search_recordings. The user is asked before it is shared.",
        risk: Risk::Ask,
        needs_folder: false,
        parameters: || {
            json!({
                "type": "object",
                "properties": { "id": { "type": "integer", "description": "The recording id." } },
                "required": ["id"]
            })
        },
        describe: |a| format!("Reading the full text of recording #{}", a["id"]),
        run: read_recording,
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
        run: save_note,
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

fn history_db(app: &AppHandle) -> Result<rusqlite::Connection, String> {
    let path = crate::portable::app_data_dir(app)
        .map_err(|e| e.to_string())?
        .join("history.db");
    rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|_| "There are no recordings yet.".to_string())
}

fn preview(text: &str) -> String {
    let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() > 160 {
        format!("{}…", flat.chars().take(160).collect::<String>())
    } else {
        flat
    }
}

fn when(ts: i64) -> String {
    chrono::DateTime::from_timestamp(ts, 0)
        .map(|d| d.with_timezone(&chrono::Local).format("%d %b %Y %H:%M").to_string())
        .unwrap_or_default()
}

fn search_recordings(app: &AppHandle, _: &AgentConfig, args: &Value) -> Result<String, String> {
    let query = args["query"].as_str().ok_or("Give some words to search for")?.trim();
    if query.is_empty() {
        return Err("Give some words to search for".into());
    }
    let limit = args["limit"].as_i64().unwrap_or(5).clamp(1, 10);
    let conn = history_db(app)?;
    let like = format!("%{}%", query.replace('%', "").replace('_', ""));
    let mut stmt = conn
        .prepare(
            "SELECT id, timestamp, title, transcription_text FROM transcription_history
             WHERE transcription_text LIKE ?1 OR title LIKE ?1 ORDER BY id DESC LIMIT ?2",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(rusqlite::params![like, limit], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?, r.get::<_, String>(2)?, r.get::<_, String>(3)?))
        })
        .map_err(|e| e.to_string())?;
    let mut lines = vec![];
    for row in rows.flatten() {
        let (id, ts, title, text) = row;
        let head = if title.trim().is_empty() { preview(&text) } else { title };
        lines.push(format!("#{id} · {} · {}", when(ts), preview(&head)));
    }
    if lines.is_empty() {
        return Ok("No recordings matched.".into());
    }
    Ok(lines.join("\n"))
}

fn read_recording(app: &AppHandle, _: &AgentConfig, args: &Value) -> Result<String, String> {
    let id = args["id"].as_i64().ok_or("Give a recording id")?;
    let conn = history_db(app)?;
    let text: Option<(String, Option<String>)> = conn
        .query_row(
            "SELECT transcription_text, post_processed_text FROM transcription_history WHERE id = ?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .ok();
    match text {
        Some((raw, cleaned)) => Ok(clip(cleaned.filter(|c| !c.trim().is_empty()).unwrap_or(raw))),
        None => Err("No recording has that id.".into()),
    }
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
        assert_eq!(find("read_recording").unwrap().risk, Risk::Ask);
        assert_eq!(find("save_note").unwrap().risk, Risk::Write);
        assert_eq!(find("search_recordings").unwrap().risk, Risk::Read);
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
