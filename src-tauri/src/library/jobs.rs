//! Jobs: pipelines that keep their work.
//!
//! Decision (architecture): a job is a list of steps, and each finished step is saved before the
//! next begins, so stopping, a crash or a closed laptop loses nothing already done. Jobs run one
//! at a time on purpose, which keeps models from fighting over a small GPU. The first pipeline is
//! summarizing; later ones (translate, clean audio, dub) use the same runner.

use super::privacy::{visible_text, Grant, Provider};
use super::{get, preview_of, Library};
use crate::llm::commands::{ready_port, stream_completion, ChatEvent};
use crate::llm::engine::LlmEngine;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::json;
use specta::Type;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager, State};
use tauri_specta::Event;

/// Text handed to the model in one piece. Longer text is summarized in parts, then combined.
const CHUNK_CHARS: usize = 5_000;

const PART_PROMPT: &str = "You summarize text for the person who recorded or wrote it. \
Write 3 to 6 short bullet points of the key points, using only what the text says. \
Add a line starting with 'Action items:' only if the text names things to do.";

const WHOLE_PROMPT: &str = "You summarize text for the person who recorded or wrote it. \
Start with one sentence that says what it is about. Then write 3 to 6 short bullet points of the \
key points, using only what the text says. End with a line starting with 'Action items:' only if \
the text names things to do. Do not add anything that is not in the text.";

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct Job {
    pub id: i64,
    pub kind: String,
    pub title: String,
    pub input_id: String,
    /// queued, running, done, failed, cancelled or interrupted.
    pub state: String,
    pub step: String,
    pub percent: f64,
    pub output_id: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct JobUpdated(pub Job);

#[derive(Default)]
pub struct Jobs {
    cancels: Mutex<HashMap<i64, Arc<AtomicBool>>>,
    /// Held while a job runs, so jobs never overlap.
    serial: tokio::sync::Mutex<()>,
}

pub fn manage(app: &AppHandle) {
    app.manage(Jobs::default());
    // A job that was running when the app closed did not finish; it can be retried from its last step.
    if let Some(lib) = app.try_state::<Library>() {
        let conn = lib.conn.lock().unwrap();
        let _ = conn.execute(
            "UPDATE jobs SET state = 'interrupted', step = 'Stopped when Airlock closed' WHERE state IN ('queued','running')",
            [],
        );
    }
}

fn job_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<Job> {
    Ok(Job {
        id: r.get(0)?,
        kind: r.get(1)?,
        title: r.get(2)?,
        input_id: r.get(3)?,
        state: r.get(4)?,
        step: r.get(5)?,
        percent: r.get(6)?,
        output_id: r.get(7)?,
        error: r.get(8)?,
    })
}

const JOB_COLUMNS: &str = "id, kind, title, input_id, state, step, percent, output_id, error";

fn load(conn: &Connection, id: i64) -> Option<Job> {
    conn.query_row(&format!("SELECT {JOB_COLUMNS} FROM jobs WHERE id = ?1"), [id], job_row)
        .optional()
        .ok()
        .flatten()
}

/// Writes a change to the job and tells the screen.
fn update(app: &AppHandle, id: i64, state: &str, step: &str, percent: f64, output: Option<&str>, error: Option<&str>) {
    let Some(lib) = app.try_state::<Library>() else {
        return;
    };
    let job = {
        let conn = lib.conn.lock().unwrap();
        let _ = conn.execute(
            "UPDATE jobs SET state = ?1, step = ?2, percent = ?3, output_id = COALESCE(?4, output_id), error = ?5 WHERE id = ?6",
            params![state, step, percent, output, error, id],
        );
        load(&conn, id)
    };
    if let Some(job) = job {
        let _ = JobUpdated(job).emit(app);
    }
}

/// Splits text into parts of about `max` characters, at paragraph or sentence ends when it can.
pub fn split_chunks(text: &str, max: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current = String::new();
    for piece in text.split_inclusive(['\n', '.', '!', '?']) {
        if !current.is_empty() && current.chars().count() + piece.chars().count() > max {
            chunks.push(std::mem::take(&mut current).trim().to_string());
        }
        // A piece with no break at all still has to fit.
        if piece.chars().count() > max {
            let letters: Vec<char> = piece.chars().collect();
            for part in letters.chunks(max) {
                chunks.push(part.iter().collect::<String>().trim().to_string());
            }
            continue;
        }
        current.push_str(piece);
    }
    if !current.trim().is_empty() {
        chunks.push(current.trim().to_string());
    }
    chunks.retain(|c| !c.is_empty());
    chunks
}

async fn ask(app: &AppHandle, system: &str, user: &str, cancel: &AtomicBool) -> Result<String, String> {
    let engine = app.state::<Arc<LlmEngine>>();
    let (port, _, key) = ready_port(&engine)
        .await
        .map_err(|_| "Load a chat model first, then try again.".to_string())?;
    let _busy = engine.inner().begin();
    let body = json!({
        "messages": [
            { "role": "system", "content": system },
            { "role": "user", "content": user },
        ],
        "temperature": 0.2,
        "stream": true,
    });
    let mut out = String::new();
    stream_completion(port, &key, body, cancel, |e| {
        if let ChatEvent::Text { delta } = e {
            out.push_str(&delta);
        }
    })
    .await?;
    Ok(out.trim().to_string())
}

#[derive(Default, Serialize, Deserialize)]
struct Checkpoint {
    /// Summaries of the parts already done, in order.
    parts: Vec<String>,
}

async fn run_summarize(app: &AppHandle, id: i64, cancel: &AtomicBool) -> Result<String, String> {
    let (input_id, title, checkpoint) = {
        let lib = app.state::<Library>();
        let conn = lib.conn.lock().unwrap();
        let job = load(&conn, id).ok_or("The job is gone.")?;
        let cp: Option<String> = conn
            .query_row("SELECT checkpoint FROM jobs WHERE id = ?1", [id], |r| r.get(0))
            .optional()
            .ok()
            .flatten()
            .flatten();
        (job.input_id, job.title, cp.and_then(|c| serde_json::from_str::<Checkpoint>(&c).ok()).unwrap_or_default())
    };

    update(app, id, "running", "Reading", 2.0, None, None);
    let text = visible_text(app, &input_id, Provider::Local, Grant::Full)?;
    let chunks = split_chunks(&text, CHUNK_CHARS);
    if chunks.is_empty() {
        return Err("There is no text to summarize.".into());
    }

    let mut parts = checkpoint.parts;
    let total = chunks.len();
    for (i, chunk) in chunks.iter().enumerate().skip(parts.len()) {
        if cancel.load(Ordering::SeqCst) {
            return Err("cancelled".into());
        }
        let step = if total == 1 { "Summarizing".to_string() } else { format!("Summarizing part {} of {total}", i + 1) };
        update(app, id, "running", &step, 5.0 + 85.0 * i as f64 / total as f64, None, None);
        let prompt = if total == 1 { WHOLE_PROMPT } else { PART_PROMPT };
        let summary = ask(app, prompt, chunk, cancel).await?;
        parts.push(summary);
        let lib = app.state::<Library>();
        let conn = lib.conn.lock().unwrap();
        let _ = conn.execute(
            "UPDATE jobs SET checkpoint = ?1 WHERE id = ?2",
            params![serde_json::to_string(&Checkpoint { parts: parts.clone() }).unwrap_or_default(), id],
        );
    }

    let body = if total == 1 {
        parts.remove(0)
    } else {
        update(app, id, "running", "Putting it together", 92.0, None, None);
        ask(app, WHOLE_PROMPT, &parts.join("\n\n"), cancel).await?
    };
    if body.is_empty() {
        return Err("The model gave no summary. Try again.".into());
    }

    let lib = app.state::<Library>();
    let conn = lib.conn.lock().unwrap();
    // A summary is as private as what it was made from.
    let privacy = get(&conn, &input_id).map_or("local_only".to_string(), |a| {
        if a.privacy == super::Privacy::Shareable { "shareable".into() } else { "local_only".into() }
    });
    let output = format!("summary:{id}");
    conn.execute(
        "INSERT OR REPLACE INTO artifacts (id, kind, title, preview, created_ms, privacy, source, path, body)
         VALUES (?1, 'summary', ?2, ?3, ?4, ?5, ?6, NULL, ?7)",
        params![
            output,
            format!("Summary of {title}"),
            preview_of(&body),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0.0, |d| d.as_millis() as f64),
            privacy,
            format!("summarize of {input_id}"),
            body,
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(output)
}

async fn run(app: AppHandle, id: i64) {
    let jobs = app.state::<Jobs>();
    let cancel = Arc::new(AtomicBool::new(false));
    jobs.cancels.lock().unwrap().insert(id, cancel.clone());
    let _turn = jobs.serial.lock().await;
    let result = if cancel.load(Ordering::SeqCst) {
        Err("cancelled".to_string())
    } else {
        run_summarize(&app, id, &cancel).await
    };
    jobs.cancels.lock().unwrap().remove(&id);
    match result {
        Ok(output) => update(&app, id, "done", "Done", 100.0, Some(&output), None),
        Err(e) if e == "cancelled" || cancel.load(Ordering::SeqCst) => update(&app, id, "cancelled", "Stopped", 0.0, None, None),
        Err(e) => update(&app, id, "failed", "Stopped", 0.0, None, Some(&e)),
    }
}

fn create(app: &AppHandle, lib: &Library, kind: &str, input_id: &str) -> Result<Job, String> {
    if kind != "summarize" {
        return Err("Unknown job".into());
    }
    let id = {
        let conn = lib.conn.lock().unwrap();
        let input = get(&conn, input_id).ok_or("That item is not in the library.")?;
        if !matches!(input.kind.as_str(), "recording" | "transcript" | "note" | "summary") {
            return Err("This kind of item has no text to summarize.".into());
        }
        conn.execute(
            "INSERT INTO jobs (kind, input_id, title, state, step, created_ms) VALUES (?1, ?2, ?3, 'queued', 'Waiting', ?4)",
            params![
                kind,
                input_id,
                input.title,
                std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0.0, |d| d.as_millis() as f64)
            ],
        )
        .map_err(|e| e.to_string())?;
        conn.last_insert_rowid()
    };
    let job = load(&lib.conn.lock().unwrap(), id).ok_or("The job could not be saved.")?;
    let _ = JobUpdated(job.clone()).emit(app);
    Ok(job)
}

#[tauri::command]
#[specta::specta]
pub fn job_start(app: AppHandle, lib: State<'_, Library>, kind: String, input_id: String) -> Result<Job, String> {
    let job = create(&app, &lib, &kind, &input_id)?;
    tauri::async_runtime::spawn(run(app.clone(), job.id));
    Ok(job)
}

/// Summarizes an item and waits for it, for callers (the assistant) that need the text back.
/// The job still shows in the library like any other. Returns the summary's id and text.
pub async fn summarize_and_wait(app: &AppHandle, input_id: &str) -> Result<(String, String), String> {
    let lib = app.try_state::<Library>().ok_or("The library is not available.")?;
    let job = create(app, &lib, "summarize", input_id)?;
    run(app.clone(), job.id).await;
    let conn = lib.conn.lock().unwrap();
    let done = load(&conn, job.id).ok_or("The job is gone.")?;
    match (done.state.as_str(), done.output_id) {
        ("done", Some(output)) => {
            let text = super::full_text(app, &conn, &output)?;
            Ok((output, text))
        }
        _ => Err(done.error.unwrap_or_else(|| "The summary was stopped.".into())),
    }
}

/// Runs a stopped job again; parts already summarized are kept, not redone.
#[tauri::command]
#[specta::specta]
pub fn job_retry(app: AppHandle, lib: State<'_, Library>, id: i64) -> Result<(), String> {
    {
        let conn = lib.conn.lock().unwrap();
        let job = load(&conn, id).ok_or("The job is gone.")?;
        if matches!(job.state.as_str(), "queued" | "running" | "done") {
            return Err("That job is already running or finished.".into());
        }
    }
    update(&app, id, "queued", "Waiting", 0.0, None, None);
    tauri::async_runtime::spawn(run(app.clone(), id));
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn job_cancel(jobs: State<'_, Jobs>, id: i64) {
    if let Some(flag) = jobs.cancels.lock().unwrap().get(&id) {
        flag.store(true, Ordering::SeqCst);
    }
}

#[tauri::command]
#[specta::specta]
pub fn job_list(lib: State<'_, Library>) -> Result<Vec<Job>, String> {
    let conn = lib.conn.lock().unwrap();
    let mut stmt = conn
        .prepare(&format!("SELECT {JOB_COLUMNS} FROM jobs ORDER BY id DESC LIMIT 50"))
        .map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], job_row).map_err(|e| e.to_string())?;
    Ok(rows.flatten().collect())
}

