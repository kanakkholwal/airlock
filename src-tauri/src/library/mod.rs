//! The library: one local index of everything Airlock has made or kept, each with a privacy flag.
//!
//! Decision (architecture): the library points at the stores that already exist (dictation
//! history, generated images, saved notes) and adds what they lack: a kind, a short preview, and
//! `privacy`. Derived things, such as summaries, are stored here with their text. Privacy starts
//! as `local_only` for everything, so nothing can ever go to a cloud provider by default.

pub mod jobs;
pub mod privacy;

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Manager, State};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum Privacy {
    LocalOnly,
    Shareable,
}

impl Privacy {
    fn as_str(self) -> &'static str {
        match self {
            Privacy::LocalOnly => "local_only",
            Privacy::Shareable => "shareable",
        }
    }

    fn parse(s: &str) -> Self {
        if s == "shareable" {
            Privacy::Shareable
        } else {
            Privacy::LocalOnly
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct Artifact {
    /// `recording:12`, `transcript:30`, `image:<id>`, `note:<file>`, `summary:<id>`.
    pub id: String,
    pub kind: String,
    pub title: String,
    /// The first words, or the prompt for an image. Never the whole text.
    pub preview: String,
    pub created_ms: f64,
    pub privacy: Privacy,
    /// What made it, such as `dictation` or `summarize of recording:12`.
    pub source: String,
    pub path: Option<String>,
}

pub struct Library {
    conn: Mutex<Connection>,
}

pub fn manage(app: &AppHandle) {
    match open(app) {
        Ok(conn) => {
            app.manage(Library { conn: Mutex::new(conn) });
            jobs::manage(app);
        }
        Err(e) => log::error!("The library could not be opened: {e}"),
    }
}

fn open(app: &AppHandle) -> Result<Connection, String> {
    let dir = crate::portable::app_data_dir(app).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let conn = Connection::open(dir.join("library.db")).map_err(|e| e.to_string())?;
    init(&conn).map_err(|e| e.to_string())?;
    Ok(conn)
}

pub(crate) fn init(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS artifacts (
            id TEXT PRIMARY KEY,
            kind TEXT NOT NULL,
            title TEXT NOT NULL,
            preview TEXT NOT NULL,
            created_ms REAL NOT NULL,
            privacy TEXT NOT NULL DEFAULT 'local_only',
            source TEXT NOT NULL DEFAULT '',
            path TEXT,
            body TEXT
        );
        CREATE TABLE IF NOT EXISTS jobs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            kind TEXT NOT NULL,
            input_id TEXT NOT NULL,
            title TEXT NOT NULL,
            state TEXT NOT NULL,
            step TEXT NOT NULL DEFAULT '',
            percent REAL NOT NULL DEFAULT 0,
            output_id TEXT,
            error TEXT,
            checkpoint TEXT,
            created_ms REAL NOT NULL
        );",
    )
}

/// First ~160 characters on one line.
pub fn preview_of(text: &str) -> String {
    let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() > 160 {
        format!("{}…", flat.chars().take(160).collect::<String>())
    } else {
        flat
    }
}

/// A note's title is its first `# ` line; the preview is the text after it.
pub fn note_parts(markdown: &str, fallback: &str) -> (String, String) {
    let mut lines = markdown.lines();
    let first = lines.next().unwrap_or("").trim();
    match first.strip_prefix("# ") {
        Some(title) => (title.trim().to_string(), preview_of(&lines.collect::<Vec<_>>().join(" "))),
        None => (fallback.to_string(), preview_of(markdown)),
    }
}

struct Seen(HashSet<String>);

fn upsert(
    conn: &Connection,
    seen: &mut Seen,
    a: &Artifact,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO artifacts (id, kind, title, preview, created_ms, privacy, source, path)
         VALUES (?1, ?2, ?3, ?4, ?5, 'local_only', ?6, ?7)
         ON CONFLICT(id) DO UPDATE SET title = ?3, preview = ?4, created_ms = ?5, source = ?6, path = ?7",
        params![a.id, a.kind, a.title, a.preview, a.created_ms, a.source, a.path],
    )?;
    seen.0.insert(a.id.clone());
    Ok(())
}

