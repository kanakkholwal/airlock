//! Speech generation: the speech models of the catalog, run by llama-server's `/tts` endpoint.
//!
//! Decision (architecture): one generic engine for many models. Every speech model is a GGUF
//! pair (model + projector) that the llama.cpp engine already installed for chat can speak, so
//! adding a voice model is a catalog entry. The server keeps the model loaded between clips and
//! is unloaded after a few idle minutes, and the chat model is unloaded first to free memory.

use crate::llm::catalog::{self, CatalogEntry, Kind};
use crate::llm::engine::{find_server, free_port, new_api_key, Backend, LlmEngine};
use crate::llm::{models, paths};
use crate::tts::{cache_dir, prune_cache, wav_seconds, SpeechClip};
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use specta::Type;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager, State};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

const MAX_TEXT: usize = 2000;
const MAX_REFERENCE_BYTES: u64 = 25 * 1024 * 1024;
const IDLE_UNLOAD: Duration = Duration::from_secs(300);
const LOAD_TIMEOUT: Duration = Duration::from_secs(240);

struct Running {
    id: String,
    port: u16,
    key: String,
    voices: Vec<String>,
    used: Instant,
    child: Child,
}

#[derive(Default)]
pub struct VoiceGen {
    inner: Mutex<Option<Running>>,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct VoiceGenStatus {
    /// The speech model in memory, if any.
    pub loaded: Option<String>,
    /// Preset voices the loaded model reports.
    pub voices: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Type)]
pub struct VoiceGenRequest {
    pub id: String,
    pub text: String,
    pub voice: Option<String>,
    pub language: Option<String>,
    pub speed: Option<f32>,
    pub seed: Option<i32>,
    pub temperature: Option<f32>,
    /// A written description of the voice to create (models with `design`).
    pub design: Option<String>,
    /// A clip of the voice to copy (models with `clone`).
    pub reference_path: Option<String>,
    pub reference_text: Option<String>,
    pub flow_steps: Option<u32>,
    pub cfg_scale: Option<f32>,
}

pub fn manage(app: &AppHandle) {
    app.manage(Arc::new(VoiceGen::default()));
}

fn speech_entry(id: &str) -> Result<&'static CatalogEntry, String> {
    catalog::find(id)
        .filter(|e| e.kind == Kind::Speech)
        .ok_or_else(|| "Unknown voice model".to_string())
}

fn status_of(run: &Option<Running>) -> VoiceGenStatus {
    VoiceGenStatus {
        loaded: run.as_ref().map(|r| r.id.clone()),
        voices: run.as_ref().map(|r| r.voices.clone()).unwrap_or_default(),
    }
}

/// The request body for `/tts`. Only settings the model lists as controls are sent: the server
/// answers 400 to a setting its model does not support.
fn build_body(
    entry: &CatalogEntry,
    req: &VoiceGenRequest,
    reference: Option<(String, Option<String>)>,
) -> Result<Value, String> {
    let has = |c: &str| entry.controls.contains(&c);
    let text = req.text.trim();
    if text.is_empty() {
        return Err("Type something to say".into());
    }
    if text.chars().count() > MAX_TEXT {
        return Err(format!("Keep it under {MAX_TEXT} characters"));
    }
    let mut spoken = text.to_string();
    if has("design") {
        let design = req.design.as_deref().unwrap_or("").trim();
        if !design.is_empty() {
            // A voice description goes in parentheses before the text.
            let clean: String = design
                .chars()
                .filter(|c| *c != '(' && *c != ')')
                .take(300)
                .collect();
            spoken = format!("({clean}){text}");
        }
    }
    let mut body = Map::new();
    body.insert("input".into(), json!(spoken));
    if let Some(v) = req
        .voice
        .as_deref()
        .filter(|_| has("voice") && reference.is_none())
    {
        body.insert("voice".into(), json!(v));
    }
    if let Some(l) = req.language.as_deref().filter(|_| has("language")) {
        body.insert("lang".into(), json!(l));
    }
    if let Some(s) = req.speed.filter(|_| has("speed")) {
        body.insert("speed".into(), json!(s.clamp(0.25, 4.0)));
    }
    if let Some(s) = req.seed.filter(|_| has("seed")) {
        body.insert("seed".into(), json!(s));
    }
    if let Some(t) = req.temperature.filter(|_| has("temperature")) {
        body.insert("temperature".into(), json!(t.clamp(0.0, 2.0)));
    }
    if let Some(n) = req.flow_steps.filter(|_| has("flow_steps")) {
        body.insert("flow_steps".into(), json!(n.clamp(2, 32)));
    }
    if let Some(c) = req.cfg_scale.filter(|_| has("cfg_scale")) {
        body.insert("cfg_scale".into(), json!(c.clamp(0.1, 10.0)));
    }
    if let Some((b64, transcript)) = reference.filter(|_| has("clone")) {
        body.insert("speaker_ref_b64".into(), json!(b64));
        if let Some(t) = transcript.filter(|_| has("clone_text")) {
            body.insert("speaker_ref_text".into(), json!(t));
        }
    }
    Ok(Value::Object(body))
}

