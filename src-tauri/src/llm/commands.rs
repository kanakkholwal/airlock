use super::engine::{Backend, EngineState, LlmEngine, Residency};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use specta::Type;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, State};

pub struct ChatCancel(pub AtomicBool);

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct WireMessage {
    pub role: String,
    pub content: String,
    /// Stored attachment paths (see `chat_attach_image`); only user messages carry images.
    #[serde(default)]
    pub images: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatEvent {
    Text {
        delta: String,
    },
    Reasoning {
        delta: String,
    },
    /// The model asked for a tool; `title` says what it does in words.
    ToolCall {
        call_id: String,
        name: String,
        arguments: String,
        title: String,
    },
    /// The call needs the person's yes before it runs.
    ToolApproval {
        call_id: String,
        name: String,
        title: String,
        risk: crate::agent::tools::Risk,
    },
    ToolResult {
        call_id: String,
        output: String,
        is_error: bool,
    },
    Done {
        tokens_per_second: Option<f64>,
        /// Tokens the model read for this reply, including any cached prefix.
        prompt_tokens: Option<u64>,
        completion_tokens: Option<u64>,
    },
    Error {
        message: String,
    },
}

/// One tool call a model asked for, put back together from its streamed pieces.
#[derive(Debug, Default, Clone)]
pub(crate) struct ToolCallReq {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

/// What the server reports once a stream ends, and any tool calls it asked for.
#[derive(Debug, Default, Clone)]
pub(crate) struct StreamStats {
    pub tokens_per_second: Option<f64>,
    pub prompt_tokens: Option<u64>,
    pub completion_tokens: Option<u64>,
    pub calls: Vec<ToolCallReq>,
}

/// OpenAI-style messages; a message with images becomes a list of text and image parts.
fn to_json_messages(app: &AppHandle, messages: &[WireMessage]) -> Result<Vec<Value>, String> {
    messages
        .iter()
        .map(|m| {
            if m.images.is_empty() {
                return Ok(json!({ "role": m.role, "content": m.content }));
            }
            let mut parts = vec![json!({ "type": "text", "text": m.content })];
            for path in &m.images {
                let url = super::attachments::data_url(app, path)?;
                parts.push(json!({ "type": "image_url", "image_url": { "url": url } }));
            }
            Ok(json!({ "role": m.role, "content": parts }))
        })
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct SpeedResult {
    pub target: String,
    pub backend: String,
    pub value: f64,
    pub unit: String,
}

fn engines_dir(app: &AppHandle) -> Result<PathBuf, String> {
    super::paths::engines_dir(app)
}

async fn ready_port(engine: &LlmEngine) -> Result<(u16, Backend, String), String> {
    let key = engine.api_key().await.unwrap_or_default();
    match engine.state().await {
        EngineState::Ready { port, backend, .. } => Ok((port, backend, key)),
        EngineState::Loading { .. } => Err("The model is still loading".into()),
        EngineState::Failed { reason } => Err(reason),
        EngineState::Stopped => Err("No model is loaded".into()),
    }
}

#[tauri::command]
#[specta::specta]
pub async fn llm_status(engine: State<'_, Arc<LlmEngine>>) -> Result<EngineState, String> {
    Ok(engine.state().await)
}

#[tauri::command]
#[specta::specta]
pub async fn llm_start_model(
    app: AppHandle,
    engine: State<'_, Arc<LlmEngine>>,
    id: String,
    backend: Backend,
) -> Result<(), String> {
    let models = super::models::models_dir(&app)?;
    let (path, projector, context) =
        super::models::resolve(&models, &id).ok_or("Model is not downloaded yet")?;
    super::paths::remember_model(&app, &id);
    engine.configure(super::paths::residency(&app));
    engine
        .inner()
        .start(
            app.clone(),
            engines_dir(&app)?,
            id,
            path,
            projector,
            context,
            backend,
        )
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn llm_stop(app: AppHandle, engine: State<'_, Arc<LlmEngine>>) -> Result<(), String> {
    engine.stop(&app).await;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn chat_cancel(cancel: State<'_, ChatCancel>) {
    cancel.0.store(true, Ordering::SeqCst);
}

/// Streams one completion over server-sent events, handing each delta to `on_chunk` as it
/// arrives; nothing is buffered beyond one network read. Returns the generation speed the
/// server reports in its final chunk. Provenance: Handy llm_client.rs (OpenAI-compatible client);
/// reasoning deltas map to the `reasoning` part type.
pub(crate) async fn stream_completion(
    port: u16,
    key: &str,
    body: Value,
    cancel: &AtomicBool,
    mut on_chunk: impl FnMut(ChatEvent),
) -> Result<StreamStats, String> {
    let mut res = reqwest::Client::new()
        .post(format!("http://127.0.0.1:{port}/v1/chat/completions"))
        .bearer_auth(key)
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !res.status().is_success() {
        return Err(format!("Engine returned {}", res.status()));
    }

    let mut buf = String::new();
    let mut stats = StreamStats::default();
    'read: while let Some(chunk) = res.chunk().await.map_err(|e| e.to_string())? {
        if cancel.load(Ordering::SeqCst) {
            break;
        }
        buf.push_str(&String::from_utf8_lossy(&chunk));
        while let Some(end) = buf.find('\n') {
            let line: String = buf.drain(..=end).collect();
            let Some(data) = line.trim().strip_prefix("data:") else {
                continue;
            };
            let data = data.trim();
            if data == "[DONE]" {
                break 'read;
            }
            let Ok(v) = serde_json::from_str::<Value>(data) else {
                continue;
            };
            let timings = &v["timings"];
            if let Some(t) = timings["predicted_per_second"].as_f64() {
                stats.tokens_per_second = Some(t);
                // Prompt tokens are the evaluated ones plus any reused from the cache.
                stats.prompt_tokens = Some(
                    timings["prompt_n"].as_u64().unwrap_or(0)
                        + timings["cache_n"].as_u64().unwrap_or(0),
                );
                stats.completion_tokens = timings["predicted_n"].as_u64();
            }
            let delta = &v["choices"][0]["delta"];
            if let Some(s) = delta["reasoning_content"]
                .as_str()
                .filter(|s| !s.is_empty())
            {
                on_chunk(ChatEvent::Reasoning { delta: s.into() });
            }
            if let Some(s) = delta["content"].as_str().filter(|s| !s.is_empty()) {
                on_chunk(ChatEvent::Text { delta: s.into() });
            }
            // Tool calls arrive in pieces keyed by `index`: the name once, the arguments a bit
            // at a time. Some servers send a whole call in one piece, which fits the same shape.
            if let Some(pieces) = delta["tool_calls"].as_array() {
                for piece in pieces {
                    let at = piece["index"].as_u64().unwrap_or(0) as usize;
                    while stats.calls.len() <= at {
                        stats.calls.push(ToolCallReq::default());
                    }
                    let call = &mut stats.calls[at];
                    if let Some(id) = piece["id"].as_str().filter(|s| !s.is_empty()) {
                        call.id = id.into();
                    }
                    if let Some(name) = piece["function"]["name"].as_str().filter(|s| !s.is_empty()) {
                        call.name = name.into();
                    }
                    if let Some(args) = piece["function"]["arguments"].as_str() {
                        call.arguments.push_str(args);
                    }
                }
            }
        }
    }
    Ok(stats)
}

#[tauri::command]
#[specta::specta]
pub async fn chat_stream(
    app: AppHandle,
    engine: State<'_, Arc<LlmEngine>>,
    cancel: State<'_, ChatCancel>,
    messages: Vec<WireMessage>,
    on_event: Channel<ChatEvent>,
) -> Result<(), String> {
    let (port, _, key) = ready_port(&engine).await?;
    let _busy = engine.inner().begin();
    cancel.0.store(false, Ordering::SeqCst);
    let wire = match to_json_messages(&app, &messages) {
        Ok(m) => m,
        Err(message) => {
            let _ = on_event.send(ChatEvent::Error {
                message: message.clone(),
            });
            return Err(message);
        }
    };
    match crate::agent::run::chat_loop(&app, port, &key, wire, &cancel.0, &on_event).await {
        Ok(stats) => {
            let _ = on_event.send(ChatEvent::Done {
                tokens_per_second: stats.tokens_per_second,
                prompt_tokens: stats.prompt_tokens,
                completion_tokens: stats.completion_tokens,
            });
            Ok(())
        }
        Err(message) => {
            let _ = on_event.send(ChatEvent::Error {
                message: message.clone(),
            });
            Err(message)
        }
    }
}

/// Measures generation speed on the loaded model, streaming like real chat and discarding the
/// text. The dictation and combined targets are measured on the transcription side.
#[tauri::command]
#[specta::specta]
pub async fn speed_test_chat(
    engine: State<'_, Arc<LlmEngine>>,
) -> Result<Vec<SpeedResult>, String> {
    let (port, backend, key) = ready_port(&engine).await?;
    let _busy = engine.inner().begin();
    let body = json!({
        "messages": [{ "role": "user", "content": "Write a short story about a lighthouse." }],
        "max_tokens": 128,
        "stream": true,
    });
    let never = AtomicBool::new(false);
    let tps = stream_completion(port, &key, body, &never, |_| {})
        .await?
        .tokens_per_second
        .ok_or("The engine did not report timings")?;
    Ok(vec![SpeedResult {
        target: "chat".into(),
        backend: format!("{backend:?}").to_lowercase(),
        value: tps,
        unit: "tok/s".into(),
    }])
}

/// Writes a short title for a conversation from its first exchange. Runs as a queued background
/// call after the first reply, so it never competes with the reply being written.
#[tauri::command]
#[specta::specta]
pub async fn chat_title(
    engine: State<'_, Arc<LlmEngine>>,
    first_user: String,
    first_reply: String,
) -> Result<String, String> {
    let (port, _, key) = ready_port(&engine).await?;
    let _busy = engine.inner().begin();
    let clip = |s: &str, n: usize| s.chars().take(n).collect::<String>();
    let body = json!({
        "messages": [
            { "role": "system", "content": "Write a title of at most six words for this conversation. Reply with the title only: no quotes, no trailing punctuation." },
            { "role": "user", "content": format!("User: {}

Assistant: {}", clip(&first_user, 600), clip(&first_reply, 600)) },
        ],
        "max_tokens": 24,
        "temperature": 0.3,
        "stream": true,
    });
    let never = AtomicBool::new(false);
    let mut title = String::new();
    stream_completion(port, &key, body, &never, |e| {
        if let ChatEvent::Text { delta } = e {
            title.push_str(&delta);
        }
    })
    .await?;
    Ok(clean_title(&title))
}

fn clean_title(raw: &str) -> String {
    let line = raw.trim().lines().next().unwrap_or("");
    let trimmed = line
        .trim_matches(|c: char| c == '"' || c == '\'' || c == '*' || c == '#' || c.is_whitespace())
        .trim_end_matches(['.', '!', ':']);
    trimmed.chars().take(60).collect()
}

#[cfg(test)]
mod tests {
    use super::clean_title;

    #[test]
    fn titles_are_cleaned_to_one_short_line() {
        assert_eq!(
            clean_title(
                "  \"Planning a Lisbon trip.\"
extra"
            ),
            "Planning a Lisbon trip"
        );
        assert_eq!(clean_title("**Rust lifetimes**"), "Rust lifetimes");
        assert_eq!(clean_title(""), "");
        assert_eq!(clean_title(&"x".repeat(200)).chars().count(), 60);
    }
}

#[tauri::command]
#[specta::specta]
pub fn llm_get_residency(app: AppHandle) -> Residency {
    super::paths::residency(&app)
}

#[tauri::command]
#[specta::specta]
pub fn llm_set_residency(
    app: AppHandle,
    engine: State<'_, Arc<LlmEngine>>,
    idle_minutes: u32,
    unload_when_hidden: bool,
) -> Result<(), String> {
    let r = Residency {
        idle_minutes: idle_minutes.min(24 * 60),
        unload_when_hidden,
    };
    super::paths::save_residency(&app, r)?;
    engine.configure(r);
    Ok(())
}

/// The window reports when it is hidden or shown, so the model can unload sooner while only
/// dictation is in use.
#[tauri::command]
#[specta::specta]
pub fn llm_set_window_hidden(engine: State<'_, Arc<LlmEngine>>, hidden: bool) {
    engine.set_hidden(hidden);
}

pub fn manage(app: &AppHandle) {
    let engine = Arc::new(LlmEngine::default());
    engine.configure(super::paths::residency(app));
    app.manage(engine);
    app.manage(ChatCancel(AtomicBool::new(false)));
}
