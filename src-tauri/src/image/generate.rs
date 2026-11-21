use super::catalog::{self, ImageModel, Role};
use crate::llm::engine::{EngineState, LlmEngine};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, State};
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::process::Command;

const MAX_PROMPT: usize = 2000;

#[derive(Debug, Clone, Deserialize, Type)]
pub struct GenerateRequest {
    pub prompt: String,
    pub model: String,
    pub width: u32,
    pub height: u32,
    pub steps: Option<u32>,
    pub seed: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ImageInfo {
    pub id: String,
    pub path: String,
    pub prompt: String,
    pub model: String,
    pub width: u32,
    pub height: u32,
    pub steps: u32,
    pub seed: u32,
    pub created_ms: f64,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ImageEvent {
    /// Waiting for a chat reply to finish before starting.
    Queued,
    /// The chat model is being unloaded and the engine started.
    Starting,
    Progress { step: u32, total: u32 },
    /// One real line from the engine's own log, so the screen shows what it is doing.
    Log { line: String },
    Done { image: ImageInfo },
    Cancelled,
    Error { message: String },
}

/// Decision (product): sizes are fixed steps, never free numbers, so a small GPU cannot be
/// asked for something it will crash on.
fn validate(req: &GenerateRequest) -> Result<(), String> {
    if req.prompt.trim().is_empty() {
        return Err("Describe the image first".into());
    }
    if req.prompt.chars().count() > MAX_PROMPT {
        return Err(format!("Keep the description under {MAX_PROMPT} characters"));
    }
    let ok_side = |n: u32| (256..=1024).contains(&n) && n % 64 == 0;
    if !ok_side(req.width) || !ok_side(req.height) {
        return Err("Width and height must be multiples of 64 between 256 and 1024".into());
    }
    if let Some(s) = req.steps {
        if !(1..=50).contains(&s) {
            return Err("Steps must be between 1 and 50".into());
        }
    }
    Ok(())
}

fn build_args(
    model: &ImageModel,
    dir: &Path,
    req: &GenerateRequest,
    steps: u32,
    seed: u32,
    out: &Path,
) -> Vec<String> {
    let part = |role: Role| {
        model
            .file(role)
            .map(|f| dir.join(f.name).to_string_lossy().into_owned())
            .unwrap_or_default()
    };
    vec![
        "--diffusion-model".into(),
        part(Role::Diffusion),
        "--llm".into(),
        part(Role::TextEncoder),
        "--vae".into(),
        part(Role::Vae),
        "-p".into(),
        req.prompt.trim().into(),
        "--cfg-scale".into(),
        model.cfg_scale.to_string(),
        "--steps".into(),
        steps.to_string(),
        "-W".into(),
        req.width.to_string(),
        "-H".into(),
        req.height.to_string(),
        "-s".into(),
        seed.to_string(),
        // 4 GB cards: the 2.5 GB text encoder runs on the CPU and weights stream to the GPU.
        "--backend".into(),
        "te=cpu".into(),
        "--offload-to-cpu".into(),
        "-o".into(),
        out.to_string_lossy().into_owned(),
    ]
}

/// Reads the `| 3/8 - 2.1s/it` progress bar sd-cli prints; ignores every other log line.
fn parse_step(line: &str) -> Option<(u32, u32)> {
    if !line.contains('|') {
        return None;
    }
    line.split_whitespace()
        .rev()
        .find_map(|tok| {
            let (a, b) = tok.split_once('/')?;
            let (step, total) = (a.parse::<u32>().ok()?, b.parse::<u32>().ok()?);
            (total > 0 && step <= total).then_some((step, total))
        })
}

/// Drops debug noise and the `[INFO ] file.cpp:123 -` prefix, leaving the sentence.
fn clean_log(line: &str) -> Option<String> {
    if line.starts_with("[DEBUG]") {
        return None;
    }
    let text = match line.split_once(" - ") {
        Some((head, rest)) if head.starts_with('[') => rest,
        _ => line,
    };
    let text = text.trim();
    (!text.is_empty()).then(|| text.chars().take(160).collect())
}

/// sd-cli redraws its bar with carriage returns, so lines end on \r as well as \n.
async fn read_lines<R: AsyncRead + Unpin>(mut r: R, mut on_line: impl FnMut(&str)) {
    let mut buf = [0u8; 4096];
    let mut pending = String::new();
    while let Ok(n) = r.read(&mut buf).await {
        if n == 0 {
            break;
        }
        pending.push_str(&String::from_utf8_lossy(&buf[..n]));
        while let Some(i) = pending.find(['\r', '\n']) {
            let line: String = pending.drain(..=i).collect();
            on_line(line.trim());
        }
    }
    if !pending.trim().is_empty() {
        on_line(pending.trim());
    }
}

fn new_id() -> (String, f64) {
    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as f64)
        .unwrap_or(0.0);
    (format!("{}-{:04x}", ms as u64, rand::random::<u16>()), ms)
}

#[tauri::command]
#[specta::specta]
pub async fn image_generate(
    app: AppHandle,
    state: State<'_, super::ImageState>,
    engine: State<'_, Arc<LlmEngine>>,
    req: GenerateRequest,
    on_event: Channel<ImageEvent>,
) -> Result<(), String> {
    let fail = |message: String| {
        let _ = on_event.send(ImageEvent::Error {
            message: message.clone(),
        });
        Err(message)
    };
    if let Err(m) = validate(&req) {
        return fail(m);
    }
    let Some(model) = catalog::find(&req.model) else {
        return fail("Unknown image model".into());
    };
    let dir = super::models_dir(&app)?;
    if !super::models::is_downloaded(&dir, model) {
        return fail("Download this model first".into());
    }
    let cli = match super::engine::find_cli(&app) {
        Ok(p) => p,
        Err(m) => return fail(m),
    };
    if state.generating.swap(true, Ordering::SeqCst) {
        return fail("An image is already being made".into());
    }
    state.cancel_generation.store(false, Ordering::SeqCst);
    let outcome = run(&app, &state, &engine, model, &dir, &cli, &req, &on_event).await;
    state.generating.store(false, Ordering::SeqCst);
    match outcome {
        Ok(Some(image)) => {
            let _ = on_event.send(ImageEvent::Done { image });
            Ok(())
        }
        Ok(None) => {
            let _ = on_event.send(ImageEvent::Cancelled);
            Ok(())
        }
        Err(m) => fail(m),
    }
}

#[allow(clippy::too_many_arguments)]
async fn run(
    app: &AppHandle,
    state: &super::ImageState,
    engine: &Arc<LlmEngine>,
    model: &ImageModel,
    dir: &Path,
    cli: &Path,
    req: &GenerateRequest,
    on_event: &Channel<ImageEvent>,
) -> Result<Option<ImageInfo>, String> {
    // The text encoder needs the same memory as a chat model, so the chat model steps aside.
    // Decision (product): work queues instead of failing. Wait for a running chat reply.
    let mut waited = Duration::ZERO;
    while engine.is_busy() {
        if waited == Duration::ZERO {
            let _ = on_event.send(ImageEvent::Queued);
        }
        if state.cancel_generation.load(Ordering::SeqCst) {
            return Ok(None);
        }
        if waited > Duration::from_secs(600) {
            return Err("The chat reply is taking too long, try again in a moment".into());
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
        waited += Duration::from_millis(250);
    }
    if !matches!(engine.state().await, EngineState::Stopped) {
        engine.stop(app).await;
    }
    // The speech model may hold GPU memory too; it reloads by itself on the next dictation.
    if let Some(tm) = app.try_state::<Arc<crate::managers::transcription::TranscriptionManager>>() {
        let _ = tm.unload_model();
    }
    let _ = on_event.send(ImageEvent::Starting);

    let steps = req.steps.unwrap_or(model.steps);
    let seed = req.seed.unwrap_or_else(rand::random);
    let (id, created_ms) = new_id();
    let out: PathBuf = super::gallery_dir(app)?.join(format!("{id}.png"));

    let mut cmd = Command::new(cli);
    cmd.args(build_args(model, dir, req, steps, seed, &out))
        .current_dir(cli.parent().unwrap_or(Path::new(".")))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    #[cfg(windows)]
    cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    let mut child = cmd.spawn().map_err(|e| format!("Could not start the image engine: {e}"))?;

    let tail = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let mut readers = Vec::new();
    for stream in [
        child.stdout.take().map(|s| Box::new(s) as Box<dyn AsyncRead + Unpin + Send>),
        child.stderr.take().map(|s| Box::new(s) as Box<dyn AsyncRead + Unpin + Send>),
    ]
    .into_iter()
    .flatten()
    {
        let (channel, tail) = (on_event.clone(), tail.clone());
        readers.push(tokio::spawn(async move {
            read_lines(stream, |line| {
                if let Some((step, total)) = parse_step(line) {
                    let _ = channel.send(ImageEvent::Progress { step, total });
                } else if !line.is_empty() {
                    if let Some(text) = clean_log(line) {
                        let _ = channel.send(ImageEvent::Log { line: text });
                    }
                    let mut t = tail.lock().unwrap();
                    t.push(line.to_string());
                    if t.len() > 12 {
                        t.remove(0);
                    }
                }
            })
            .await;
        }));
    }

    let status = loop {
        tokio::select! {
            status = child.wait() => break status.map_err(|e| e.to_string())?,
            _ = tokio::time::sleep(Duration::from_millis(200)) => {
                if state.cancel_generation.load(Ordering::SeqCst) {
                    let _ = child.kill().await;
                    let _ = std::fs::remove_file(&out);
                    return Ok(None);
                }
            }
        }
    };
    for r in readers {
        let _ = r.await;
    }
    if !status.success() || !out.exists() {
        let lines = tail.lock().unwrap();
        let detail = lines[lines.len().saturating_sub(5)..].join("\n");
        return Err(format!("The image engine stopped ({status}).\n{detail}").trim().to_string());
    }

    let image = ImageInfo {
        id: id.clone(),
        path: out.to_string_lossy().into_owned(),
        prompt: req.prompt.trim().into(),
        model: model.name.into(),
        width: req.width,
        height: req.height,
        steps,
        seed,
        created_ms,
    };
    if let Ok(json) = serde_json::to_string(&image) {
        let _ = std::fs::write(out.with_extension("json"), json);
    }
    Ok(Some(image))
}

#[tauri::command]
#[specta::specta]
pub fn image_cancel(state: State<'_, super::ImageState>) {
    state.cancel_generation.store(true, Ordering::SeqCst);
}

#[tauri::command]
#[specta::specta]
pub fn image_list(app: AppHandle) -> Result<Vec<ImageInfo>, String> {
    let dir = super::gallery_dir(&app)?;
    let mut images: Vec<ImageInfo> = std::fs::read_dir(dir)
        .map_err(|e| e.to_string())?
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
        .filter_map(|e| std::fs::read_to_string(e.path()).ok())
        .filter_map(|s| serde_json::from_str::<ImageInfo>(&s).ok())
        .filter(|i| Path::new(&i.path).exists())
        .collect();
    images.sort_by(|a, b| b.created_ms.total_cmp(&a.created_ms));
    Ok(images)
}

#[tauri::command]
#[specta::specta]
pub fn image_delete(app: AppHandle, id: String) -> Result<(), String> {
    // The id becomes a file name, so only the characters we generate are allowed.
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err("Invalid image".into());
    }
    let dir = super::gallery_dir(&app)?;
    for ext in ["png", "json"] {
        let _ = std::fs::remove_file(dir.join(format!("{id}.{ext}")));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(w: u32, h: u32, prompt: &str) -> GenerateRequest {
        GenerateRequest {
            prompt: prompt.into(),
            model: "z-image-turbo-q4".into(),
            width: w,
            height: h,
            steps: None,
            seed: None,
        }
    }

    #[test]
    fn parses_the_progress_bar() {
        assert_eq!(parse_step("|=====>    | 3/8 - 2.10s/it"), Some((3, 8)));
        assert_eq!(parse_step("[INFO ] sampling 3/8"), None);
        assert_eq!(parse_step("| 9/8 - x"), None);
    }

    #[test]
    fn rejects_empty_prompts_and_odd_sizes() {
        assert!(validate(&req(512, 512, "a cat")).is_ok());
        assert!(validate(&req(512, 512, "   ")).is_err());
        assert!(validate(&req(500, 512, "a cat")).is_err());
        assert!(validate(&req(2048, 512, "a cat")).is_err());
    }

    #[test]
    fn arguments_carry_all_three_model_parts() {
        let m = catalog::find("z-image-turbo-q4").unwrap();
        let args = build_args(m, Path::new("/m"), &req(512, 512, "a cat"), 8, 7, Path::new("/o.png"));
        for flag in ["--diffusion-model", "--llm", "--vae", "--offload-to-cpu"] {
            assert!(args.iter().any(|a| a == flag), "{flag}");
        }
    }

    #[test]
    fn cleans_engine_log_lines() {
        assert_eq!(
            clean_log("[INFO ] stable-diffusion.cpp:812 - loading tensors"),
            Some("loading tensors".into())
        );
        assert_eq!(clean_log("[DEBUG] x.cpp:1 - noise"), None);
        assert_eq!(clean_log("plain line"), Some("plain line".into()));
    }

    #[tokio::test]
    async fn splits_lines_on_carriage_returns() {
        let data: &[u8] = b"a\r| 1/2\r| 2/2\nend";
        let mut seen = Vec::new();
        read_lines(data, |l| seen.push(l.to_string())).await;
        assert_eq!(seen, vec!["a", "| 1/2", "| 2/2", "end"]);
    }
}
