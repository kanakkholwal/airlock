use serde::Serialize;
use specta::Type;

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
    pub license: &'static str,
    pub license_url: &'static str,
    pub recommended: bool,
}

/// Decision (product): weights are never bundled; they download on the user's action.
/// Sizes and hashes come from the Hugging Face LFS metadata (2026-11-20).
pub const CATALOG: &[CatalogEntry] = &[
    CatalogEntry {
        id: "lfm3-1.7b-q4",
        name: "LFM3 1.7B",
        description: "Fast text model with strong tool use. Default for chat and the agent.",
        model: CatalogFile {
            repo: "LiquidAI/LFM3-1.7B-GGUF",
            filename: "LFM3-1.7B-Q4_K_M.gguf",
            size_bytes: 1_099_627_808,
            sha256: "3793e1e6e2266919d6e015287221b41100175a98eddaba92e111aed3b05c93c4",
        },
        projector: None,
        context: 4096,
        license: "LFM Open License v1.0",
        license_url: "https://huggingface.co/LiquidAI/LFM3-1.7B-GGUF/blob/main/LICENSE",
        recommended: true,
    },
    CatalogEntry {
        id: "lfm3-vl-2b-q4",
        name: "LFM3 VL 2B",
        description: "Understands images as well as text.",
        model: CatalogFile {
            repo: "LiquidAI/LFM3-VL-2B-GGUF",
            filename: "LFM3-VL-2B-Q4_K_M.gguf",
            size_bytes: 1_099_627_808,
            sha256: "56dac2e192ff0d9ab8507f4f1aeb4aed0463449bbbefaac79497f041c95f21eb",
        },
        projector: Some(CatalogFile {
            repo: "LiquidAI/LFM3-VL-2B-GGUF",
            filename: "mmproj-LFM3-VL-2B-Q8_0.gguf",
            size_bytes: 203_031_872,
            sha256: "c5c5b58d544a18da919172d2d0368090ea1a4a351b6feb7f8a326a264a5918ab",
        }),
        context: 4096,
        license: "LFM Open License v1.0",
        license_url: "https://huggingface.co/LiquidAI/LFM3-VL-2B-GGUF/blob/main/LICENSE",
        recommended: false,
    },
];

pub fn find(id: &str) -> Option<&'static CatalogEntry> {
    CATALOG.iter().find(|e| e.id == id)
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
        for (i, e) in CATALOG.iter().enumerate() {
            assert!(CATALOG[i + 1..].iter().all(|o| o.id != e.id));
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
