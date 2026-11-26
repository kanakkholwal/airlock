//! Automations: things Airlock does on a schedule, on the user's own device.
//!
//! Decision (architecture): an automation is a cron expression plus one action. Runs execute
//! through the library's serial job runner, so they never fight a chat or a dictation for the GPU.
//! Decision (product): runs only happen while Airlock is open. A run missed while it was closed
//! is skipped or run once on launch, per the automation's `catchup`.

pub mod schedule;

use crate::library::{self, Library};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::HashSet;
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Manager, State};
use tauri_specta::Event;

const TICK: Duration = Duration::from_secs(30);
/// A run later than this after its time counts as missed (laptop asleep, app closed).
const GRACE_MS: i64 = 120_000;
const FIRST_LOOKBACK_MS: i64 = 24 * 3_600_000;
const ACTIONS: &[&str] = &["summarize_new"];

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct Automation {
    pub id: i64,
    pub name: String,
    pub cron: String,
    pub action: String,
    pub enabled: bool,
    /// `skip` or `run_once`.
    pub catchup: String,
    pub next_run_ms: Option<f64>,
    pub last_run_ms: Option<f64>,
}

#[derive(Debug, Clone, Deserialize, Type)]
pub struct AutomationInput {
    pub id: Option<i64>,
    pub name: String,
    pub cron: String,
    pub action: String,
    pub enabled: bool,
    pub catchup: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct Run {
    pub id: i64,
    pub automation_id: i64,
    pub started_ms: f64,
    pub finished_ms: Option<f64>,
    /// running, done, failed or skipped.
    pub status: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct TimezoneInfo {
    /// `system` or an IANA name.
    pub setting: String,
    /// The zone schedules use right now.
    pub effective: String,
    pub system: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct AutomationUpdated(pub Automation);

#[derive(Default)]
pub struct Scheduler {
    running: Mutex<HashSet<i64>>,
}

pub(crate) fn init(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS automations (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            cron TEXT NOT NULL,
            action TEXT NOT NULL,
            enabled INTEGER NOT NULL DEFAULT 1,
            catchup TEXT NOT NULL DEFAULT 'run_once',
            next_run_ms REAL,
            last_run_ms REAL
        );
        CREATE TABLE IF NOT EXISTS automation_runs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            automation_id INTEGER NOT NULL,
            started_ms REAL NOT NULL,
            finished_ms REAL,
            status TEXT NOT NULL,
            message TEXT NOT NULL DEFAULT ''
        );
        CREATE TABLE IF NOT EXISTS automation_settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );",
    )
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as i64)
}

const COLUMNS: &str = "id, name, cron, action, enabled, catchup, next_run_ms, last_run_ms";

fn row(r: &rusqlite::Row<'_>) -> rusqlite::Result<Automation> {
    Ok(Automation {
        id: r.get(0)?,
        name: r.get(1)?,
        cron: r.get(2)?,
        action: r.get(3)?,
        enabled: r.get::<_, i64>(4)? != 0,
        catchup: r.get(5)?,
        next_run_ms: r.get(6)?,
        last_run_ms: r.get(7)?,
    })
}

fn load(conn: &Connection, id: i64) -> Option<Automation> {
    conn.query_row(
        &format!("SELECT {COLUMNS} FROM automations WHERE id = ?1"),
        [id],
        row,
    )
    .optional()
    .ok()
    .flatten()
}

fn zone_setting(conn: &Connection) -> String {
    conn.query_row(
        "SELECT value FROM automation_settings WHERE key = 'timezone'",
        [],
        |r| r.get(0),
    )
    .unwrap_or_else(|_| schedule::SYSTEM.to_string())
}

fn zone(conn: &Connection) -> chrono_tz::Tz {
    schedule::resolve_zone(&zone_setting(conn)).unwrap_or_else(|_| schedule::system_zone())
}