fn index_history(app: &AppHandle, conn: &Connection, seen: &mut Seen) {
    let Ok(dir) = crate::portable::app_data_dir(app) else {
        return;
    };
    let Ok(db) = Connection::open_with_flags(dir.join("history.db"), rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY) else {
        return;
    };
    let Ok(mut stmt) = db.prepare(
        "SELECT id, timestamp, title, transcription_text, source_name FROM transcription_history
         WHERE transcription_text != '' ORDER BY id DESC LIMIT 2000",
    ) else {
        return;
    };
    let rows = stmt.query_map([], |r| {
        Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?, r.get::<_, String>(2)?, r.get::<_, String>(3)?, r.get::<_, Option<String>>(4)?))
    });
    for (id, ts, title, text, source_name) in rows.into_iter().flatten().flatten() {
        let file = source_name.is_some();
        let name = source_name.clone().filter(|n| !n.is_empty()).or(Some(title).filter(|t| !t.trim().is_empty()));
        let a = Artifact {
            id: format!("{}:{id}", if file { "transcript" } else { "recording" }),
            kind: if file { "transcript" } else { "recording" }.into(),
            title: name.unwrap_or_else(|| preview_of(&text).chars().take(48).collect()),
            preview: preview_of(&text),
            created_ms: ts as f64 * 1000.0,
            privacy: Privacy::LocalOnly,
            source: if file { "file transcription".into() } else { "dictation".into() },
            path: None,
        };
        let _ = upsert(conn, seen, &a);
    }
}

fn index_images(app: &AppHandle, conn: &Connection, seen: &mut Seen) {
    let Ok(list) = crate::image::generate::image_list(app.clone()) else {
        return;
    };
    for img in list {
        let a = Artifact {
            id: format!("image:{}", img.id),
            kind: "image".into(),
            title: preview_of(&img.prompt).chars().take(48).collect(),
            preview: img.prompt.clone(),
            created_ms: img.created_ms,
            privacy: Privacy::LocalOnly,
            source: format!("{} · {}×{}", img.model, img.width, img.height),
            path: Some(img.path),
        };
        let _ = upsert(conn, seen, &a);
    }
}

fn index_notes(app: &AppHandle, conn: &Connection, seen: &mut Seen) {
    let Ok(dir) = crate::portable::app_data_dir(app).map(|d| d.join("notes")) else {
        return;
    };
    let Ok(read) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in read.flatten() {
        let path = entry.path();
        if path.extension().is_none_or(|e| e != "md") {
            continue;
        }
        let name = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let (title, preview) = note_parts(&text, &name);
        let created = entry
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map_or(0.0, |d| d.as_millis() as f64);
        let a = Artifact {
            id: format!("note:{name}"),
            kind: "note".into(),
            title,
            preview,
            created_ms: created,
            privacy: Privacy::LocalOnly,
            source: "note".into(),
            path: Some(path.to_string_lossy().into_owned()),
        };
        let _ = upsert(conn, seen, &a);
    }
}