#[tauri::command]
#[specta::specta]
pub fn job_clear(lib: State<'_, Library>, id: i64) -> Result<(), String> {
    let conn = lib.conn.lock().unwrap();
    conn.execute("DELETE FROM jobs WHERE id = ?1 AND state NOT IN ('queued','running')", [id])
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_text_stays_whole() {
        assert_eq!(split_chunks("One. Two.", 100), vec!["One. Two."]);
    }

    #[test]
    fn long_text_splits_at_sentence_ends_and_loses_nothing() {
        let text = "Alpha one. Beta two. Gamma three. Delta four.";
        let chunks = split_chunks(text, 22);
        assert!(chunks.len() >= 2);
        assert!(chunks.iter().all(|c| c.chars().count() <= 22));
        let joined = chunks.join(" ");
        for word in ["Alpha", "Beta", "Gamma", "Delta"] {
            assert!(joined.contains(word), "{word}");
        }
    }

    #[test]
    fn text_with_no_breaks_is_still_cut_to_fit() {
        let chunks = split_chunks(&"a".repeat(25), 10);
        assert_eq!(chunks.iter().map(|c| c.len()).collect::<Vec<_>>(), vec![10, 10, 5]);
    }

    #[test]
    fn empty_text_has_no_parts() {
        assert!(split_chunks("  \n ", 100).is_empty());
    }

    #[test]
    fn the_schema_has_a_jobs_table() {
        let conn = Connection::open_in_memory().unwrap();
        super::super::init(&conn).unwrap();
        conn.execute(
            "INSERT INTO jobs (kind, input_id, title, state, created_ms) VALUES ('summarize','note:a','t','queued',1)",
            [],
        )
        .unwrap();
        assert_eq!(load(&conn, 1).unwrap().state, "queued");
    }
}
