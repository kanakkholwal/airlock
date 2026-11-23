//! Read-aloud with Kokoro, run by the sherpa-onnx command line tool as a sidecar.
//!
//! Decision (architecture): same pattern as images. A pinned official release is downloaded
//! on the user's action, verified against its published checksum, and run as its own process,
//! so a crash in the voice engine can never take the app down.

use crate::data::{self, BuildSource, EngineSource};
use crate::llm::download::{fetch_verified, DownloadEvent, Fetch};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, State};
use tokio::process::Command;

const MAX_TEXT: usize = 2000;

#[derive(Deserialize)]
struct RawVoice {
    id: u32,
    name: String,
    accent: String,
    gender: String,
}

#[derive(Deserialize)]
struct RawArchive {
    url: String,
    sha256: String,
    size: u64,
    folder: String,
}

#[derive(Deserialize)]
struct RawFiles {
    model: String,
    voices: String,
    tokens: String,
    lexicon: String,
}

#[derive(Deserialize)]
struct RawPack {
    id: String,
    name: String,
    #[serde(default)]
    license: String,
    archive: RawArchive,
    files: RawFiles,
    default_voice: u32,
    #[serde(default)]
    recommended: bool,
    voices: Vec<RawVoice>,
}

/// One downloadable voice pack from `catalog/voices.json`.
struct Pack {
    id: String,
    name: String,
    license: String,
    archive: RawArchive,
    files: RawFiles,
    default_voice: u32,
    voices: Vec<TtsVoice>,
}

fn pack() -> &'static Pack {
    static CELL: OnceLock<Pack> = OnceLock::new();
    CELL.get_or_init(|| {
        let mut packs = data::load::<RawPack>("voices", data::VOICES, |p| p.id.as_str());
        let at = packs.iter().position(|p| p.recommended).unwrap_or(0);
        let raw = packs.swap_remove(at);
        Pack {
            id: raw.id,
            name: raw.name,
            license: raw.license,
            archive: raw.archive,
            files: raw.files,
            default_voice: raw.default_voice,
            voices: raw
                .voices
                .into_iter()
                .map(|v| TtsVoice {
                    id: v.id,
                    name: v.name,
                    accent: v.accent,
                    gender: v.gender,
                })
                .collect(),
        }
    })
}

/// The speech engine and its build for this machine, or None where there is none.
fn engine() -> Option<(&'static EngineSource, &'static BuildSource)> {
    let source = data::engine("sherpa-tts")?;
    Some((source, source.builds.first()?))
}

fn build_bytes(build: &BuildSource) -> u64 {
    build.assets.iter().map(|a| a.size).sum()
}

#[derive(Default)]
pub struct TtsState {
    cancel: Mutex<Option<Arc<AtomicBool>>>,
}

