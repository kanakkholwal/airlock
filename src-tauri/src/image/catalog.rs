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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

const VAE: ImageFile = ImageFile {
    repo: "Comfy-Org/Z-Image-Turbo",
    path: "split_files/vae/ae.safetensors",
    name: "ae.safetensors",
    role: Role::Vae,
    size_bytes: 335_304_388,
    sha256: "afc8e28272cd15db3919bacdb6918ce9c1ed22e96cb12c4d5ed0fba823529e38",
};

/// Decision (product): weights never ship with the app. Sizes and hashes are the Hugging Face
/// LFS metadata (2026-11-21). Z-Image Turbo is the quality default for 4 GB GPUs per PLAN §10.
pub const MODELS: &[ImageModel] = &[
    ImageModel {
        id: "z-image-turbo-q4",
        name: "Z-Image Turbo",
        description: "Best quality that fits a small GPU. About 8 steps per image.",
        files: &[
            ImageFile {
                repo: "leejet/Z-Image-Turbo-GGUF",
                path: "z_image_turbo-Q4_0.gguf",
                name: "z_image_turbo-Q4_0.gguf",
                role: Role::Diffusion,
                size_bytes: 3_683_370_944,
                sha256: "2bc57986874c84f7ec6d02d9d7070a53b0029954a0e38a6e1342eb91095572f5",
            },
            ImageFile {
                repo: "unsloth/Qwen3-4B-Instruct-2507-GGUF",
                path: "Qwen3-4B-Instruct-2507-Q4_K_M.gguf",
                name: "Qwen3-4B-Instruct-2507-Q4_K_M.gguf",
                role: Role::TextEncoder,
                size_bytes: 2_497_281_120,
                sha256: "3605803b982cb64aead44f6c1b2ae36e3acdb41d8e46c8a94c6533bc4c67e597",
            },
            VAE,
        ],
        steps: 8,
        cfg_scale: 1.0,
        license: "Apache-2.0",
        recommended: true,
    },
    ImageModel {
        id: "z-image-turbo-q3",
        name: "Z-Image Turbo (small)",
        description: "Smaller download and less memory, slightly softer detail.",
        files: &[
            ImageFile {
                repo: "leejet/Z-Image-Turbo-GGUF",
                path: "z_image_turbo-Q3_K.gguf",
                name: "z_image_turbo-Q3_K.gguf",
                role: Role::Diffusion,
                size_bytes: 3_143_559_104,
                sha256: "4b44bdaa7814f20d7cf144e3939bd93aa32f50660204dd0c2aea5c5376232980",
            },
            ImageFile {
                repo: "unsloth/Qwen3-4B-Instruct-2507-GGUF",
                path: "Qwen3-4B-Instruct-2507-Q3_K_M.gguf",
                name: "Qwen3-4B-Instruct-2507-Q3_K_M.gguf",
                role: Role::TextEncoder,
                size_bytes: 2_075_618_400,
                sha256: "9c6e0763577125a994a9bea0bbd7a737ac4498b8a6a4e0f788727553af1806c9",
            },
            VAE,
        ],
        steps: 8,
        cfg_scale: 1.0,
        license: "Apache-2.0",
        recommended: false,
    },
];

pub fn find(id: &str) -> Option<&'static ImageModel> {
    MODELS.iter().find(|m| m.id == id)
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
        for m in MODELS {
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
        for a in MODELS.iter().flat_map(|m| m.files) {
            for b in MODELS.iter().flat_map(|m| m.files) {
                if a.name == b.name {
                    assert_eq!(a.sha256, b.sha256);
                }
            }
        }
    }
}
