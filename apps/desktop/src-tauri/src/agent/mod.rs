//! The assistant's tools and the loop that lets a model use them.
//!
//! Decision (architecture): a tool is a typed contract in one registry. The chat agent calls
//! them now; Studio pages and workflows will call the same ones later, so no capability is
//! written twice. Every tool declares how risky it is, and what a model may see of the user's
//! data is decided here in Rust, not in the interface.

pub mod config;
pub mod files;
pub mod permission;
pub mod run;
pub mod tools;

use serde::Deserialize;
use specta::Type;
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::{AppHandle, Manager, State};
use tokio::sync::oneshot;

/// What the person chose on an approval card.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum Answer {
    Deny,
    Once,
    /// Yes now, and remember it as an allow rule for this tool.
    Always,
}

/// Calls waiting for the person to answer.
#[derive(Default)]
pub struct Approvals {
    pending: Mutex<HashMap<String, oneshot::Sender<Answer>>>,
}

impl Approvals {
    pub fn ask(&self, call_id: &str) -> oneshot::Receiver<Answer> {
        let (tx, rx) = oneshot::channel();
        self.pending.lock().unwrap().insert(call_id.to_string(), tx);
        rx
    }

    pub fn answer(&self, call_id: &str, answer: Answer) {
        if let Some(tx) = self.pending.lock().unwrap().remove(call_id) {
            let _ = tx.send(answer);
        }
    }

    pub fn forget(&self, call_id: &str) {
        self.pending.lock().unwrap().remove(call_id);
    }
}

pub fn manage(app: &AppHandle) {
    app.manage(Approvals::default());
}

#[tauri::command]
#[specta::specta]
pub fn agent_decide(approvals: State<'_, Approvals>, call_id: String, answer: Answer) {
    approvals.answer(&call_id, answer);
}
