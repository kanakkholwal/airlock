use super::download::{fetch_verified, DownloadEvent, Fetch};
use super::models::{free_disk_bytes, models_dir, LlmModels};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;
use tauri::ipc::Channel;
use tauri::{AppHandle, State};

const API: &str = "https://huggingface.co/api/models";

/// What `llm_hf_search` returns for each repo. `architecture` comes from the GGUF header Hugging
/// Face indexes, which is what decides whether the engine can run it.
#[derive(Debug, Clone, Serialize, Type)]
pub struct HfModel {
    pub id: String,
    pub downloads: u64,
    pub likes: u64,
    pub last_modified: Option<String>,
    pub license: Option<String>,
    pub architecture: Option<String>,
    pub parameters: Option<u64>,
    pub context_length: Option<u64>,
    pub vision: bool,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct HfFile {
    pub name: String,
    pub size_bytes: u64,
    /// True for the vision projector that goes with a vision model.
    pub projector: bool,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct HfRepo {
    pub model: HfModel,
    pub files: Vec<HfFile>,
}

#[derive(Deserialize)]
struct RawGguf {
    architecture: Option<String>,
    total: Option<u64>,
    context_length: Option<u64>,
}

#[derive(Deserialize)]
struct RawLfs {
    sha256: Option<String>,
}

#[derive(Deserialize)]
struct RawSibling {
    rfilename: String,
    size: Option<u64>,
    lfs: Option<RawLfs>,
}

#[derive(Deserialize)]
struct RawModel {
    id: String,
    #[serde(default)]
    downloads: u64,
    #[serde(default)]
    likes: u64,
    #[serde(default)]
    tags: Vec<String>,
    pipeline_tag: Option<String>,
    #[serde(rename = "lastModified")]
    last_modified: Option<String>,
    gguf: Option<RawGguf>,
    #[serde(default)]
    siblings: Vec<RawSibling>,
}

impl RawModel {
    fn into_model(self) -> (HfModel, Vec<RawSibling>) {
        let vision = self.pipeline_tag.as_deref() == Some("image-text-to-text")
            || self.tags.iter().any(|t| t == "image-text-to-text");
        let model = HfModel {
            license: license_from_tags(&self.tags),
            vision,
            id: self.id,
            downloads: self.downloads,
            likes: self.likes,
            last_modified: self.last_modified,
            architecture: self.gguf.as_ref().and_then(|g| g.architecture.clone()),
            parameters: self.gguf.as_ref().and_then(|g| g.total),
            context_length: self.gguf.as_ref().and_then(|g| g.context_length),
        };
        (model, self.siblings)
    }
}

fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent("Airlock/0.1")
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|e| e.to_string())
}

fn license_from_tags(tags: &[String]) -> Option<String> {
    tags.iter()
        .find_map(|t| t.strip_prefix("license:"))
        .map(str::to_string)
}

/// Decision (security): repo and file names are interpolated into URLs and file paths, so they
/// are checked strictly instead of trusting whatever the webview sends.
fn valid_repo(repo: &str) -> bool {
    let part = |s: &str| {
        !s.is_empty()
            && s.len() <= 96
            && !s.starts_with('.')
            && s.chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
    };
    matches!(repo.split_once('/'), Some((a, b)) if part(a) && part(b) && !b.contains('/'))
}

/// A single-file GGUF at the repo root. Sharded and nested files are not supported yet.
fn valid_gguf_name(name: &str) -> bool {
    name.len() <= 200
        && name.ends_with(".gguf")
        && !name.contains("-of-0")
        && !name.starts_with('.')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

fn whitelist<'a>(value: &str, allowed: &[&'a str], default: &'a str) -> &'a str {
    allowed
        .iter()
        .copied()
        .find(|a| *a == value)
        .unwrap_or(default)
}

/// Searches GGUF models. `task` is `chat` or `vision`; `sort` is one of the Hub's sort keys.
#[tauri::command]
#[specta::specta]
pub async fn llm_hf_search(
    query: String,
    task: String,
    sort: String,
) -> Result<Vec<HfModel>, String> {
    let pipeline = if task == "vision" {
        "image-text-to-text"
    } else {
        "text-generation"
    };
    let sort = whitelist(
        &sort,
        &["trendingScore", "downloads", "likes", "lastModified"],
        "trendingScore",
    );
    let mut params: Vec<(&str, String)> = vec![
        ("search", query.chars().take(80).collect()),
        ("filter", "gguf".into()),
        ("pipeline_tag", pipeline.into()),
        ("sort", sort.into()),
        ("direction", "-1".into()),
        ("limit", "24".into()),
    ];
    for field in [
        "downloads",
        "likes",
        "tags",
        "pipeline_tag",
        "lastModified",
        "gguf",
    ] {
        params.push(("expand[]", field.into()));
    }
    let raw: Vec<RawModel> = client()?
        .get(API)
        .query(&params)
        .send()
        .await
        .map_err(|e| format!("Could not reach huggingface.co: {e}"))?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    Ok(raw.into_iter().map(|m| m.into_model().0).collect())
}

