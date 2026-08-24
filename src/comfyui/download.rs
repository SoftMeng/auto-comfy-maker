use std::path::{Path, PathBuf};

use chrono::Local;
use serde_json::Value;
use sha2::{Digest, Sha256};
use thiserror::Error;

use super::{ComfyuiClient, ComfyuiError};

#[derive(Debug, Error)]
pub enum DownloadError {
    #[error("comfyui: {0}")]
    Comfyui(#[from] ComfyuiError),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("no image in history result")]
    NoImage,
}

#[derive(Debug, Clone)]
pub struct ImageRef {
    pub filename: String,
    pub subfolder: String,
    pub kind: String,
}

pub fn extract_first_image(history: &Value, prompt_id: &str) -> Option<ImageRef> {
    let entry = history.get(prompt_id)?;
    let outputs = entry.get("outputs")?.as_object()?;
    for (_node_id, node_data) in outputs {
        if let Some(images) = node_data.get("images").and_then(|v| v.as_array()) {
            for img in images {
                let filename = img.get("filename")?.as_str()?.to_string();
                let subfolder = img
                    .get("subfolder")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let kind = img
                    .get("type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("output")
                    .to_string();
                return Some(ImageRef {
                    filename,
                    subfolder,
                    kind,
                });
            }
        }
    }
    None
}

pub async fn download_image(
    client: &ComfyuiClient,
    img: &ImageRef,
) -> Result<Vec<u8>, DownloadError> {
    let url = format!(
        "{}/view?filename={}&subfolder={}&type={}",
        client.base_url(),
        urlencoded(&img.filename),
        urlencoded(&img.subfolder),
        urlencoded(&img.kind),
    );
    let bytes = client
        .http()
        .get(&url)
        .send()
        .await
        .map_err(ComfyuiError::from)?
        .bytes()
        .await
        .map_err(ComfyuiError::from)?;
    Ok(bytes.to_vec())
}

pub fn build_output_path(
    output_root: &Path,
    prompt: &str,
    extension: &str,
) -> Result<PathBuf, DownloadError> {
    let today = Local::now().format("%Y-%m-%d").to_string();
    let dir = output_root.join(&today);
    std::fs::create_dir_all(&dir)?;

    let ts = Local::now().format("%Y%m%d-%H%M%S").to_string();
    let mut hasher = Sha256::new();
    hasher.update(prompt.as_bytes());
    let hash = format!("{:x}", hasher.finalize());
    let hash8 = &hash[..8];

    let ext = if extension.is_empty() {
        "png"
    } else {
        extension
    };
    Ok(dir.join(format!("{}_{}.{}", ts, hash8, ext)))
}

pub async fn download_and_save(
    client: &ComfyuiClient,
    history: &Value,
    prompt_id: &str,
    prompt: &str,
    output_root: &Path,
) -> Result<PathBuf, DownloadError> {
    let img = extract_first_image(history, prompt_id).ok_or(DownloadError::NoImage)?;
    let bytes = download_image(client, &img).await?;
    let ext = std::path::Path::new(&img.filename)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("png");
    let path = build_output_path(output_root, prompt, ext)?;
    std::fs::write(&path, &bytes)?;
    Ok(path)
}

fn urlencoded(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '~') {
            out.push(c);
        } else {
            for b in c.to_string().as_bytes() {
                out.push_str(&format!("%{:02X}", b));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn extract_first_image_basic() {
        let v = json!({
            "abc": {
                "outputs": {
                    "9": {
                        "images": [
                            { "filename": "ComfyUI_00001.png", "subfolder": "", "type": "output" }
                        ]
                    }
                }
            }
        });
        let img = extract_first_image(&v, "abc").unwrap();
        assert_eq!(img.filename, "ComfyUI_00001.png");
        assert_eq!(img.kind, "output");
    }

    #[test]
    fn extract_first_image_none_when_missing() {
        let v = json!({"abc": { "outputs": {} }});
        assert!(extract_first_image(&v, "abc").is_none());
    }

    #[test]
    fn build_output_path_includes_date_and_hash() {
        let root = tempfile::tempdir().unwrap();
        let p = build_output_path(root.path(), "hello world", "png").unwrap();
        let s = p.to_string_lossy();
        assert!(s.contains("20"));
        assert!(s.ends_with(".png"));
    }

    #[test]
    fn urlencoded_handles_unicode() {
        assert_eq!(urlencoded("abc"), "abc");
        assert_eq!(urlencoded("中文"), "%E4%B8%AD%E6%96%87");
        assert_eq!(urlencoded("a b"), "a%20b");
    }
}