/// Sets `next_run_ms` for one automation from now, in the current zone. Disabled ones get none.
fn reschedule(conn: &Connection, a: &Automation, from_ms: i64) -> Result<(), String> {
    let next = if a.enabled {
        schedule::next_run_ms(&a.cron, zone(conn), from_ms)?
    } else {
        None
    };
    conn.execute(
        "UPDATE automations SET next_run_ms = ?1 WHERE id = ?2",
        params![next.map(|n| n as f64), a.id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn emit(app: &AppHandle, id: i64) {
    if let Some(lib) = app.try_state::<Library>() {
        if let Some(a) = load(&lib.conn.lock().unwrap(), id) {
            let _ = AutomationUpdated(a).emit(app);
        }
    }
}

// --- Actions ---

/// Notes, transcripts and recordings made since `since_ms` that have no summary yet.
fn unsummarized_since(conn: &Connection, since_ms: i64) -> Vec<String> {
    let Ok(mut stmt) = conn.prepare(
        "SELECT a.id FROM artifacts a
         WHERE a.kind IN ('recording','transcript','note') AND a.created_ms > ?1
           AND NOT EXISTS (SELECT 1 FROM artifacts s WHERE s.kind = 'summary' AND s.source = 'summarize of ' || a.id)
         ORDER BY a.created_ms",
    ) else {
        return Vec::new();
    };
    stmt.query_map([since_ms as f64], |r| r.get::<_, String>(0))
        .map(|rows| rows.flatten().collect())
        .unwrap_or_default()
}

/// Returns a one-line result for the run history.
async fn summarize_new(app: &AppHandle, since_ms: i64) -> Result<String, String> {
    let lib = app.state::<Library>();
    let ids = unsummarized_since(&lib.conn.lock().unwrap(), since_ms);
    if ids.is_empty() {
        return Ok("Nothing new to summarize.".into());
    }
    let (mut done, mut last_error) = (0, None);
    for id in &ids {
        match library::jobs::summarize_and_wait(app, id).await {
            Ok(_) => done += 1,
            Err(e) => {
                let stop = e.contains("Load a chat model");
                last_error = Some(e);
                if stop {
                    break;
                }
            }
        }
    }
    match (done, last_error) {
        (0, Some(e)) => Err(e),
        (n, Some(e)) => Ok(format!("Summarized {n} of {}. Last problem: {e}", ids.len())),
        (n, None) => Ok(format!("Summarized {n}.")),
    }
}

// --- Running ---

async fn execute(app: AppHandle, id: i64, manual: bool) {
    let sched = app.state::<Scheduler>();
    if !sched.running.lock().unwrap().insert(id) {
        return;
    }
    let started = now_ms();
    let (action, since, run_id) = {
        let lib = app.state::<Library>();
        let conn = lib.conn.lock().unwrap();
        let Some(a) = load(&conn, id) else {
            sched.running.lock().unwrap().remove(&id);
            return;
        };
        let _ = conn.execute(
            "INSERT INTO automation_runs (automation_id, started_ms, status, message) VALUES (?1, ?2, 'running', ?3)",
            params![id, started as f64, if manual { "Started by you" } else { "" }],
        );
        (
            a.action,
            a.last_run_ms.map_or(started - FIRST_LOOKBACK_MS, |m| m as i64),
            conn.last_insert_rowid(),
        )
    };

    let result = match action.as_str() {
        "summarize_new" => summarize_new(&app, since).await,
        other => Err(format!("Unknown action '{other}'.")),
    };

    let (status, message) = match result {
        Ok(m) => ("done", m),
        Err(e) => ("failed", e),
    };
    {
        let lib = app.state::<Library>();
        let conn = lib.conn.lock().unwrap();
        let _ = conn.execute(
            "UPDATE automation_runs SET finished_ms = ?1, status = ?2, message = ?3 WHERE id = ?4",
            params![now_ms() as f64, status, message, run_id],
        );
        // A failed run keeps its old watermark so the next run retries the same items.
        if status == "done" {
            let _ = conn.execute(
                "UPDATE automations SET last_run_ms = ?1 WHERE id = ?2",
                params![started as f64, id],
            );
        }
        let _ = conn.execute(
            "DELETE FROM automation_runs WHERE automation_id = ?1 AND id NOT IN
             (SELECT id FROM automation_runs WHERE automation_id = ?1 ORDER BY id DESC LIMIT 50)",
            [id],
        );
    }
    sched.running.lock().unwrap().remove(&id);
    emit(&app, id);
}

/// Starts every automation whose time has come. A missed one runs once or is skipped.
async fn tick(app: &AppHandle) {
    let now = now_ms();
    let due: Vec<Automation> = {
        let Some(lib) = app.try_state::<Library>() else {
            return;
        };
        let conn = lib.conn.lock().unwrap();
        let Ok(mut stmt) = conn.prepare(&format!(
            "SELECT {COLUMNS} FROM automations WHERE enabled = 1 AND next_run_ms IS NOT NULL AND next_run_ms <= ?1"
        )) else {
            return;
        };
        stmt.query_map([now as f64], row)
            .map(|r| r.flatten().collect())
            .unwrap_or_default()
    };
    for a in due {
        let late = now - a.next_run_ms.map_or(now, |n| n as i64);
        {
            let lib = app.state::<Library>();
            let conn = lib.conn.lock().unwrap();
            let _ = reschedule(&conn, &a, now);
            if late > GRACE_MS && a.catchup == "skip" {
                let _ = conn.execute(
                    "INSERT INTO automation_runs (automation_id, started_ms, finished_ms, status, message)
                     VALUES (?1, ?2, ?2, 'skipped', 'Missed while Airlock was closed')",
                    params![a.id, now as f64],
                );
                continue;
            }
        }
        tauri::async_runtime::spawn(execute(app.clone(), a.id, false));
    }
}

pub fn manage(app: &AppHandle) {
    app.manage(Scheduler::default());
    // Saved next runs are kept, even overdue ones, so tick() can apply the catch-up rule.
    if let Some(lib) = app.try_state::<Library>() {
        let conn = lib.conn.lock().unwrap();
        let all: Vec<Automation> = conn
            .prepare(&format!("SELECT {COLUMNS} FROM automations"))
            .and_then(|mut s| s.query_map([], row).map(|r| r.flatten().collect()))
            .unwrap_or_default();
        for a in all {
            if a.next_run_ms.is_none() {
                let _ = reschedule(&conn, &a, now_ms());
            }
        }
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let mut timer = tokio::time::interval(TICK);
        loop {
            timer.tick().await;
            tick(&app).await;
        }
    });
}

// --- Commands ---

#[tauri::command]
#[specta::specta]
pub fn automation_list(lib: State<'_, Library>) -> Result<Vec<Automation>, String> {
    let conn = lib.conn.lock().unwrap();
    let mut stmt = conn
        .prepare(&format!("SELECT {COLUMNS} FROM automations ORDER BY id"))
        .map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], row).map_err(|e| e.to_string())?;
    Ok(rows.flatten().collect())
}

