use super::engine::{Backend, EngineState, LlmEngine};
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
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatEvent {
    Text { delta: String },
    Reasoning { delta: String },
    Done { tokens_per_second: Option<f64> },
    Error { message: String },
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

async fn ready_port(engine: &LlmEngine) -> Result<(u16, Backend), String> {
    match engine.state().await {
        EngineState::Ready { port, backend, .. } => Ok((port, backend)),
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

// Streams one reply as typed events. Provenance: Handy llm_client.rs (OpenAI-compatible
// client); reasoning deltas map to the `reasoning` part type.
#[tauri::command]
#[specta::specta]
pub async fn chat_stream(
    engine: State<'_, Arc<LlmEngine>>,
    cancel: State<'_, ChatCancel>,
    messages: Vec<WireMessage>,
    on_event: Channel<ChatEvent>,
) -> Result<(), String> {
    let (port, _) = ready_port(&engine).await?;
    cancel.0.store(false, Ordering::SeqCst);

    let body = json!({ "messages": messages, "stream": true });
    let mut res = reqwest::Client::new()
        .post(format!("http://127.0.0.1:{port}/v1/chat/completions"))
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !res.status().is_success() {
        let message = format!("Engine returned {}", res.status());
        let _ = on_event.send(ChatEvent::Error {
            message: message.clone(),
        });
        return Err(message);
    }

    let mut buf = String::new();
    let mut tps = None;
    'read: while let Some(chunk) = res.chunk().await.map_err(|e| e.to_string())? {
        if cancel.0.load(Ordering::SeqCst) {
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
            if let Some(t) = v["timings"]["predicted_per_second"].as_f64() {
                tps = Some(t);
            }
            let delta = &v["choices"][0]["delta"];
            if let Some(s) = delta["reasoning_content"]
                .as_str()
                .filter(|s| !s.is_empty())
            {
                let _ = on_event.send(ChatEvent::Reasoning { delta: s.into() });
            }
            if let Some(s) = delta["content"].as_str().filter(|s| !s.is_empty()) {
                let _ = on_event.send(ChatEvent::Text { delta: s.into() });
            }
        }
    }
    let _ = on_event.send(ChatEvent::Done {
        tokens_per_second: tps,
    });
    Ok(())
}

/// Measures generation speed on the loaded model; the dictation and combined targets
/// are measured on the transcription side and wired up separately.
#[tauri::command]
#[specta::specta]
pub async fn speed_test_chat(
    engine: State<'_, Arc<LlmEngine>>,
) -> Result<Vec<SpeedResult>, String> {
    let (port, backend) = ready_port(&engine).await?;
    let body = json!({
        "messages": [{ "role": "user", "content": "Write a short story about a lighthouse." }],
        "max_tokens": 128,
        "stream": false,
    });
    let v: Value = reqwest::Client::new()
        .post(format!("http://127.0.0.1:{port}/v1/chat/completions"))
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    let tps = v["timings"]["predicted_per_second"]
        .as_f64()
        .ok_or("The engine did not report timings")?;
    Ok(vec![SpeedResult {
        target: "chat".into(),
        backend: format!("{backend:?}").to_lowercase(),
        value: tps,
        unit: "tok/s".into(),
    }])
}

pub fn manage(app: &AppHandle) {
    app.manage(Arc::new(LlmEngine::default()));
    app.manage(ChatCancel(AtomicBool::new(false)));
}
