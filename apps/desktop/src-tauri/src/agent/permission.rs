//! Decides whether a tool call runs, asks the person, or is refused.
//!
//! Design provenance: the order of checks and the `deny > ask > allow` rule precedence follow
//! the permission pipelines of xai-org/grok-build and openai/codex (both Apache-2.0). This is a
//! small reimplementation for Airlock's own tools, not a copy of their code.
//!
//! Decision (product): there is no "approve everything" mode. A private assistant that can be
//! switched to skip every question is the one setting that defeats the rest.

use super::tools::{Risk, ToolDef};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use specta::Type;

/// The baseline for calls that have no rule. Rules still apply on top of every mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    /// Reading ordinary things runs; private reads and anything that writes ask first.
    #[default]
    Ask,
    /// Like Ask, but saving notes runs without asking. Private reads still ask.
    AcceptWrites,
    /// Nothing asks. Only what a rule allows runs, and everything else is refused.
    DontAsk,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Allow,
    Ask,
    Deny,
}

/// `tool` is a tool name or `*`. `pattern` narrows it to calls whose subject matches, where `*`
/// stands for any run of characters. No pattern means every call of that tool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct Rule {
    pub action: Action,
    pub tool: String,
    #[serde(default)]
    pub pattern: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    Allow,
    Ask,
    Deny(String),
}

/// The part of a call a rule can match on: the path, the id, or the title.
pub fn subject(args: &Value) -> String {
    for key in ["path", "id", "title", "query"] {
        match &args[key] {
            Value::String(s) => return s.clone(),
            Value::Number(n) => return n.to_string(),
            _ => {}
        }
    }
    String::new()
}

/// Matches `text` against a pattern where `*` is any run of characters (including none).
pub fn wildcard(pattern: &str, text: &str) -> bool {
    let parts: Vec<&str> = pattern.split('*').collect();
    if parts.len() == 1 {
        return pattern == text;
    }
    let mut rest = text;
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        if i == 0 {
            match rest.strip_prefix(part) {
                Some(r) => rest = r,
                None => return false,
            }
        } else if i == parts.len() - 1 {
            return rest.ends_with(part);
        } else {
            match rest.find(part) {
                Some(at) => rest = &rest[at + part.len()..],
                None => return false,
            }
        }
    }
    true
}

fn matches(rule: &Rule, tool: &str, subject: &str) -> bool {
    (rule.tool == "*" || rule.tool == tool)
        && rule
            .pattern
            .as_deref()
            .is_none_or(|p| wildcard(&p.replace('\\', "/"), &subject.replace('\\', "/")))
}

/// The checks, in order: a deny rule, an ask rule, an allow rule (yours, then remembered
/// grants), then what the tool's own risk and the mode say.
pub fn decide(
    mode: Mode,
    rules: &[Rule],
    grants: &[Rule],
    tool: &ToolDef,
    args: &Value,
) -> Decision {
    let subject = subject(args);
    let hit = |action: Action, list: &[Rule]| {
        list.iter()
            .any(|r| r.action == action && matches(r, tool.name, &subject))
    };

    if hit(Action::Deny, rules) {
        return Decision::Deny("A rule you set blocks this.".into());
    }
    if hit(Action::Ask, rules) {
        return Decision::Ask;
    }
    if hit(Action::Allow, rules) || hit(Action::Allow, grants) {
        return Decision::Allow;
    }
    match (tool.risk, mode) {
        (Risk::Read, _) => Decision::Allow,
        (Risk::Write, Mode::AcceptWrites) => Decision::Allow,
        (_, Mode::DontAsk) => {
            Decision::Deny("Asking is turned off, so only allowed tools can run.".into())
        }
        _ => Decision::Ask,
    }
}

