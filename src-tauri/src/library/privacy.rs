//! The privacy gateway: the one place that decides what a model may see of the library.
//!
//! Decision (architecture): this is enforced here in Rust, not in the interface, so no screen,
//! tool or future workflow can route around it. A model on this computer may read what the
//! person grants. A cloud model never sees anything marked private, and for shared items sees
//! only a short preview unless the person grants the full text. Every read is written to a local
//! log the person can open.

use super::{full_text, get, Artifact, Library, Privacy};
use serde::Serialize;
use std::io::Write;
use tauri::{AppHandle, Manager};

#[allow(dead_code)] // Cloud is used once cloud providers exist; the rules for it are tested now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    /// A model running on this computer.
    Local,
    /// Any model reached over the internet.
    Cloud,
}

/// What the person has agreed to hand over for this use.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grant {
    Preview,
    Full,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Share {
    Preview,
    Full,
    Nothing(&'static str),
}

pub fn decide(privacy: Privacy, provider: Provider, grant: Grant) -> Share {
    match (provider, privacy, grant) {
        (Provider::Local, _, Grant::Full) => Share::Full,
        (Provider::Local, _, Grant::Preview) => Share::Preview,
        (Provider::Cloud, Privacy::LocalOnly, _) => {
            Share::Nothing("This is marked private, so it never leaves this computer.")
        }
        (Provider::Cloud, Privacy::Shareable, Grant::Preview) => Share::Preview,
        (Provider::Cloud, Privacy::Shareable, Grant::Full) => Share::Full,
    }
}

#[derive(Serialize)]
struct LogLine<'a> {
    at_ms: u128,
    provider: &'a str,
    item: &'a str,
    shared: &'a str,
}

/// Appends one line to the privacy log. Failing to log never blocks the read, but it is noted.
fn record(app: &AppHandle, provider: Provider, item: &str, shared: &str) {
    let Ok(dir) = crate::portable::app_data_dir(app) else {
        return;
    };
    let line = LogLine {
        at_ms: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_millis()),
        provider: if provider == Provider::Local { "local" } else { "cloud" },
        item,
        shared,
    };
    let Ok(json) = serde_json::to_string(&line) else {
        return;
    };
    let file = std::fs::OpenOptions::new().create(true).append(true).open(dir.join("privacy-log.jsonl"));
    match file {
        Ok(mut f) => {
            let _ = writeln!(f, "{json}");
        }
        Err(e) => log::warn!("Could not write the privacy log: {e}"),
    }
}

/// What a model gets for `id`, after the rules, and a log line saying so.
pub fn visible_text(app: &AppHandle, id: &str, provider: Provider, grant: Grant) -> Result<String, String> {
    let lib = app.try_state::<Library>().ok_or("The library is not available.")?;
    let conn = lib.conn.lock().unwrap();
    let artifact: Artifact = get(&conn, id).ok_or("That item is not in the library.")?;
    match decide(artifact.privacy, provider, grant) {
        Share::Nothing(why) => {
            record(app, provider, id, "nothing");
            Err(why.to_string())
        }
        Share::Preview => {
            record(app, provider, id, "preview");
            Ok(artifact.preview)
        }
        Share::Full => {
            let text = full_text(app, &conn, id)?;
            record(app, provider, id, "full");
            Ok(text)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cloud_model_never_sees_private_items() {
        for grant in [Grant::Preview, Grant::Full] {
            assert!(matches!(decide(Privacy::LocalOnly, Provider::Cloud, grant), Share::Nothing(_)));
        }
    }

    #[test]
    fn a_cloud_model_gets_a_preview_unless_the_full_text_is_granted() {
        assert_eq!(decide(Privacy::Shareable, Provider::Cloud, Grant::Preview), Share::Preview);
        assert_eq!(decide(Privacy::Shareable, Provider::Cloud, Grant::Full), Share::Full);
    }

    #[test]
    fn a_local_model_follows_the_grant_whatever_the_flag() {
        assert_eq!(decide(Privacy::LocalOnly, Provider::Local, Grant::Full), Share::Full);
        assert_eq!(decide(Privacy::LocalOnly, Provider::Local, Grant::Preview), Share::Preview);
    }
}
