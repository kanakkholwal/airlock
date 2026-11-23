use super::arch::ModelArch;
use crate::data::{leak_slice, leak_str, load, CHAT_MODELS};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, Serialize, Type)]
pub struct CatalogFile {
    pub repo: &'static str,
    pub filename: &'static str,
    pub size_bytes: u64,
    pub sha256: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub struct CatalogEntry {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub model: CatalogFile,
    /// Vision projector, downloaded together with the model.
    pub projector: Option<CatalogFile>,
    pub context: u32,
    /// Largest context the model supports, shown in the picker.
    pub max_context: Option<u32>,
    /// Chat-cache size per token at 16-bit, counting only full-attention layers
    /// (2 x kv_heads x head_dim x 2 bytes each), from the model's config.json.
    pub kv_kib_per_token: u32,
    /// Fixed cache for sliding-window layers (window x layer size), in MiB.
    pub kv_fixed_mib: u32,
    pub license: &'static str,
    pub license_url: &'static str,
    pub recommended: bool,
    /// Short labels such as `vision` or `uncensored`, shown as badges.
    pub tags: &'static [&'static str],
    /// 1 to 5 ratings shown in the model picker; absent when not rated.
    pub speed: Option<u8>,
    pub intelligence: Option<u8>,
}

#[derive(Deserialize)]
struct RawFile {
    repo: String,
    filename: String,
    size_bytes: u64,
    sha256: String,
}

#[derive(Deserialize)]
struct RawEntry {
    id: String,
    name: String,
    description: String,
    model: RawFile,
    #[serde(default)]
    projector: Option<RawFile>,
    context: u32,
    #[serde(default)]
    max_context: Option<u32>,
    #[serde(default = "default_kv_kib")]
    kv_kib_per_token: u32,
    #[serde(default)]
    kv_fixed_mib: u32,
    license: String,
    license_url: String,
    #[serde(default)]
    recommended: bool,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    speed: Option<u8>,
    #[serde(default)]
    intelligence: Option<u8>,
}

/// For catalog entries that omit it: the old flat guess, safe for small models.
fn default_kv_kib() -> u32 {
    128
}

fn file(raw: RawFile) -> CatalogFile {
    CatalogFile {
        repo: leak_str(raw.repo),
        filename: leak_str(raw.filename),
        size_bytes: raw.size_bytes,
        sha256: leak_str(raw.sha256),
    }
}

/// Decision (product): weights are never bundled; they download on the user's action. The list
/// lives in `catalog/chat-models.json` (sizes and hashes from the Hugging Face LFS metadata).
pub fn all() -> &'static [CatalogEntry] {
    static CELL: OnceLock<Vec<CatalogEntry>> = OnceLock::new();
    CELL.get_or_init(|| {
        load::<RawEntry>("chat-models", CHAT_MODELS, |e| e.id.as_str())
            .into_iter()
            .map(|e| CatalogEntry {
                id: leak_str(e.id),
                name: leak_str(e.name),
                description: leak_str(e.description),
                model: file(e.model),
                projector: e.projector.map(file),
                context: e.context,
                max_context: e.max_context,
                kv_kib_per_token: e.kv_kib_per_token,
                kv_fixed_mib: e.kv_fixed_mib,
                license: leak_str(e.license),
                license_url: leak_str(e.license_url),
                recommended: e.recommended,
                tags: leak_slice(e.tags.into_iter().map(leak_str).collect()),
                speed: e.speed.map(|v| v.min(5)),
                intelligence: e.intelligence.map(|v| v.min(5)),
            })
            .collect()
    })
}

pub fn find(id: &str) -> Option<&'static CatalogEntry> {
    all().iter().find(|e| e.id == id)
}

impl CatalogEntry {
    pub fn total_bytes(&self) -> u64 {
        self.model.size_bytes + self.projector.map_or(0, |p| p.size_bytes)
    }

    pub fn files(&self) -> impl Iterator<Item = &CatalogFile> {
        std::iter::once(&self.model).chain(self.projector.iter())
    }
}

impl CatalogEntry {
    pub fn arch(&self) -> ModelArch {
        ModelArch {
            max_context: self.max_context.unwrap_or(self.context),
            kv_kib_per_token: self.kv_kib_per_token,
            kv_fixed_mib: self.kv_fixed_mib,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_and_hashes_look_right() {
        let all = all();
        assert!(!all.is_empty());
        for (i, e) in all.iter().enumerate() {
            assert!(all[i + 1..].iter().all(|o| o.id != e.id));
            for f in e.files() {
                assert_eq!(f.sha256.len(), 64);
                assert!(f.sha256.chars().all(|c| c.is_ascii_hexdigit()));
            }
        }
    }

    #[test]
    fn memory_estimate_exceeds_weights() {
        let e = find("lfm3-1.7b-q4").unwrap();
        let t = super::super::engine::Tuning::default();
        let a = e.arch();
        assert!(a.memory_bytes(e.total_bytes(), 4096, &t) > e.total_bytes());
        let long = a.memory_bytes(e.total_bytes(), 131072, &t);
        assert!(long > a.memory_bytes(e.total_bytes(), 4096, &t));
        let q8 = super::super::engine::Tuning { kv_q8: true, ..t };
        assert!(a.memory_bytes(e.total_bytes(), 131072, &q8) < long);
    }
}
