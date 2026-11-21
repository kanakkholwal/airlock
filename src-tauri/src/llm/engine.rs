use serde::{Deserialize, Serialize};
use specta::Type;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;
use std::time::Instant;
use tauri_specta::Event;
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

/// Which official llama.cpp build to run. Decision (architecture): Vulkan is the default on
/// Windows/Linux, CUDA is an opt-in download, CPU is the fallback; macOS builds use Metal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum Backend {
    Auto,
    Vulkan,
    Cuda,
    Metal,
    Cpu,
}

impl Backend {
    pub(super) fn dir_name(self) -> &'static str {
        match self {
            Backend::Auto => "llama-auto",
            Backend::Vulkan => "llama-vulkan",
            Backend::Cuda => "llama-cuda",
            Backend::Metal => "llama-metal",
            Backend::Cpu => "llama-cpu",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum EngineState {
    Stopped,
    Loading {
        model: String,
        backend: Backend,
    },
    Ready {
        model: String,
        backend: Backend,
        port: u16,
        /// Context window in tokens, the single source for the UI's context meter.
        context: u32,
    },
    Failed {
        reason: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct LlmStateChanged(pub EngineState);

struct Inner {
    child: Option<Child>,
    // Random per launch; requests without it are rejected by llama-server.
    api_key: Option<String>,
    state: EngineState,
    // Bumped on every start/stop so a stale load task never overwrites newer state.
    generation: u64,
}

/// When the chat model frees its memory. Decision (architecture): chat and dictation share one
/// small GPU, so the chat model does not stay resident forever. It loads on the first message
/// and unloads when idle, and sooner while the window is hidden (when dictation is what runs).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct Residency {
    /// Minutes without use before the model unloads; 0 keeps it loaded.
    pub idle_minutes: u32,
    pub unload_when_hidden: bool,
}

impl Default for Residency {
    fn default() -> Self {
        Self {
            idle_minutes: 5,
            unload_when_hidden: true,
        }
    }
}

/// How long a hidden window keeps the model, so a quick look away does not cost a reload.
const HIDDEN_GRACE: Duration = Duration::from_secs(30);

/// The idle time after which the model should unload, or None to keep it.
pub fn unload_after(policy: Residency, hidden: bool) -> Option<Duration> {
    let idle =
        (policy.idle_minutes > 0).then(|| Duration::from_secs(u64::from(policy.idle_minutes) * 60));
    let when_hidden = (hidden && policy.unload_when_hidden).then_some(HIDDEN_GRACE);
    match (idle, when_hidden) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (a, b) => a.or(b),
    }
}

pub struct LlmEngine {
    inner: Mutex<Inner>,
    policy: StdMutex<Residency>,
    hidden: AtomicBool,
    last_used: StdMutex<Instant>,
    /// Requests in flight; the model never unloads while this is above zero.
    active: AtomicUsize,
}

/// Marks the engine busy for its lifetime, and counts as use when it ends.
pub struct BusyGuard(Arc<LlmEngine>);

impl Drop for BusyGuard {
    fn drop(&mut self) {
        self.0.active.fetch_sub(1, Ordering::SeqCst);
        self.0.touch();
    }
}

impl Default for LlmEngine {
    fn default() -> Self {
        Self {
            inner: Mutex::new(Inner {
                child: None,
                api_key: None,
                state: EngineState::Stopped,
                generation: 0,
            }),
            policy: StdMutex::new(Residency::default()),
            hidden: AtomicBool::new(false),
            last_used: StdMutex::new(Instant::now()),
            active: AtomicUsize::new(0),
        }
    }
}

fn exe_name() -> &'static str {
    if cfg!(windows) {
        "llama-server.exe"
    } else {
        "llama-server"
    }
}

/// `AIRLOCK_LLAMA_DIR` overrides the lookup for local development.
fn find_server(engines_dir: &std::path::Path, backend: Backend) -> Option<(PathBuf, Backend)> {
    if let Some(dir) = std::env::var_os("AIRLOCK_LLAMA_DIR") {
        let exe = PathBuf::from(dir).join(exe_name());
        let resolved = if backend == Backend::Auto {
            Backend::Vulkan
        } else {
            backend
        };
        return exe.exists().then_some((exe, resolved));
    }
    let auto: &[Backend] = if cfg!(target_os = "macos") {
        &[Backend::Metal, Backend::Cpu]
    } else {
        &[Backend::Cuda, Backend::Vulkan, Backend::Cpu]
    };
    let one = [backend];
    let order: &[Backend] = if backend == Backend::Auto { auto } else { &one };
    order.iter().find_map(|b| {
        let exe = engines_dir.join(b.dir_name()).join(exe_name());
        exe.exists().then_some((exe, *b))
    })
}

/// Decision (security): the loopback server is protected by a per-launch key, so other local
/// programs and web pages cannot use the model or read prompts.
fn new_api_key() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn free_port() -> std::io::Result<u16> {
    Ok(std::net::TcpListener::bind("127.0.0.1:0")?
        .local_addr()?
        .port())
}

impl LlmEngine {
    pub async fn state(&self) -> EngineState {
        self.inner.lock().await.state.clone()
    }

