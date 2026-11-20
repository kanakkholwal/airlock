use serde::{Deserialize, Serialize};
use specta::Type;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
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
    },
    Failed {
        reason: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct LlmStateChanged(pub EngineState);

struct Inner {
    child: Option<Child>,
    state: EngineState,
    // Bumped on every start/stop so a stale load task never overwrites newer state.
    generation: u64,
}

pub struct LlmEngine {
    inner: Mutex<Inner>,
}

impl Default for LlmEngine {
    fn default() -> Self {
        Self {
            inner: Mutex::new(Inner {
                child: None,
                state: EngineState::Stopped,
                generation: 0,
            }),
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

fn free_port() -> std::io::Result<u16> {
    Ok(std::net::TcpListener::bind("127.0.0.1:0")?
        .local_addr()?
        .port())
}

impl LlmEngine {
    pub async fn state(&self) -> EngineState {
        self.inner.lock().await.state.clone()
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

        let generation = {
            let mut inner = self.inner.lock().await;
            inner.generation += 1;
            if let Some(mut old) = inner.child.take() {
                let _ = old.kill().await;
            }
            inner.child = Some(cmd.spawn().map_err(|e| e.to_string())?);
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
                            },
                        )
                        .await;
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