pub fn manage(app: &AppHandle) {
    app.manage(TtsState::default());
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct TtsVoice {
    pub id: u32,
    pub name: String,
    pub accent: String,
    pub gender: String,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct TtsStatus {
    /// The voice model in use, so screens can name it (and later offer a choice).
    pub model_id: String,
    pub model_name: String,
    pub license: String,
    pub supported: bool,
    pub engine_installed: bool,
    pub model_downloaded: bool,
    pub ready: bool,
    /// Bytes still to download for the voice pack.
    pub missing_bytes: u64,
    pub total_bytes: u64,
    pub default_voice: u32,
    pub voices: Vec<TtsVoice>,
}

#[derive(Debug, Clone, Deserialize, Type)]
pub struct SpeakRequest {
    pub text: String,
    pub voice: u32,
    pub speed: f32,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct SpeechClip {
    pub path: String,
    pub seconds: f64,
}

fn exe_name() -> &'static str {
    if cfg!(windows) {
        "sherpa-onnx-offline-tts.exe"
    } else {
        "sherpa-onnx-offline-tts"
    }
}

fn engine_dir(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(crate::llm::paths::engines_dir(app)?.join("sherpa-tts"))
}

fn model_root(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = crate::llm::paths::models_dir(app)?.join("tts");
    std::fs::create_dir_all(&dir).map_err(|e| format!("Models folder unavailable: {e}"))?;
    Ok(dir)
}

fn model_dir(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(model_root(app)?.join(&pack().archive.folder))
}

pub(crate) fn cache_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = crate::portable::app_data_dir(app)
        .map_err(|e| e.to_string())?
        .join("tts-cache");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

/// The tool sits in `<release folder>/bin`, so look a few levels down.
fn find_exe(dir: &Path, depth: u32) -> Option<PathBuf> {
    let direct = dir.join(exe_name());
    if direct.is_file() {
        return Some(direct);
    }
    if depth == 0 {
        return None;
    }
    std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .filter(|e| e.path().is_dir())
        .find_map(|e| find_exe(&e.path(), depth - 1))
}

fn model_ready(dir: &Path) -> bool {
    let f = &pack().files;
    [&f.model, &f.voices, &f.tokens, &f.lexicon]
        .iter()
        .all(|name| dir.join(name).is_file())
}

fn missing_bytes(app: &AppHandle) -> Result<u64, String> {
    let mut missing = 0;
    if let Some((_, build)) = engine() {
        if find_exe(&engine_dir(app)?, 3).is_none() {
            missing += build_bytes(build);
        }
    }
    if !model_ready(&model_dir(app)?) {
        missing += pack().archive.size;
    }
    Ok(missing)
}

#[tauri::command]
#[specta::specta]
pub fn tts_status(app: AppHandle) -> Result<TtsStatus, String> {
    let engine_installed = engine().is_some() && find_exe(&engine_dir(&app)?, 3).is_some();
    let model_downloaded = model_ready(&model_dir(&app)?);
    Ok(TtsStatus {
        model_id: pack().id.clone(),
        model_name: pack().name.clone(),
        license: pack().license.clone(),
        supported: engine().is_some(),
        engine_installed,
        model_downloaded,
        ready: engine_installed && model_downloaded,
        missing_bytes: missing_bytes(&app)?,
        total_bytes: pack().archive.size + engine().map_or(0, |(_, b)| build_bytes(b)),
        default_voice: pack().default_voice,
        voices: pack().voices.clone(),
    })
}

/// Downloads one archive, checks it, unpacks it into a staging folder and returns that folder.
async fn fetch_archive(
    url: String,
    file_name: &str,
    sha256: &str,
    downloads: &Path,
    staging: &Path,
    base: u64,
    total: u64,
    cancel: &AtomicBool,
    on_event: &Channel<DownloadEvent>,
) -> Result<bool, String> {
    let target = downloads.join(file_name);
    let fetch = Fetch {
        url,
        target: &target,
        sha256,
    };
    if !fetch_verified(&fetch, base, total, cancel, on_event).await? {
        return Ok(false);
    }
    let _ = std::fs::remove_dir_all(staging);
    let (archive, dest) = (target.clone(), staging.to_path_buf());
    let extracted =
        tokio::task::spawn_blocking(move || crate::llm::engines::extract(&archive, &dest))
            .await
            .map_err(|e| e.to_string())
            .and_then(|r| r);
    let _ = std::fs::remove_file(&target);
    extracted.map(|_| true)
}

#[tauri::command]
#[specta::specta]
pub async fn tts_install(
    app: AppHandle,
    state: State<'_, TtsState>,
    on_event: Channel<DownloadEvent>,
) -> Result<(), String> {
    let Some((source, build)) = engine() else {
        return Err("Read aloud with natural voices is not available on this system yet".into());
    };
    let cancel = Arc::new(AtomicBool::new(false));
    {
        let mut slot = state.cancel.lock().unwrap();
        if slot.is_some() {
            return Err("The voice pack is already downloading".into());
        }
        *slot = Some(cancel.clone());
    }

    let result = install(&app, source, build, &cancel, &on_event).await;
    *state.cancel.lock().unwrap() = None;
    match result {
        Ok(true) => {
            let _ = on_event.send(DownloadEvent::Done);
            Ok(())
        }
        Ok(false) => {
            let _ = on_event.send(DownloadEvent::Cancelled);
            Ok(())
        }
        Err(message) => {
            let _ = on_event.send(DownloadEvent::Error {
                message: message.clone(),
            });
            Err(message)
        }
    }
}

async fn install(
    app: &AppHandle,
    source: &EngineSource,
    build: &BuildSource,
    cancel: &AtomicBool,
    on_event: &Channel<DownloadEvent>,
) -> Result<bool, String> {
    let engines = crate::llm::paths::engines_dir(app)?;
    let downloads = engines.join(".downloads");
    std::fs::create_dir_all(&downloads).map_err(|e| e.to_string())?;
    let total = missing_bytes(app)?;
    let free = crate::llm::models::free_disk_bytes(&engines).unwrap_or(u64::MAX);
    if free < total * 2 {
        return Err(format!(
            "Not enough disk space: about {:.1} GB needed, {:.1} GB free",
            (total * 2) as f64 / 1e9,
            free as f64 / 1e9
        ));
    }

    let mut base = 0;
    if find_exe(&engine_dir(app)?, 3).is_none() {
        let staging = engines.join(".sherpa-tts.staging");
        let asset = build
            .assets
            .first()
            .ok_or("The voice engine has no download")?;
        let url = source.asset_url(asset);
        if !fetch_archive(
            url,
            asset.name,
            asset.sha256,
            &downloads,
            &staging,
            base,
            total,
            cancel,
            on_event,
        )
        .await?
        {
            let _ = std::fs::remove_dir_all(&staging);
            return Ok(false);
        }
        if find_exe(&staging, 3).is_none() {
            let _ = std::fs::remove_dir_all(&staging);
            return Err("The archive did not contain the voice engine".into());
        }
        let dest = engine_dir(app)?;
        let _ = std::fs::remove_dir_all(&dest);
        std::fs::rename(&staging, &dest).map_err(|e| e.to_string())?;
        base += build_bytes(build);
    }

    if !model_ready(&model_dir(app)?) {
        let staging = model_root(app)?.join(".kokoro.staging");
        let archive = &pack().archive;
        let name = archive.url.rsplit('/').next().unwrap_or("voices.tar.bz2");
        if !fetch_archive(
            archive.url.clone(),
            name,
            &archive.sha256,
            &downloads,
            &staging,
            base,
            total,
            cancel,
            on_event,
        )
        .await?
        {
            let _ = std::fs::remove_dir_all(&staging);
            return Ok(false);
        }
        let unpacked = staging.join(&pack().archive.folder);
        if !model_ready(&unpacked) {
            let _ = std::fs::remove_dir_all(&staging);
            return Err("The voice archive was missing files".into());
        }
        let dest = model_dir(app)?;
        let _ = std::fs::remove_dir_all(&dest);
        std::fs::rename(&unpacked, &dest).map_err(|e| e.to_string())?;
        let _ = std::fs::remove_dir_all(&staging);
    }
    Ok(true)
}

#[tauri::command]
#[specta::specta]
pub fn tts_install_cancel(state: State<'_, TtsState>) {
    if let Some(flag) = state.cancel.lock().unwrap().as_ref() {
        flag.store(true, Ordering::SeqCst);
    }
}

#[tauri::command]
#[specta::specta]
pub fn tts_remove(app: AppHandle) -> Result<(), String> {
    for dir in [engine_dir(&app)?, model_dir(&app)?, cache_dir(&app)?] {
        if dir.exists() {
            std::fs::remove_dir_all(dir).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn validate(req: &SpeakRequest) -> Result<(), String> {
    if req.text.trim().is_empty() {
        return Err("There is nothing to read".into());
    }
    if req.text.chars().count() > MAX_TEXT {
        return Err("That passage is too long to read in one go".into());
    }
    if !pack().voices.iter().any(|v| v.id == req.voice) {
        return Err("Unknown voice".into());
    }
    if !(0.5..=2.0).contains(&req.speed) {
        return Err("Speed must be between 0.5 and 2".into());
    }
    Ok(())
}

fn build_args(model: &Path, req: &SpeakRequest, threads: usize, out: &Path) -> Vec<String> {
    let files = &pack().files;
    let file = |name: &str| model.join(name).to_string_lossy().into_owned();
    // The tool reads options as --name=value; text that starts with dashes would look like one.
    let text = req.text.trim().trim_start_matches('-').trim().to_string();
    vec![
        format!("--kokoro-model={}", file(&files.model)),
        format!("--kokoro-voices={}", file(&files.voices)),
        format!("--kokoro-tokens={}", file(&files.tokens)),
        format!("--kokoro-lexicon={}", file(&files.lexicon)),
        format!("--num-threads={threads}"),
        format!("--sid={}", req.voice),
        // A larger length scale is slower speech, so speed is its inverse.
        format!("--kokoro-length-scale={:.3}", 1.0 / req.speed),
        format!("--output-filename={}", out.to_string_lossy()),
        text,
    ]
}

/// Playing time of a 16-bit PCM wav file, from its header.
pub fn wav_seconds(bytes: &[u8]) -> Option<f64> {
    if bytes.len() < 44 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return None;
    }
    let byte_rate = u32::from_le_bytes(bytes[28..32].try_into().ok()?) as f64;
    (byte_rate > 0.0).then(|| (bytes.len() - 44) as f64 / byte_rate)
}

/// Keeps the cache small: clips are played once, then only help a quick replay.
pub(crate) fn prune_cache(dir: &Path) {
    let Ok(read) = std::fs::read_dir(dir) else {
        return;
    };
    let mut files: Vec<_> = read
        .flatten()
        .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path())))
        .collect();
    files.sort_by_key(|(time, _)| std::cmp::Reverse(*time));
    for (_, path) in files.into_iter().skip(30) {
        let _ = std::fs::remove_file(path);
    }
}

#[tauri::command]
#[specta::specta]
pub async fn tts_synthesize(app: AppHandle, req: SpeakRequest) -> Result<SpeechClip, String> {
    validate(&req)?;
    let cli = find_exe(&engine_dir(&app)?, 3).ok_or("Install the voice pack first")?;
    let model = model_dir(&app)?;
    if !model_ready(&model) {
        return Err("Install the voice pack first".into());
    }
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
    let threads = std::thread::available_parallelism()
        .map(|n| (n.get() / 2).clamp(1, 4))
        .unwrap_or(2);

    let mut cmd = Command::new(&cli);
    cmd.args(build_args(&model, &req, threads, &out))
        .current_dir(cli.parent().unwrap_or(Path::new(".")))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    #[cfg(windows)]
    cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW

    let child = cmd
        .spawn()
        .map_err(|e| format!("Could not start the voice engine: {e}"))?;
    crate::job_object::bind(&child);
    let run = tokio::time::timeout(Duration::from_secs(120), child.wait_with_output())
        .await
        .map_err(|_| "The voice took too long".to_string())?
        .map_err(|e| format!("Could not start the voice engine: {e}"))?;
    if !run.status.success() || !out.exists() {
        let stderr = String::from_utf8_lossy(&run.stderr);
        let detail: Vec<&str> = stderr.lines().rev().take(3).collect();
        return Err(format!(
            "The voice engine stopped ({}).\n{}",
            run.status,
            detail.into_iter().rev().collect::<Vec<_>>().join("\n")
        ));
    }
    let seconds = std::fs::read(&out)
        .ok()
        .and_then(|b| wav_seconds(&b))
        .unwrap_or(0.0);
    Ok(SpeechClip {
        path: out.to_string_lossy().into_owned(),
        seconds,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(text: &str, voice: u32, speed: f32) -> SpeakRequest {
        SpeakRequest {
            text: text.into(),
            voice,
            speed,
        }
    }

    #[test]
    fn rejects_bad_requests() {
        assert!(validate(&req("Hello there", 3, 1.0)).is_ok());
        assert!(validate(&req("  ", 3, 1.0)).is_err());
        assert!(validate(&req("Hi", 99, 1.0)).is_err());
        assert!(validate(&req("Hi", 3, 5.0)).is_err());
    }

    #[test]
    fn leading_dashes_never_become_options() {
        let args = build_args(
            Path::new("/m"),
            &req("--sid=1 hello", 3, 1.0),
            2,
            Path::new("/o.wav"),
        );
        assert_eq!(args.last().unwrap(), "sid=1 hello");
        assert!(args.iter().any(|a| a == "--sid=3"));
    }

    #[test]
    fn speed_becomes_the_inverse_length_scale() {
        let args = build_args(Path::new("/m"), &req("Hi", 3, 2.0), 2, Path::new("/o.wav"));
        assert!(args.iter().any(|a| a == "--kokoro-length-scale=0.500"));
    }

    #[test]
    fn reads_the_length_of_a_wav() {
        let mut wav = vec![0u8; 44];
        wav[0..4].copy_from_slice(b"RIFF");
        wav[8..12].copy_from_slice(b"WAVE");
        wav[28..32].copy_from_slice(&48_000u32.to_le_bytes());
        wav.extend(vec![0u8; 96_000]);
        assert_eq!(wav_seconds(&wav), Some(2.0));
        assert_eq!(wav_seconds(b"nope"), None);
    }

    #[test]
    fn the_catalog_has_a_default_voice_that_exists() {
        let p = pack();
        assert!(p.voices.iter().any(|v| v.id == p.default_voice));
        assert!(p.voices.iter().any(|v| v.accent == "British"));
    }

    #[test]
    fn every_asset_has_a_real_checksum() {
        assert_eq!(pack().archive.sha256.len(), 64);
        if let Some((_, build)) = engine() {
            for asset in build.assets {
                assert_eq!(asset.sha256.len(), 64);
            }
        }
    }
}