    /// Base URL and key for OpenAI-compatible callers (dictation cleanup) while a model is ready.
    pub fn touch(&self) {
        *self.last_used.lock().unwrap() = Instant::now();
    }

    pub fn configure(&self, policy: Residency) {
        *self.policy.lock().unwrap() = policy;
    }

    pub fn set_hidden(&self, hidden: bool) {
        self.hidden.store(hidden, Ordering::SeqCst);
    }

    /// Hold while a request is running so the model cannot unload under it.
    pub fn is_busy(&self) -> bool {
        self.active.load(Ordering::SeqCst) > 0
    }

    pub fn begin(self: &Arc<Self>) -> BusyGuard {
        self.active.fetch_add(1, Ordering::SeqCst);
        self.touch();
        BusyGuard(Arc::clone(self))
    }

    pub async fn endpoint(&self) -> Option<(String, String)> {
        let inner = self.inner.lock().await;
        match (&inner.state, &inner.api_key) {
            (EngineState::Ready { port, .. }, Some(key)) => {
                // Dictation cleanup counts as use, so it keeps the model warm.
                self.touch();
                Some((format!("http://127.0.0.1:{port}/v1"), key.clone()))
            }
            _ => None,
        }
    }

    pub async fn api_key(&self) -> Option<String> {
        self.inner.lock().await.api_key.clone()
    }

    async fn set_state(&self, app: &tauri::AppHandle, generation: u64, state: EngineState) {
        let mut inner = self.inner.lock().await;
        if inner.generation != generation {
            return;
        }
        inner.state = state.clone();
        drop(inner);
        let _ = LlmStateChanged(state).emit(app);
    }

    pub async fn stop(&self, app: &tauri::AppHandle) {
        let mut inner = self.inner.lock().await;
        inner.generation += 1;
        if let Some(mut child) = inner.child.take() {
            let _ = child.kill().await;
        }
        inner.state = EngineState::Stopped;
        drop(inner);
        let _ = LlmStateChanged(EngineState::Stopped).emit(app);
    }

    /// Polls the engine process until it is replaced or stops, and reports an unexpected exit.
    async fn watch_after_ready(&self, app: tauri::AppHandle, generation: u64) {
        loop {
            tokio::time::sleep(Duration::from_millis(500)).await;
            let mut inner = self.inner.lock().await;
            if inner.generation != generation {
                return;
            }
            let exited = inner
                .child
                .as_mut()
                .and_then(|c| c.try_wait().ok().flatten());
            if let Some(status) = exited {
                drop(inner);
                self.set_state(
                    &app,
                    generation,
                    EngineState::Failed {
                        reason: format!("The model stopped unexpectedly ({status})."),
                    },
                )
                .await;
                return;
            }
            // Free the memory once it has been idle long enough, never under a request.
            if self.active.load(Ordering::SeqCst) == 0 {
                let policy = *self.policy.lock().unwrap();
                let hidden = self.hidden.load(Ordering::SeqCst);
                if let Some(limit) = unload_after(policy, hidden) {
                    if self.last_used.lock().unwrap().elapsed() > limit {
                        drop(inner);
                        self.stop(&app).await;
                        return;
                    }
                }
            }
        }
    }

