//! Model, engine and voice catalogs, kept as JSON files instead of code.
//!
//! Decision (architecture): what can be downloaded is data. The files in `src-tauri/catalog`
//! are bundled into the app, and a file with the same name in `<app data>/catalog/` adds
//! entries or replaces ones with the same `id`. A broken override is skipped with a warning and
//! never stops the app. Adding a model, a voice pack or an engine build is a file edit.

use crate::llm::engine::Backend;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::OnceLock;
use tauri::AppHandle;

const SCHEMA: u32 = 1;

pub const CHAT_MODELS: &str = include_str!("../catalog/chat-models.json");
pub const IMAGE_MODELS: &str = include_str!("../catalog/image-models.json");
pub const ENGINES: &str = include_str!("../catalog/engines.json");
pub const VOICES: &str = include_str!("../catalog/voices.json");

static USER_DIR: OnceLock<PathBuf> = OnceLock::new();

/// Remembers where user catalog files live. Until this runs only the bundled files are used.
pub fn init(app: &AppHandle) {
    if let Ok(dir) = crate::portable::app_data_dir(app) {
        let _ = USER_DIR.set(dir.join("catalog"));
    }
}

/// The text of the user's override file for a catalog, if there is one.
pub fn user_file(name: &str) -> Option<String> {
    let path = USER_DIR.get()?.join(format!("{name}.json"));
    std::fs::read_to_string(path).ok()
}

#[derive(Deserialize)]
struct Doc<T> {
    schema: u32,
    items: Vec<T>,
}

fn parse<T: DeserializeOwned>(text: &str) -> Result<Vec<T>, String> {
    let doc: Doc<T> = serde_json::from_str(text).map_err(|e| e.to_string())?;
    if doc.schema != SCHEMA {
        return Err(format!("unsupported schema {}", doc.schema));
    }
    Ok(doc.items)
}

/// Bundled items first, then the user's file: same `id` replaces, a new `id` is added.
pub fn load<T: DeserializeOwned>(name: &str, bundled: &str, id: fn(&T) -> &str) -> Vec<T> {
    let mut items =
        parse::<T>(bundled).unwrap_or_else(|e| panic!("bundled catalog {name} is invalid: {e}"));
    if let Some(dir) = USER_DIR.get() {
        let path = dir.join(format!("{name}.json"));
        if let Ok(text) = std::fs::read_to_string(&path) {
            match parse::<T>(&text) {
                Ok(extra) => {
                    for item in extra {
                        match items.iter().position(|i| id(i) == id(&item)) {
                            Some(at) => items[at] = item,
                            None => items.push(item),
                        }
                    }
                }
                Err(e) => log::warn!("Ignoring {}: {e}", path.display()),
            }
        }
    }
    items
}

/// Catalogs load once and live for the whole run, so their text is leaked on purpose.
pub fn leak_str(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

pub fn leak_slice<T>(v: Vec<T>) -> &'static [T] {
    Box::leak(v.into_boxed_slice())
}

/// `windows-x86_64`, `linux-x86_64`, `macos-aarch64`, and so on.
pub fn platform() -> String {
    format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
}

// --- Engines: official release assets, one list per platform. ---

#[derive(Clone, Copy)]
pub struct AssetSource {
    pub name: &'static str,
    pub sha256: &'static str,
    pub size: u64,
}

#[derive(Clone, Copy)]
pub struct BuildSource {
    pub backend: Backend,
    pub label: &'static str,
    pub recommended: bool,
    pub assets: &'static [AssetSource],
}

pub struct EngineSource {
    pub tag: &'static str,
    url: &'static str,
    /// The builds that exist for this operating system and CPU, best first.
    pub builds: &'static [BuildSource],
}

impl EngineSource {
    pub fn asset_url(&self, asset: &AssetSource) -> String {
        self.url.replace("{tag}", self.tag).replace("{asset}", asset.name)
    }
}

#[derive(Deserialize)]
struct RawAsset {
    name: String,
    sha256: String,
    size: u64,
}

#[derive(Deserialize)]
struct RawBuild {
    platform: String,
    backend: Backend,
    label: String,
    #[serde(default)]
    recommended: bool,
    assets: Vec<RawAsset>,
}

#[derive(Deserialize)]
struct RawEngine {
    id: String,
    tag: String,
    url: String,
    builds: Vec<RawBuild>,
}

fn build_engine(raw: RawEngine, platform: &str) -> (String, EngineSource) {
    let builds = raw
        .builds
        .into_iter()
        .filter(|b| b.platform == platform)
        .map(|b| BuildSource {
            backend: b.backend,
            label: leak_str(b.label),
            recommended: b.recommended,
            assets: leak_slice(
                b.assets
                    .into_iter()
                    .map(|a| AssetSource {
                        name: leak_str(a.name),
                        sha256: leak_str(a.sha256),
                        size: a.size,
                    })
                    .collect(),
            ),
        })
        .collect();
    (
        raw.id,
        EngineSource {
            tag: leak_str(raw.tag),
            url: leak_str(raw.url),
            builds: leak_slice(builds),
        },
    )
}

/// The named engine with only the builds for this machine, or None if it has no builds here.
pub fn engine(id: &str) -> Option<&'static EngineSource> {
    static CELL: OnceLock<Vec<(String, EngineSource)>> = OnceLock::new();
    let all = CELL.get_or_init(|| {
        let platform = platform();
        load::<RawEngine>("engines", ENGINES, |e| e.id.as_str())
            .into_iter()
            .map(|raw| build_engine(raw, &platform))
            .collect()
    });
    all.iter()
        .find(|(name, _)| name == id)
        .map(|(_, e)| e)
        .filter(|e| !e.builds.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_catalogs_parse_and_have_valid_hashes() {
        let engines = parse::<RawEngine>(ENGINES).unwrap();
        assert!(engines.iter().any(|e| e.id == "llama"));
        for engine in &engines {
            for build in &engine.builds {
                for asset in &build.assets {
                    assert_eq!(asset.sha256.len(), 64, "{}", asset.name);
                }
            }
        }
        for text in [CHAT_MODELS, IMAGE_MODELS, VOICES] {
            let doc: serde_json::Value = serde_json::from_str(text).unwrap();
            assert_eq!(doc["schema"], SCHEMA);
            assert!(!doc["items"].as_array().unwrap().is_empty());
        }
    }

    #[test]
    fn a_user_item_with_the_same_id_replaces_and_a_new_one_is_added() {
        #[derive(Deserialize, Debug)]
        struct Item {
            id: String,
            v: u32,
        }
        let mut items: Vec<Item> =
            parse(r#"{"schema":1,"items":[{"id":"a","v":1},{"id":"b","v":1}]}"#).unwrap();
        let extra: Vec<Item> =
            parse(r#"{"schema":1,"items":[{"id":"b","v":2},{"id":"c","v":3}]}"#).unwrap();
        for item in extra {
            match items.iter().position(|i| i.id == item.id) {
                Some(at) => items[at] = item,
                None => items.push(item),
            }
        }
        let got: Vec<_> = items.iter().map(|i| (i.id.as_str(), i.v)).collect();
        assert_eq!(got, vec![("a", 1), ("b", 2), ("c", 3)]);
    }

    #[test]
    fn rejects_an_unknown_schema() {
        assert!(parse::<serde_json::Value>(r#"{"schema":9,"items":[]}"#).is_err());
    }

    #[test]
    fn only_this_platforms_builds_are_kept() {
        let raw = parse::<RawEngine>(ENGINES).unwrap().remove(0);
        let (_, source) = build_engine(raw, "nowhere-x86_64");
        assert!(source.builds.is_empty());
    }
}