/// Checks the arguments a model sent against the tool's own schema before anything runs, so a
/// small model that forgets a field is told what is missing instead of getting a vague failure.
pub fn check_args(tool: &ToolDef, args: &Value) -> Result<(), String> {
    let schema = (tool.parameters)();
    if !args.is_object() {
        return Err("The arguments must be a JSON object.".into());
    }
    if let Some(required) = schema["required"].as_array() {
        for key in required.iter().filter_map(|k| k.as_str()) {
            if args.get(key).is_none_or(|v| v.is_null()) {
                return Err(format!("Missing required argument: {key}"));
            }
        }
    }
    if let Some(props) = schema["properties"].as_object() {
        for key in args.as_object().into_iter().flat_map(|o| o.keys()) {
            if !props.contains_key(key) {
                return Err(format!("Unknown argument: {key}"));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::tools::find;
    use serde_json::json;

    fn rule(action: Action, tool: &str, pattern: Option<&str>) -> Rule {
        Rule {
            action,
            tool: tool.into(),
            pattern: pattern.map(Into::into),
        }
    }

    #[test]
    fn wildcards_match_any_run_of_characters() {
        assert!(wildcard("notes/*", "notes/a.md"));
        assert!(wildcard("*.md", "a/b/c.md"));
        assert!(wildcard("a*c", "abbbc"));
        assert!(!wildcard("a*c", "abbbd"));
        assert!(wildcard("exact", "exact"));
        assert!(!wildcard("exact", "exactly"));
        assert!(wildcard("*", ""));
    }

    #[test]
    fn deny_beats_ask_beats_allow() {
        let tool = find("save_note").unwrap();
        let args = json!({"title": "Plan", "text": "x"});
        let all = [
            rule(Action::Allow, "save_note", None),
            rule(Action::Ask, "save_note", None),
            rule(Action::Deny, "save_note", Some("Pla*")),
        ];
        assert!(matches!(
            decide(Mode::Ask, &all, &[], tool, &args),
            Decision::Deny(_)
        ));
        assert_eq!(
            decide(Mode::Ask, &all[..2], &[], tool, &args),
            Decision::Ask
        );
        assert_eq!(
            decide(Mode::Ask, &all[..1], &[], tool, &args),
            Decision::Allow
        );
    }

    #[test]
    fn modes_set_the_baseline_for_calls_without_a_rule() {
        let read = find("get_time").unwrap();
        let private = find("read_item").unwrap();
        let write = find("save_note").unwrap();
        let none = json!({});
        assert_eq!(decide(Mode::Ask, &[], &[], read, &none), Decision::Allow);
        assert_eq!(
            decide(Mode::Ask, &[], &[], private, &json!({"id": "recording:3"})),
            Decision::Ask
        );
        assert_eq!(
            decide(
                Mode::Ask,
                &[],
                &[],
                write,
                &json!({"title": "t", "text": "x"})
            ),
            Decision::Ask
        );
        assert_eq!(
            decide(
                Mode::AcceptWrites,
                &[],
                &[],
                write,
                &json!({"title": "t", "text": "x"})
            ),
            Decision::Allow
        );
        // Accepting writes never opens up private reads.
        assert_eq!(
            decide(
                Mode::AcceptWrites,
                &[],
                &[],
                private,
                &json!({"id": "recording:3"})
            ),
            Decision::Ask
        );
        assert!(matches!(
            decide(
                Mode::DontAsk,
                &[],
                &[],
                private,
                &json!({"id": "recording:3"})
            ),
            Decision::Deny(_)
        ));
        assert_eq!(
            decide(Mode::DontAsk, &[], &[], read, &none),
            Decision::Allow
        );
    }

    #[test]
    fn a_remembered_grant_allows_without_asking() {
        let tool = find("read_item").unwrap();
        let grants = [rule(Action::Allow, "read_item", Some("recording:7"))];
        assert_eq!(
            decide(Mode::Ask, &[], &grants, tool, &json!({"id": "recording:7"})),
            Decision::Allow
        );
        assert_eq!(
            decide(Mode::Ask, &[], &grants, tool, &json!({"id": "recording:8"})),
            Decision::Ask
        );
    }

    #[test]
    fn a_deny_rule_beats_a_remembered_grant() {
        let tool = find("read_item").unwrap();
        let rules = [rule(Action::Deny, "*", None)];
        let grants = [rule(Action::Allow, "read_item", None)];
        assert!(matches!(
            decide(
                Mode::Ask,
                &rules,
                &grants,
                tool,
                &json!({"id": "recording:1"})
            ),
            Decision::Deny(_)
        ));
    }

    #[test]
    fn arguments_are_checked_against_the_schema() {
        let tool = find("read_text_file").unwrap();
        assert!(check_args(tool, &json!({"path": "a.txt"})).is_ok());
        assert!(check_args(tool, &json!({}))
            .unwrap_err()
            .contains("Missing"));
        assert!(check_args(tool, &json!({"path": "a", "mode": "w"}))
            .unwrap_err()
            .contains("Unknown"));
        assert!(check_args(tool, &json!("a.txt")).is_err());
    }
}