#[tauri::command]
#[specta::specta]
pub fn automation_save(
    app: AppHandle,
    lib: State<'_, Library>,
    input: AutomationInput,
) -> Result<Automation, String> {
    if input.name.trim().is_empty() {
        return Err("Give the automation a name.".into());
    }
    schedule::validate(&input.cron)?;
    if !ACTIONS.contains(&input.action.as_str()) {
        return Err("Unknown action.".into());
    }
    if !matches!(input.catchup.as_str(), "skip" | "run_once") {
        return Err("Catch-up must be skip or run_once.".into());
    }
    let id = {
        let conn = lib.conn.lock().unwrap();
        let id = match input.id {
            Some(id) => {
                conn.execute(
                    "UPDATE automations SET name=?1, cron=?2, action=?3, enabled=?4, catchup=?5 WHERE id=?6",
                    params![input.name.trim(), input.cron, input.action, input.enabled, input.catchup, id],
                )
                .map_err(|e| e.to_string())?;
                id
            }
            None => {
                conn.execute(
                    "INSERT INTO automations (name, cron, action, enabled, catchup) VALUES (?1,?2,?3,?4,?5)",
                    params![input.name.trim(), input.cron, input.action, input.enabled, input.catchup],
                )
                .map_err(|e| e.to_string())?;
                conn.last_insert_rowid()
            }
        };
        let a = load(&conn, id).ok_or("The automation could not be saved.")?;
        reschedule(&conn, &a, now_ms())?;
        id
    };
    emit(&app, id);
    load(&lib.conn.lock().unwrap(), id).ok_or_else(|| "The automation is gone.".into())
}

