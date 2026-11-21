use serde::Serialize;
use sha2::{Digest, Sha256};
use specta::Type;
use std::io::Read;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tauri::ipc::Channel;
use tokio::io::AsyncWriteExt;

const PROGRESS_THROTTLE: Duration = Duration::from_millis(100);

#[derive(Debug, Clone, Serialize, Type)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DownloadEvent {
    Progress {
        downloaded: u64,
        total: u64,
        bytes_per_second: f64,
    },
    Verifying,
    Done,
    Cancelled,
    Error {
        message: String,
    },
}

/// One file to fetch: the partial lives beside the target as `<name>.partial`.
pub struct Fetch<'a> {
    pub url: String,
    pub target: &'a Path,
    pub sha256: &'a str,
}

pub fn partial_of(target: &Path) -> std::path::PathBuf {
    let mut name = target.file_name().unwrap_or_default().to_os_string();
    name.push(".partial");
    target.with_file_name(name)
}

pub fn sha256_file(path: &Path) -> std::io::Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}

/// Resumable download with checksum verification. Returns Ok(false) when cancelled.
/// Provenance: Handy managers/model/download.rs (resumable HTTP, stall timeout).
pub async fn fetch_verified(
    f: &Fetch<'_>,
    base: u64,
    total: u64,
    cancel: &AtomicBool,
    on_event: &Channel<DownloadEvent>,
) -> Result<bool, String> {
    let target = f.target;
    if target.exists() {
        return Ok(true);
    }
    let partial = partial_of(target);
    let mut have = std::fs::metadata(&partial).map(|m| m.len()).unwrap_or(0);

    let mut req = reqwest::Client::new().get(&f.url);
    if have > 0 {
        req = req.header("Range", format!("bytes={have}-"));
    }
    let mut res = req.send().await.map_err(|e| e.to_string())?;
    let status = res.status();
    if status == reqwest::StatusCode::RANGE_NOT_SATISFIABLE {
        // Partial is already the full size; fall through to verification.
    } else if !status.is_success() {
        return Err(format!("Download failed with HTTP {status}"));
    } else if have > 0 && status != reqwest::StatusCode::PARTIAL_CONTENT {
        have = 0; // Server ignored the range; start over.
    }

    if status != reqwest::StatusCode::RANGE_NOT_SATISFIABLE {
        let mut out = tokio::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .append(have > 0)
            .truncate(have == 0)
            .open(&partial)
            .await
            .map_err(|e| e.to_string())?;
        let started = Instant::now();
        let start_bytes = have;
        let mut last = Instant::now();
        while let Some(chunk) = tokio::time::timeout(Duration::from_secs(60), res.chunk())
            .await
            .map_err(|_| "Download stalled".to_string())?
            .map_err(|e| e.to_string())?
        {
            if cancel.load(Ordering::SeqCst) {
                out.flush().await.ok();
                return Ok(false);
            }
            out.write_all(&chunk).await.map_err(|e| e.to_string())?;
            have += chunk.len() as u64;
            if last.elapsed() >= PROGRESS_THROTTLE {
                last = Instant::now();
                let secs = started.elapsed().as_secs_f64().max(0.001);
                let _ = on_event.send(DownloadEvent::Progress {
                    downloaded: base + have,
                    total,
                    bytes_per_second: (have - start_bytes) as f64 / secs,
                });
            }
        }
        out.flush().await.map_err(|e| e.to_string())?;
    }

    let _ = on_event.send(DownloadEvent::Verifying);
    let hash_path = partial.clone();
    let digest = tokio::task::spawn_blocking(move || sha256_file(&hash_path))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    if digest != f.sha256 {
        let _ = std::fs::remove_file(&partial);
        return Err("Checksum mismatch; the partial file was removed, try again".into());
    }
    std::fs::rename(&partial, target).map_err(|e| e.to_string())?;
    Ok(true)
}
