use super::config;
use super::permission::{self, Action, Decision, Rule};
use super::tools;
use super::{Answer, Approvals};
use crate::llm::commands::{stream_completion, ChatEvent, StreamStats};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager};

/// A reply may use tools several times in a row (look, then read, then answer). The cap stops a
/// model that keeps calling tools without ever answering.
const MAX_STEPS: usize = 6;
const APPROVAL_TIMEOUT: Duration = Duration::from_secs(300);

/// Waits for the person's answer. Stopping the reply, or waiting too long, counts as a no.
async fn wait_for_answer(rx: tokio::sync::oneshot::Receiver<Answer>, cancel: &AtomicBool) -> Answer {
    tokio::pin!(rx);
    let started = std::time::Instant::now();
    loop {
        tokio::select! {
            answer = &mut rx => return answer.unwrap_or(Answer::Deny),
            _ = tokio::time::sleep(Duration::from_millis(200)) => {
                if cancel.load(Ordering::SeqCst) || started.elapsed() > APPROVAL_TIMEOUT {
                    return Answer::Deny;
                }
            }
        }
    }
}

/// Streams a reply and runs any tools the model asks for, then lets it continue, until it
/// answers in plain text. Every step is sent to the screen as it happens.
pub async fn chat_loop(
    app: &AppHandle,
    port: u16,
    key: &str,
    mut messages: Vec<Value>,
    cancel: &AtomicBool,
    on_event: &Channel<ChatEvent>,
) -> Result<StreamStats, String> {
    let mut config = config::load(app);
    let active = tools::active(&config);
    let specs = tools::specs(&active);
    let approvals = app.state::<Approvals>();
    let mut stats = StreamStats::default();

    for step in 0..MAX_STEPS {
        let mut body = json!({ "messages": messages, "stream": true });
        if !specs.is_empty() {
            body["tools"] = json!(specs);
        }
        let mut text = String::new();
        let outcome = stream_completion(port, key, body, cancel, |e| {
            if let ChatEvent::Text { delta } = &e {
                text.push_str(delta);
            }
            let _ = on_event.send(e);
        })
        .await?;
        stats = StreamStats {
            calls: vec![],
            ..outcome.clone()
        };
        if outcome.calls.is_empty() || cancel.load(Ordering::SeqCst) {
            return Ok(stats);
        }

        let calls: Vec<_> = outcome
            .calls
            .into_iter()
            .enumerate()
            .map(|(i, mut c)| {
                if c.id.is_empty() {
                    c.id = format!("call_{step}_{i}");
                }
                c
            })
            .collect();
        messages.push(json!({
            "role": "assistant",
            "content": text,
            "tool_calls": calls.iter().map(|c| json!({
                "id": c.id,
                "type": "function",
                "function": { "name": c.name, "arguments": c.arguments },
            })).collect::<Vec<_>>(),
        }));

        for call in calls {
            let args: Value = serde_json::from_str(&call.arguments).unwrap_or_else(|_| json!({}));
            // A model can only use a tool that was offered to it in this very request.
            let def = tools::find(&call.name).filter(|d| active.iter().any(|t| t.name == d.name));
            let title = def.map_or_else(|| format!("Using {}", call.name), |d| (d.describe)(&args));
            let _ = on_event.send(ChatEvent::ToolCall {
                call_id: call.id.clone(),
                name: call.name.clone(),
                arguments: call.arguments.clone(),
                title: title.clone(),
            });

            let (output, is_error) = match def {
                None => ("That tool is not available.".to_string(), true),
                Some(d) => match permission::check_args(d, &args) {
                    Err(problem) => (problem, true),
                    Ok(()) => {
                        let decision = permission::decide(config.mode, &config.rules, &config.grants, d, &args);
                        let allowed = match decision {
                            Decision::Allow => Ok(()),
                            Decision::Deny(why) => Err(why),
                            Decision::Ask => {
                                let rx = approvals.ask(&call.id);
                                let _ = on_event.send(ChatEvent::ToolApproval {
                                    call_id: call.id.clone(),
                                    name: call.name.clone(),
                                    title: title.clone(),
                                    risk: d.risk,
                                });
                                let answer = wait_for_answer(rx, cancel).await;
                                approvals.forget(&call.id);
                                match answer {
                                    Answer::Deny => Err("The user declined, so it was not done.".to_string()),
                                    Answer::Once => Ok(()),
                                    Answer::Always => {
                                        config.grants.push(Rule {
                                            action: Action::Allow,
                                            tool: d.name.to_string(),
                                            pattern: None,
                                        });
                                        let _ = config::save(app, &config);
                                        Ok(())
                                    }
                                }
                            }
                        };
                        match allowed {
                            Ok(()) => match (d.run)(app.clone(), config.clone(), args.clone()).await {
                                Ok(out) => (tools::clip(out), false),
                                Err(e) => (e, true),
                            },
                            Err(why) => (why, true),
                        }
                    }
                },
            };
            let _ = on_event.send(ChatEvent::ToolResult {
                call_id: call.id.clone(),
                output: output.clone(),
                is_error,
            });
            messages.push(json!({ "role": "tool", "tool_call_id": call.id, "content": output }));
            if cancel.load(Ordering::SeqCst) {
                return Ok(stats);
            }
        }
    }
    let _ = on_event.send(ChatEvent::Text {
        delta: "\n\nI stopped after several tool steps without a final answer. Ask again to continue.".into(),
    });
    Ok(stats)
}
