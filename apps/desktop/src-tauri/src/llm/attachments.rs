use serde::Serialize;
use sha2::{Digest, Sha256};
use specta::Type;
use std::path::{Path, PathBuf};
use tauri::AppHandle;

const MAX_IMAGE_BYTES: u64 = 20 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Type)]
pub struct AttachedImage {
    /// Absolute path of the stored copy inside the app data folder.
    pub path: String,
    pub name: String,
    pub size_bytes: u64,
}

fn attachments_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = crate::portable::app_data_dir(app)
        .map_err(|e| e.to_string())?
        .join("attachments");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

/// Decision (security): images are identified by their bytes, not their extension.
fn image_ext(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        Some("png")
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("jpg")
    } else if bytes.starts_with(b"GIF8") {
        Some("gif")
    } else if bytes.len() > 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some("webp")
    } else {
        None
    }
}

fn mime_for(ext: &str) -> &'static str {
    match ext {
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        _ => "image/jpeg",
    }
}

/// Copies a user-chosen image into the app's attachments folder under a content hash, so the
/// chat keeps working if the original moves and chat can only ever read files stored here.
#[tauri::command]
#[specta::specta]
pub async fn chat_attach_image(app: AppHandle, path: String) -> Result<AttachedImage, String> {
    let dir = attachments_dir(&app)?;
    tokio::task::spawn_blocking(move || {
        let src = PathBuf::from(&path);
        let meta = std::fs::metadata(&src).map_err(|e| format!("Cannot read the file: {e}"))?;
        if !meta.is_file() || meta.len() > MAX_IMAGE_BYTES {
            return Err("Images must be files under 20 MB".to_string());
        }
        let bytes = std::fs::read(&src).map_err(|e| e.to_string())?;
        let ext = image_ext(&bytes).ok_or("That file is not a PNG, JPEG, GIF or WebP image")?;
        let hash: String = Sha256::digest(&bytes)
            .iter()
            .take(12)
            .map(|b| format!("{b:02x}"))
            .collect();
        let stored = dir.join(format!("{hash}.{ext}"));
        if !stored.exists() {
            std::fs::write(&stored, &bytes).map_err(|e| e.to_string())?;
        }
        Ok(AttachedImage {
            path: stored.to_string_lossy().into_owned(),
            name: src
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "image".into()),
            size_bytes: meta.len(),
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Reads a stored attachment as a data URL for the model. Refuses any path outside the
/// attachments folder.
pub fn data_url(app: &AppHandle, path: &str) -> Result<String, String> {
    use base64::Engine;
    let dir = attachments_dir(app)?
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let file = Path::new(path)
        .canonicalize()
        .map_err(|e| format!("Attachment is missing: {e}"))?;
    if !file.starts_with(&dir) {
        return Err("Attachment is outside the attachments folder".into());
    }
    let bytes = std::fs::read(&file).map_err(|e| e.to_string())?;
    let ext = image_ext(&bytes).ok_or("Attachment is not a supported image")?;
    Ok(format!(
        "data:{};base64,{}",
        mime_for(ext),
        base64::engine::general_purpose::STANDARD.encode(bytes)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_images_by_content() {
        assert_eq!(image_ext(&[0x89, b'P', b'N', b'G', 0, 0]), Some("png"));
        assert_eq!(image_ext(&[0xFF, 0xD8, 0xFF, 0xE0]), Some("jpg"));
        assert_eq!(image_ext(b"GIF89a"), Some("gif"));
        assert_eq!(image_ext(b"RIFF\0\0\0\0WEBPVP8 "), Some("webp"));
    }

    #[test]
    fn rejects_other_files_even_with_an_image_name() {
        assert_eq!(image_ext(b"MZ\x90\0 not an image"), None);
        assert_eq!(image_ext(b"plain text"), None);
    }
}