/// Reads the stores the library points at, adds what is new and forgets what is gone. Privacy
/// flags the person set are kept. Summaries live only here, so they are never forgotten.
pub fn refresh(app: &AppHandle) -> Result<(), String> {
    let lib = app.try_state::<Library>().ok_or("The library is not available.")?;
    let conn = lib.conn.lock().unwrap();
    let mut seen = Seen(HashSet::new());
    index_history(app, &conn, &mut seen);
    index_images(app, &conn, &mut seen);
    index_notes(app, &conn, &mut seen);
    let mut stmt = conn
        .prepare("SELECT id FROM artifacts WHERE kind IN ('recording','transcript','image','note')")
        .map_err(|e| e.to_string())?;
    let stale: Vec<String> = stmt
        .query_map([], |r| r.get::<_, String>(0))
        .map_err(|e| e.to_string())?
        .flatten()
        .filter(|id| !seen.0.contains(id))
        .collect();
    drop(stmt);
    for id in stale {
        let _ = conn.execute("DELETE FROM artifacts WHERE id = ?1", [id]);
    }
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn library_refresh(app: AppHandle) -> Result<(), String> {
    refresh(&app)
}

fn row(r: &rusqlite::Row<'_>) -> rusqlite::Result<Artifact> {
    Ok(Artifact {
        id: r.get(0)?,
        kind: r.get(1)?,
        title: r.get(2)?,
        preview: r.get(3)?,
        created_ms: r.get(4)?,
        privacy: Privacy::parse(&r.get::<_, String>(5)?),
        source: r.get(6)?,
        path: r.get(7)?,
    })
}

const COLUMNS: &str = "id, kind, title, preview, created_ms, privacy, source, path";

pub(crate) fn get(conn: &Connection, id: &str) -> Option<Artifact> {
    conn.query_row(&format!("SELECT {COLUMNS} FROM artifacts WHERE id = ?1"), [id], row)
        .optional()
        .ok()
        .flatten()
}

fn list(conn: &Connection, kind: &str, query: &str, limit: usize) -> Result<Vec<Artifact>, String> {
    let like = format!("%{}%", query.replace(['%', '_'], ""));
    let mut stmt = conn
        .prepare(&format!(
            "SELECT {COLUMNS} FROM artifacts
             WHERE (?1 = '' OR kind = ?1) AND (title LIKE ?2 OR preview LIKE ?2)
             ORDER BY created_ms DESC LIMIT ?3"
        ))
        .map_err(|e| e.to_string())?;
    let rows = stmt.query_map(params![kind, like, limit as i64], row).map_err(|e| e.to_string())?;
    Ok(rows.flatten().collect())
}

/// Items matching some words, newest first. Used by the assistant's search tool.
pub fn search(app: &AppHandle, kind: &str, query: &str, limit: usize) -> Result<Vec<Artifact>, String> {
    refresh(app)?;
    let lib = app.try_state::<Library>().ok_or("The library is not available.")?;
    let conn = lib.conn.lock().unwrap();
    list(&conn, kind, query, limit)
}

#[tauri::command]
#[specta::specta]
pub fn library_list(
    lib: State<'_, Library>,
    kind: Option<String>,
    query: Option<String>,
) -> Result<Vec<Artifact>, String> {
    let conn = lib.conn.lock().unwrap();
    list(&conn, &kind.unwrap_or_default(), &query.unwrap_or_default(), 500)
}

#[tauri::command]
#[specta::specta]
pub fn library_set_privacy(lib: State<'_, Library>, id: String, privacy: Privacy) -> Result<(), String> {
    let conn = lib.conn.lock().unwrap();
    conn.execute("UPDATE artifacts SET privacy = ?1 WHERE id = ?2", params![privacy.as_str(), id])
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// The full text of an item, for the person's own screen. Models never get it from here: they
/// go through `privacy::visible_text`, which applies the grants.
pub(crate) fn full_text(app: &AppHandle, conn: &Connection, id: &str) -> Result<String, String> {
    let (kind, rest) = id.split_once(':').ok_or("Unknown item")?;
    match kind {
        "recording" | "transcript" => {
            let num: i64 = rest.parse().map_err(|_| "Unknown item".to_string())?;
            let dir = crate::portable::app_data_dir(app).map_err(|e| e.to_string())?;
            let db = Connection::open_with_flags(dir.join("history.db"), rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
                .map_err(|e| e.to_string())?;
            db.query_row(
                "SELECT COALESCE(NULLIF(post_processed_text, ''), transcription_text) FROM transcription_history WHERE id = ?1",
                [num],
                |r| r.get::<_, String>(0),
            )
            .map_err(|_| "That recording is gone.".to_string())
        }
        "note" => {
            let a = get(conn, id).ok_or("Unknown item")?;
            std::fs::read_to_string(a.path.ok_or("Unknown item")?).map_err(|e| e.to_string())
        }
        "summary" => conn
            .query_row("SELECT body FROM artifacts WHERE id = ?1", [id], |r| r.get::<_, Option<String>>(0))
            .optional()
            .ok()
            .flatten()
            .flatten()
            .ok_or_else(|| "Unknown item".to_string()),
        _ => Err("This kind of item has no text.".into()),
    }
}

#[tauri::command]
#[specta::specta]
pub fn library_text(app: AppHandle, lib: State<'_, Library>, id: String) -> Result<String, String> {
    let conn = lib.conn.lock().unwrap();
    full_text(&app, &conn, &id)
}

#[allow(dead_code)]
pub(crate) fn db_path(app: &AppHandle) -> Option<PathBuf> {
    crate::portable::app_data_dir(app).ok().map(|d| d.join("library.db"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mem() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        init(&conn).unwrap();
        conn
    }

    fn art(id: &str, title: &str) -> Artifact {
        Artifact {
            id: id.into(),
            kind: "note".into(),
            title: title.into(),
            preview: "p".into(),
            created_ms: 1.0,
            privacy: Privacy::LocalOnly,
            source: "s".into(),
            path: None,
        }
    }

    #[test]
    fn everything_starts_private_and_refreshing_keeps_the_choice() {
        let conn = mem();
        let mut seen = Seen(HashSet::new());
        upsert(&conn, &mut seen, &art("note:a", "First")).unwrap();
        assert_eq!(get(&conn, "note:a").unwrap().privacy, Privacy::LocalOnly);
        conn.execute("UPDATE artifacts SET privacy = 'shareable' WHERE id = 'note:a'", []).unwrap();
        upsert(&conn, &mut seen, &art("note:a", "Renamed")).unwrap();
        let a = get(&conn, "note:a").unwrap();
        assert_eq!((a.title.as_str(), a.privacy), ("Renamed", Privacy::Shareable));
    }

    #[test]
    fn previews_are_short_and_flat() {
        assert_eq!(preview_of("a\n\n  b   c"), "a b c");
        assert!(preview_of(&"word ".repeat(100)).ends_with('…'));
    }

    #[test]
    fn notes_take_their_title_from_the_heading() {
        assert_eq!(note_parts("# Plan\n\nDo the thing.", "x"), ("Plan".into(), "Do the thing.".into()));
        assert_eq!(note_parts("no heading here", "file").0, "file");
    }
}
