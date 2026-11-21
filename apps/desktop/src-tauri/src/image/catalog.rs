use crate::data::{leak_slice, leak_str, load, IMAGE_MODELS};
use serde::Deserialize;
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy)]
pub struct ImageFile {
    pub repo: &'static str,
    /// Path inside the repository.
    pub path: &'static str,
    /// File name on disk; shared parts keep one name so they download once.
    pub name: &'static str,
    pub role: Role,
    pub size_bytes: u64,
    pub sha256: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Diffusion,
    TextEncoder,
    Vae,
}

#[derive(Debug, Clone, Copy)]
pub struct ImageModel {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub files: &'static [ImageFile],
    pub steps: u32,
    pub cfg_scale: f32,
    pub license: &'static str,
    pub recommended: bool,
}

#[derive(Deserialize)]
struct RawFile {
    repo: String,
    path: String,
    name: String,
    role: Role,
    size_bytes: u64,
    sha256: String,
}

#[derive(Deserialize)]
struct RawModel {
    id: String,
    name: String,
    description: String,
    files: Vec<RawFile>,
    steps: u32,
    cfg_scale: f32,
    license: String,
    #[serde(default)]
    recommended: bool,
}

/// Decision (product): weights never ship with the app. The list lives in
/// `catalog/image-models.json`; Z-Image Turbo is the quality default for 4 GB GPUs (PLAN 10).
pub fn models() -> &'static [ImageModel] {
    static CELL: OnceLock<Vec<ImageModel>> = OnceLock::new();
    CELL.get_or_init(|| {
        load::<RawModel>("image-models", IMAGE_MODELS, |m| m.id.as_str())
            .into_iter()
            .map(|m| ImageModel {
                id: leak_str(m.id),
                name: leak_str(m.name),
                description: leak_str(m.description),
                files: leak_slice(
                    m.files
                        .into_iter()
                        .map(|f| ImageFile {
                            repo: leak_str(f.repo),
                            path: leak_str(f.path),
                            name: leak_str(f.name),
                            role: f.role,
                            size_bytes: f.size_bytes,
                            sha256: leak_str(f.sha256),
                        })
                        .collect(),
                ),
                steps: m.steps,
                cfg_scale: m.cfg_scale,
                license: leak_str(m.license),
                recommended: m.recommended,
            })
            .collect()
    })
}

pub fn find(id: &str) -> Option<&'static ImageModel> {
    models().iter().find(|m| m.id == id)
}

impl ImageModel {
    pub fn total_bytes(&self) -> u64 {
        self.files.iter().map(|f| f.size_bytes).sum()
    }

    pub fn file(&self, role: Role) -> Option<&ImageFile> {
        self.files.iter().find(|f| f.role == role)
    }
}

pub fn file_url(f: &ImageFile) -> String {
    format!("https://huggingface.co/{}/resolve/main/{}", f.repo, f.path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_model_has_all_three_parts_and_real_hashes() {
        for m in models() {
            for role in [Role::Diffusion, Role::TextEncoder, Role::Vae] {
                assert!(m.file(role).is_some(), "{} misses {role:?}", m.id);
            }
            for f in m.files {
                assert_eq!(f.sha256.len(), 64, "{}", f.name);
            }
        }
    }

    #[test]
    fn same_file_name_means_same_file() {
        for a in models().iter().flat_map(|m| m.files) {
            for b in models().iter().flat_map(|m| m.files) {
                if a.name == b.name {
                    assert_eq!(a.sha256, b.sha256);
                }
            }
        }
    }
}