async fn fetch_repo(repo: &str) -> Result<(HfModel, Vec<(HfFile, Option<String>)>), String> {
    if !valid_repo(repo) {
        return Err("That is not a valid model name".into());
    }
    let raw: RawModel = client()?
        .get(format!("{API}/{repo}"))
        .query(&[("blobs", "true")])
        .send()
        .await
        .map_err(|e| format!("Could not reach huggingface.co: {e}"))?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    let (model, siblings) = raw.into_model();
    let mut files: Vec<_> = siblings
        .into_iter()
        .filter(|s| valid_gguf_name(&s.rfilename))
        .map(|s| {
            (
                HfFile {
                    projector: s.rfilename.to_lowercase().starts_with("mmproj"),
                    size_bytes: s.size.unwrap_or(0),
                    name: s.rfilename,
                },
                s.lfs.and_then(|l| l.sha256),
            )
        })
        .collect();
    files.sort_by_key(|(f, _)| f.size_bytes);
    Ok((model, files))
}

#[tauri::command]
#[specta::specta]
pub async fn llm_hf_files(repo: String) -> Result<HfRepo, String> {
    let (model, files) = fetch_repo(&repo).await?;
    Ok(HfRepo {
        model,
        files: files.into_iter().map(|(f, _)| f).collect(),
    })
}

/// Downloads one GGUF (and optionally its vision projector) into the imported-models folder,
/// verified against the checksums Hugging Face publishes, and records where it came from.
#[tauri::command]
#[specta::specta]
pub async fn llm_hf_download(
    app: AppHandle,
    models: State<'_, LlmModels>,
    repo: String,
    filename: String,
    projector: Option<String>,
    on_event: Channel<DownloadEvent>,
) -> Result<(), String> {
    let wanted: Vec<String> = std::iter::once(filename.clone())
        .chain(projector.clone())
        .collect();
    if wanted.iter().any(|n| !valid_gguf_name(n)) {
        return Err("That file cannot be downloaded".into());
    }
    let (model, files) = fetch_repo(&repo).await?;
    let mut jobs = Vec::new();
    for name in &wanted {
        let (file, sha) = files
            .iter()
            .find(|(f, _)| &f.name == name)
            .cloned()
            .ok_or("That file is not in the repository")?;
        let sha = sha.ok_or("Hugging Face published no checksum for this file")?;
        jobs.push((file, sha));
    }
    let total: u64 = jobs.iter().map(|(f, _)| f.size_bytes).sum();

    let dir = models_dir(&app)?.join("imported");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    if let Some(free) = free_disk_bytes(&dir) {
        if free < total {
            let message = format!(
                "Needs {} MB of disk, {} MB free",
                total / 1_048_576,
                free / 1_048_576
            );
            let _ = on_event.send(DownloadEvent::Error {
                message: message.clone(),
            });
            return Err(message);
        }
    }

    let key = format!("hf:{repo}/{filename}");
    let cancel = Arc::new(AtomicBool::new(false));
    models
        .cancels
        .lock()
        .unwrap()
        .insert(key.clone(), cancel.clone());

    let mut base = 0;
    let mut outcome: Result<bool, String> = Ok(true);
    for (file, sha256) in &jobs {
        let target = dir.join(&file.name);
        let fetch = Fetch {
            url: format!("https://huggingface.co/{repo}/resolve/main/{}", file.name),
            target: &target,
            sha256,
        };
        match fetch_verified(&fetch, base, total, &cancel, &on_event).await {
            Ok(true) => base += file.size_bytes,
            other => {
                outcome = other;
                break;
            }
        }
    }
    models.cancels.lock().unwrap().remove(&key);

    match outcome {
        Ok(true) => {
            let note = serde_json::json!({
                "repo": repo,
                "license": model.license,
                "projector": projector,
            });
            let _ = std::fs::write(dir.join(format!("{filename}.json")), note.to_string());
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repo_names_are_checked_strictly() {
        assert!(valid_repo("LiquidAI/LFM3-1.7B-GGUF"));
        assert!(valid_repo("a/b.c_d-e"));
        for bad in [
            "", "noslash", "a/b/c", "../x", "a/..", "a/ b", "a/b?x=1", "/a", "a/", ".a/b",
        ] {
            assert!(!valid_repo(bad), "{bad}");
        }
    }

    #[test]
    fn gguf_names_must_be_plain_single_files() {
        assert!(valid_gguf_name("model-Q4_K_M.gguf"));
        for bad in [
            "a/b.gguf",
            "..gguf",
            "x.bin",
            "m-00001-of-00003.gguf",
            "a b.gguf",
            ".hidden.gguf",
        ] {
            assert!(!valid_gguf_name(bad), "{bad}");
        }
    }

    #[test]
    fn license_comes_from_the_tag_list() {
        let tags = vec!["gguf".to_string(), "license:apache-2.0".to_string()];
        assert_eq!(license_from_tags(&tags).as_deref(), Some("apache-2.0"));
        assert_eq!(license_from_tags(&["gguf".to_string()]), None);
    }

    #[test]
    fn unknown_sort_keys_fall_back() {
        assert_eq!(whitelist("evil", &["downloads"], "downloads"), "downloads");
        assert_eq!(
            whitelist("likes", &["downloads", "likes"], "downloads"),
            "likes"
        );
    }
}