#[tauri::command]
#[specta::specta]
pub fn automation_delete(lib: State<'_, Library>, id: i64) -> Result<(), String> {
    let conn = lib.conn.lock().unwrap();
    conn.execute("DELETE FROM automation_runs WHERE automation_id = ?1", [id])
        .map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM automations WHERE id = ?1", [id])
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub fn automation_run_now(app: AppHandle, id: i64) {
    tauri::async_runtime::spawn(execute(app, id, true));
}

#[tauri::command]
#[specta::specta]
pub fn automation_runs(lib: State<'_, Library>, id: i64) -> Result<Vec<Run>, String> {
    let conn = lib.conn.lock().unwrap();
    let mut stmt = conn
        .prepare(
            "SELECT id, automation_id, started_ms, finished_ms, status, message
             FROM automation_runs WHERE automation_id = ?1 ORDER BY id DESC LIMIT 50",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([id], |r| {
            Ok(Run {
                id: r.get(0)?,
                automation_id: r.get(1)?,
                started_ms: r.get(2)?,
                finished_ms: r.get(3)?,
                status: r.get(4)?,
                message: r.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?;
    Ok(rows.flatten().collect())
}

#[tauri::command]
#[specta::specta]
pub fn automation_timezone(lib: State<'_, Library>) -> TimezoneInfo {
    let conn = lib.conn.lock().unwrap();
    let setting = zone_setting(&conn);
    TimezoneInfo {
        effective: zone(&conn).name().to_string(),
        system: schedule::system_zone().name().to_string(),
        setting,
    }
}

/// `system` or an IANA name. Every schedule is recomputed in the new zone.
#[tauri::command]
#[specta::specta]
pub fn automation_set_timezone(
    app: AppHandle,
    lib: State<'_, Library>,
    setting: String,
) -> Result<TimezoneInfo, String> {
    schedule::resolve_zone(&setting)?;
    let ids: Vec<i64> = {
        let conn = lib.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO automation_settings (key, value) VALUES ('timezone', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [&setting],
        )
        .map_err(|e| e.to_string())?;
        let all: Vec<Automation> = conn
            .prepare(&format!("SELECT {COLUMNS} FROM automations"))
            .and_then(|mut s| s.query_map([], row).map(|r| r.flatten().collect()))
            .map_err(|e| e.to_string())?;
        for a in &all {
            reschedule(&conn, a, now_ms())?;
        }
        all.iter().map(|a| a.id).collect()
    };
    for id in ids {
        emit(&app, id);
    }
    Ok(automation_timezone(lib))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        library::init(&conn).unwrap();
        conn
    }

    fn artifact(conn: &Connection, id: &str, kind: &str, created: f64, source: &str) {
        conn.execute(
            "INSERT INTO artifacts (id, kind, title, preview, created_ms, source) VALUES (?1,?2,'t','',?3,?4)",
            params![id, kind, created, source],
        )
        .unwrap();
    }

    #[test]
    fn picks_new_items_without_a_summary() {
        let conn = db();
        artifact(&conn, "note:old", "note", 10.0, "");
        artifact(&conn, "note:a", "note", 200.0, "");
        artifact(&conn, "note:b", "note", 300.0, "");
        artifact(&conn, "summary:1", "summary", 400.0, "summarize of note:b");
        artifact(&conn, "image:x", "image", 500.0, "");
        assert_eq!(unsummarized_since(&conn, 100), vec!["note:a"]);
    }

    #[test]
    fn changing_the_zone_moves_the_next_run() {
        let conn = db();
        conn.execute(
            "INSERT INTO automations (name, cron, action) VALUES ('n','0 0 * * *','summarize_new')",
            [],
        )
        .unwrap();
        let a = load(&conn, 1).unwrap();
        let from = 1_780_000_000_000;
        conn.execute("INSERT INTO automation_settings VALUES ('timezone','UTC')", [])
            .unwrap();
        reschedule(&conn, &a, from).unwrap();
        let utc = load(&conn, 1).unwrap().next_run_ms.unwrap();
        conn.execute("UPDATE automation_settings SET value = 'Asia/Kolkata'", [])
            .unwrap();
        reschedule(&conn, &a, from).unwrap();
        let ist = load(&conn, 1).unwrap().next_run_ms.unwrap();
        assert_ne!(utc, ist);
    }

    #[test]
    fn a_disabled_automation_has_no_next_run() {
        let conn = db();
        conn.execute(
            "INSERT INTO automations (name, cron, action, enabled) VALUES ('n','0 0 * * *','summarize_new',0)",
            [],
        )
        .unwrap();
        let a = load(&conn, 1).unwrap();
        reschedule(&conn, &a, 0).unwrap();
        assert!(load(&conn, 1).unwrap().next_run_ms.is_none());
    }
}