fn read_reference(req: &VoiceGenRequest) -> Result<Option<(String, Option<String>)>, String> {
    let Some(path) = req.reference_path.as_deref().filter(|p| !p.is_empty()) else {
        return Ok(None);
    };
    let path = PathBuf::from(path);
    let len = std::fs::metadata(&path)
        .map_err(|_| "The reference clip could not be read".to_string())?
        .len();
    if len > MAX_REFERENCE_BYTES {
        return Err("The reference clip is too large (25 MB at most)".into());
    }
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    let text = req
        .reference_text
        .as_deref()
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(str::to_string);
    Ok(Some((
        base64::engine::general_purpose::STANDARD.encode(bytes),
        text,
    )))
}

fn http() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .no_proxy()
        .build()
        .map_err(|e| e.to_string())
}

async fn stop_running(slot: &mut Option<Running>) {
    if let Some(mut run) = slot.take() {
        let _ = run.child.kill().await;
    }
}

/// Starts the model's server, unloading chat first. Returns the status once it answers.
#[tauri::command]
#[specta::specta]
pub async fn voice_gen_load(
    app: AppHandle,
    gen: State<'_, Arc<VoiceGen>>,
    engine: State<'_, Arc<LlmEngine>>,
    id: String,
) -> Result<VoiceGenStatus, String> {
    speech_entry(&id)?;
    let mut slot = gen.inner.lock().await;
    if slot.as_ref().is_some_and(|r| r.id == id) {
        return Ok(status_of(&slot));
    }
    stop_running(&mut slot).await;

    let dir = models::models_dir(&app)?;
    let (model, projector, _) = models::resolve(&dir, &id).ok_or("Download this voice first")?;
    let projector = projector.ok_or("This voice model is missing its second file")?;
    let engines = paths::engines_dir(&app)?;
    let (exe, backend) = find_server(&engines, paths::engine_pref(&app))
        .or_else(|| find_server(&engines, Backend::Auto))
        .ok_or("Install an engine first (Models > Engine)")?;

    // Free the memory the chat model holds; it reloads on the next message.
    engine.stop(&app).await;

    let port = free_port().map_err(|e| e.to_string())?;
    let key = new_api_key();
    let log_path = crate::portable::app_data_dir(&app)
        .map_err(|e| e.to_string())?
        .join("voice-engine.log");
    let log = std::fs::File::create(&log_path).map_err(|e| e.to_string())?;
    let mut cmd = Command::new(&exe);
    cmd.arg("-m")
        .arg(&model)
        .arg("-mm")
        .arg(&projector)
        .args(["--host", "127.0.0.1", "--port", &port.to_string()])
        .args(["--api-key", &key, "-np", "1"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(log)
        .kill_on_drop(true);
    if backend == Backend::Cpu {
        cmd.args(["--device", "none"]);
    } else {
        cmd.args(["--fit", "on", "--fit-target", "512"]);
    }
    #[cfg(windows)]
    cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Could not start the voice engine: {e}"))?;
    crate::job_object::bind(&child);

    let client = http()?;
    let base = format!("http://127.0.0.1:{port}");
    let started = Instant::now();
    loop {
        if let Ok(Some(status)) = child.try_wait() {
            let tail = std::fs::read_to_string(&log_path).unwrap_or_default();
            let last: Vec<&str> = tail.lines().rev().take(3).collect();
            return Err(format!(
                "The voice engine stopped ({status}).\n{}",
                last.into_iter().rev().collect::<Vec<_>>().join("\n")
            ));
        }
        let up = client
            .get(format!("{base}/health"))
            .timeout(Duration::from_secs(2))
            .send()
            .await
            .is_ok_and(|r| r.status().is_success());
        if up {
            break;
        }
        if started.elapsed() > LOAD_TIMEOUT {
            let _ = child.kill().await;
            return Err("The voice model took too long to load".into());
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    let props: Value = match client
        .get(format!("{base}/props"))
        .bearer_auth(&key)
        .send()
        .await
    {
        Ok(r) => r.json().await.unwrap_or_default(),
        Err(_) => Value::Null,
    };
    let voices: Vec<String> = props["voices"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();

    *slot = Some(Running {
        id,
        port,
        key,
        voices,
        used: Instant::now(),
        child,
    });
    Ok(status_of(&slot))
}

#[tauri::command]
#[specta::specta]
pub async fn voice_gen_status(gen: State<'_, Arc<VoiceGen>>) -> Result<VoiceGenStatus, String> {
    Ok(status_of(&*gen.inner.lock().await))
}

#[tauri::command]
#[specta::specta]
pub async fn voice_gen_unload(gen: State<'_, Arc<VoiceGen>>) -> Result<(), String> {
    stop_running(&mut *gen.inner.lock().await).await;
    Ok(())
}

/// Speaks `req.text` and returns the clip. The model must be loaded first.
#[tauri::command]
#[specta::specta]
pub async fn voice_gen_say(
    app: AppHandle,
    gen: State<'_, Arc<VoiceGen>>,
    req: VoiceGenRequest,
) -> Result<SpeechClip, String> {
    let entry = speech_entry(&req.id)?;
    let reference = read_reference(&req)?;
    let body = build_body(entry, &req, reference)?;
    let (port, key) = {
        let mut slot = gen.inner.lock().await;
        let run = slot
            .as_mut()
            .filter(|r| r.id == req.id)
            .ok_or("Load the voice model first")?;
        run.used = Instant::now();
        (run.port, run.key.clone())
    };
    let res = http()?
        .post(format!("http://127.0.0.1:{port}/tts"))
        .bearer_auth(&key)
        .json(&body)
        .timeout(Duration::from_secs(300))
        .send()
        .await
        .map_err(|e| format!("The voice engine did not answer: {e}"))?;
    if !res.status().is_success() {
        let status = res.status();
        let detail = res.text().await.unwrap_or_default();
        let msg = serde_json::from_str::<Value>(&detail)
            .ok()
            .and_then(|v| v["error"]["message"].as_str().map(str::to_string))
            .unwrap_or(detail);
        return Err(format!("The voice engine refused ({status}): {msg}"));
    }
    let bytes = res.bytes().await.map_err(|e| e.to_string())?;

    let dir = cache_dir(&app)?;
    prune_cache(&dir);
    let out = dir.join(format!(
        "{}-{:04x}.wav",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0),
        rand::random::<u16>()
    ));
    std::fs::write(&out, &bytes).map_err(|e| e.to_string())?;

    // Unload after a quiet spell so a big voice model does not sit in memory.
    let gen = gen.inner().clone();
    let id = req.id.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(IDLE_UNLOAD).await;
        let mut slot = gen.inner.lock().await;
        if slot
            .as_ref()
            .is_some_and(|r| r.id == id && r.used.elapsed() >= IDLE_UNLOAD - Duration::from_secs(5))
        {
            stop_running(&mut slot).await;
        }
    });

    Ok(SpeechClip {
        path: out.to_string_lossy().into_owned(),
        seconds: wav_seconds(&bytes).unwrap_or(0.0),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(id: &str) -> VoiceGenRequest {
        VoiceGenRequest {
            id: id.into(),
            text: "Hello there".into(),
            voice: Some("F2".into()),
            language: Some("en".into()),
            speed: Some(9.0),
            seed: Some(7),
            temperature: Some(0.5),
            design: Some("A calm (older) man".into()),
            reference_path: None,
            reference_text: None,
            flow_steps: Some(99),
            cfg_scale: Some(2.0),
        }
    }

    #[test]
    fn only_listed_controls_reach_the_server() {
        let tiny = speech_entry("tiny-voca").unwrap();
        let body = build_body(tiny, &request("tiny-voca"), None).unwrap();
        assert_eq!(body["voice"], "F2");
        assert_eq!(body["speed"], 4.0);
        assert!(body.get("lang").is_none() && body.get("flow_steps").is_none());
        assert_eq!(body["input"], "Hello there");
    }

    #[test]
    fn a_voice_description_goes_in_front_of_the_text() {
        let vox = speech_entry("voxcpm2").unwrap();
        let body = build_body(vox, &request("voxcpm2"), None).unwrap();
        assert_eq!(body["input"], "(A calm older man)Hello there");
        assert_eq!(body["flow_steps"], 32);
        assert!(body.get("voice").is_none());
    }

    #[test]
    fn a_reference_clip_replaces_a_preset_voice() {
        let zonos = speech_entry("zonos3-mini").unwrap();
        let body = build_body(zonos, &request("zonos3-mini"), Some(("QUJD".into(), None))).unwrap();
        assert_eq!(body["speaker_ref_b64"], "QUJD");
        assert!(body.get("voice").is_none());
    }

    #[test]
    fn empty_and_huge_text_are_refused() {
        let tiny = speech_entry("tiny-voca").unwrap();
        let mut r = request("tiny-voca");
        r.text = "  ".into();
        assert!(build_body(tiny, &r, None).is_err());
        r.text = "a".repeat(MAX_TEXT + 1);
        assert!(build_body(tiny, &r, None).is_err());
    }

    #[test]
    fn every_speech_entry_lists_known_controls() {
        const KNOWN: &[&str] = &[
            "voice",
            "language",
            "speed",
            "seed",
            "temperature",
            "design",
            "clone",
            "clone_text",
            "flow_steps",
            "cfg_scale",
        ];
        for e in catalog::all().iter().filter(|e| e.kind == Kind::Speech) {
            assert!(e.projector.is_some(), "{} needs its second file", e.id);
            assert!(!e.languages.is_empty(), "{} needs languages", e.id);
            assert!(e.controls.iter().all(|c| KNOWN.contains(c)), "{}", e.id);
        }
    }
}
