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
    /// Rough total memory (RAM plus VRAM) for comfortable use, in GB.
    pub min_ram_gb: Option<u32>,
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
    #[serde(default)]
    min_ram_gb: Option<u32>,
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
                min_ram_gb: e.min_ram_gb,
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

/// Rough resident memory: weights + projector + KV cache + a fixed work reserve.
/// Provenance: Ensu `llm/memory.rs` memory_budget(); the 256 MiB reserve is its calibration.
/// KV cache is estimated at 128 KiB per context token, a conservative figure for ~2B models.
pub fn estimated_memory_bytes(entry: &CatalogEntry) -> u64 {
    const WORK_RESERVE: u64 = 256 * 1024 * 1024;
    const KV_PER_TOKEN: u64 = 128 * 1024;
    entry.total_bytes() + entry.context as u64 * KV_PER_TOKEN + WORK_RESERVE
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
        assert!(estimated_memory_bytes(e) > e.total_bytes());
    }
}