    /// Spawns the sidecar and returns at once; readiness arrives as `LlmStateChanged`.
    pub async fn start(
        self: &Arc<Self>,
        app: tauri::AppHandle,
        engines_dir: PathBuf,
        model_id: String,
        model_path: PathBuf,
        projector: Option<PathBuf>,
        context: u32,
        backend: Backend,
    ) -> Result<(), String> {
        let (exe, backend) = find_server(&engines_dir, backend)
            .ok_or_else(|| "No llama-server engine is installed".to_string())?;
        if !model_path.exists() {
            return Err(format!("Model file not found: {}", model_path.display()));
        }
        let port = free_port().map_err(|e| e.to_string())?;
        let model_name = model_id;

        let api_key = new_api_key();
        let mut cmd = Command::new(&exe);
        cmd.arg("-m")
            .arg(&model_path)
            .args([
                "--host",
                "127.0.0.1",
                "--port",
                &port.to_string(),
                "-c",
                &context.to_string(),
            ])
            .args(["--api-key", &api_key])
            // Lets models that support it call tools, which the assistant relies on.
            .arg("--jinja")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        if let Some(p) = &projector {
            cmd.arg("--mmproj").arg(p);
        }
        if backend == Backend::Cpu {
            cmd.args(["--device", "none"]);
        } else {
            cmd.args(["-ngl", "99"]);
        }
        #[cfg(windows)]
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW

        self.touch();
        let generation = {
            let mut inner = self.inner.lock().await;
            inner.generation += 1;
            if let Some(mut old) = inner.child.take() {
                let _ = old.kill().await;
            }
            inner.child = Some(cmd.spawn().map_err(|e| e.to_string())?);
            inner.api_key = Some(api_key);
            inner.state = EngineState::Loading {
                model: model_name.clone(),
                backend,
            };
            inner.generation
        };
        let _ = LlmStateChanged(EngineState::Loading {
            model: model_name.clone(),
            backend,
        })
        .emit(&app);

        let this = self.clone();
        tokio::spawn(async move {
            let url = format!("http://127.0.0.1:{port}/health");
            let client = reqwest::Client::new();
            // Health reports not-ready while loading; no byte progress is available (plan section 14).
            for _ in 0..600 {
                tokio::time::sleep(Duration::from_millis(200)).await;
                {
                    let mut inner = this.inner.lock().await;
                    if inner.generation != generation {
                        return;
                    }
                    if let Some(child) = inner.child.as_mut() {
                        if let Ok(Some(status)) = child.try_wait() {
                            drop(inner);
                            this.set_state(
                                &app,
                                generation,
                                EngineState::Failed {
                                    reason: format!("Engine exited during load ({status})"),
                                },
                            )
                            .await;
                            return;
                        }
                    }
                }
                if let Ok(r) = client.get(&url).send().await {
                    if r.status().is_success() {
                        this.set_state(
                            &app,
                            generation,
                            EngineState::Ready {
                                model: model_name,
                                backend,
                                port,
                                context,
                            },
                        )
                        .await;
                        // Decision (code): keep watching after Ready. Without this a crashed
                        // engine would still read as Ready and every message would just fail.
                        this.watch_after_ready(app.clone(), generation).await;
                        return;
                    }
                }
            }
            this.set_state(
                &app,
                generation,
                EngineState::Failed {
                    reason: "Timed out loading the model".into(),
                },
            )
            .await;
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unload_limit_combines_idle_and_hidden() {
        let p = Residency {
            idle_minutes: 5,
            unload_when_hidden: true,
        };
        assert_eq!(unload_after(p, false), Some(Duration::from_secs(300)));
        assert_eq!(unload_after(p, true), Some(HIDDEN_GRACE));
        let never = Residency {
            idle_minutes: 0,
            unload_when_hidden: false,
        };
        assert_eq!(unload_after(never, true), None);
        let hidden_only = Residency {
            idle_minutes: 0,
            unload_when_hidden: true,
        };
        assert_eq!(unload_after(hidden_only, false), None);
        assert_eq!(unload_after(hidden_only, true), Some(HIDDEN_GRACE));
        let long = Residency {
            idle_minutes: 1,
            unload_when_hidden: true,
        };
        assert_eq!(unload_after(long, true), Some(HIDDEN_GRACE));
    }

    fn install(dir: &std::path::Path, backend: Backend) {
        let d = dir.join(backend.dir_name());
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join(exe_name()), b"").unwrap();
    }

    #[test]
    fn auto_prefers_cuda_then_vulkan_then_cpu() {
        std::env::remove_var("AIRLOCK_LLAMA_DIR");
        let tmp = tempfile::tempdir().unwrap();
        assert!(find_server(tmp.path(), Backend::Auto).is_none());
        install(tmp.path(), Backend::Cpu);
        assert_eq!(
            find_server(tmp.path(), Backend::Auto).unwrap().1,
            Backend::Cpu
        );
        install(tmp.path(), Backend::Vulkan);
        assert_eq!(
            find_server(tmp.path(), Backend::Auto).unwrap().1,
            Backend::Vulkan
        );
        install(tmp.path(), Backend::Cuda);
        assert_eq!(
            find_server(tmp.path(), Backend::Auto).unwrap().1,
            Backend::Cuda
        );
    }

    #[test]
    fn explicit_backend_does_not_fall_back() {
        std::env::remove_var("AIRLOCK_LLAMA_DIR");
        let tmp = tempfile::tempdir().unwrap();
        install(tmp.path(), Backend::Cpu);
        assert!(find_server(tmp.path(), Backend::Vulkan).is_none());
    }
}
