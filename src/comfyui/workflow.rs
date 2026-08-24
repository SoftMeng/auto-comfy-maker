use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ReplaceError {
    #[error("json pointer not found: {0}")]
    PathNotFound(String),
    #[error("manifest not found: {0}")]
    ManifestMissing(String),
    #[error("manifest parse: {0}")]
    ManifestParse(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub templates: std::collections::HashMap<String, ManifestEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestEntry {
    pub positive_prompt_node: String,
    pub positive_prompt_field: String,
    #[serde(default)]
    pub negative_prompt_node: Option<String>,
    #[serde(default)]
    pub negative_prompt_field: Option<String>,
    #[serde(default)]
    pub seed_node: Option<String>,
    #[serde(default)]
    pub seed_field: Option<String>,
    #[serde(default)]
    pub width_node: Option<String>,
    #[serde(default)]
    pub width_field: Option<String>,
    #[serde(default)]
    pub height_node: Option<String>,
    #[serde(default)]
    pub height_field: Option<String>,
}

impl Manifest {
    pub fn load(path: &Path) -> Result<Self, ReplaceError> {
        if !path.exists() {
            return Err(ReplaceError::ManifestMissing(path.display().to_string()));
        }
        let text = std::fs::read_to_string(path)
            .map_err(|e| ReplaceError::ManifestParse(e.to_string()))?;
        toml::from_str(&text).map_err(|e| ReplaceError::ManifestParse(e.to_string()))
    }

    pub fn get(&self, template: &str) -> Option<&ManifestEntry> {
        self.templates.get(template)
    }
}

pub struct WorkflowReplacer<'a> {
    value: &'a mut Value,
}

impl<'a> WorkflowReplacer<'a> {
    pub fn new(value: &'a mut Value) -> Self {
        Self { value }
    }

    pub fn replace_text(
        &mut self,
        node_id: &str,
        field_path: &str,
        text: &str,
    ) -> Result<(), ReplaceError> {
        let pointer = build_pointer(node_id, field_path);
        let target = self
            .value
            .pointer_mut(&pointer)
            .ok_or_else(|| ReplaceError::PathNotFound(pointer.clone()))?;
        *target = Value::String(text.to_string());
        Ok(())
    }

    pub fn replace_int(
        &mut self,
        node_id: &str,
        field_path: &str,
        value: i64,
    ) -> Result<(), ReplaceError> {
        let pointer = build_pointer(node_id, field_path);
        let target = self
            .value
            .pointer_mut(&pointer)
            .ok_or_else(|| ReplaceError::PathNotFound(pointer.clone()))?;
        *target = Value::Number(value.into());
        Ok(())
    }
}

fn build_pointer(node_id: &str, field_path: &str) -> String {
    let trimmed = field_path.trim_start_matches('/');
    let parts: Vec<&str> = if trimmed.is_empty() {
        vec![]
    } else {
        trimmed.split('.').collect()
    };
    if parts.is_empty() {
        format!("/{}", node_id)
    } else {
        format!("/{}/{}", node_id, parts.join("/"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample_workflow() -> Value {
        json!({
            "6": {
                "class_type": "CLIPTextEncode",
                "inputs": { "text": "placeholder" }
            },
            "3": {
                "class_type": "KSampler",
                "inputs": { "seed": 0, "steps": 20 }
            }
        })
    }

    #[test]
    fn replace_text_by_node_and_field() {
        let mut wf = sample_workflow();
        let mut r = WorkflowReplacer::new(&mut wf);
        r.replace_text("6", "inputs.text", "new prompt").unwrap();
        assert_eq!(wf["6"]["inputs"]["text"], "new prompt");
    }

    #[test]
    fn replace_text_special_chars_roundtrip() {
        // 含换行/双引号/反斜杠/制表符的 prompt：replace 后经 serde_json
        // round-trip 必须保留原字符，不能让 ComfyUI 解析失败
        let tricky = "line1\nline2\twith tab\n\"quoted\"\nand \\backslash";
        let mut wf = sample_workflow();
        let mut r = WorkflowReplacer::new(&mut wf);
        r.replace_text("6", "inputs.text", tricky).unwrap();

        let serialized = serde_json::to_string(&wf).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&serialized).unwrap();
        assert_eq!(parsed["6"]["inputs"]["text"].as_str().unwrap(), tricky);
    }

    #[test]
    fn replace_text_unicode_roundtrip() {
        // 中文 / emoji / 零宽字符也需安全
        let tricky = "中文 prompt 🚀\u{200B}zwsp";
        let mut wf = sample_workflow();
        let mut r = WorkflowReplacer::new(&mut wf);
        r.replace_text("6", "inputs.text", tricky).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&serde_json::to_string(&wf).unwrap()).unwrap();
        assert_eq!(parsed["6"]["inputs"]["text"].as_str().unwrap(), tricky);
    }

    #[test]
    fn replace_int_by_node_and_field() {
        let mut wf = sample_workflow();
        let mut r = WorkflowReplacer::new(&mut wf);
        r.replace_int("3", "inputs.seed", 42).unwrap();
        assert_eq!(wf["3"]["inputs"]["seed"], 42);
    }

    #[test]
    fn replace_missing_path_errors() {
        let mut wf = sample_workflow();
        let mut r = WorkflowReplacer::new(&mut wf);
        let err = r.replace_text("99", "inputs.text", "x").unwrap_err();
        assert!(matches!(err, ReplaceError::PathNotFound(_)));
    }

    #[test]
    fn manifest_load_and_get() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("MANIFEST.toml");
        std::fs::write(
            &p,
            r#"
[templates.default]
positive_prompt_node = "6"
positive_prompt_field = "inputs.text"
seed_node = "3"
seed_field = "inputs.seed"
"#,
        )
        .unwrap();
        let m = Manifest::load(&p).unwrap();
        let e = m.get("default").unwrap();
        assert_eq!(e.positive_prompt_node, "6");
        assert_eq!(e.seed_field.as_deref(), Some("inputs.seed"));
    }

    #[test]
    fn manifest_missing_file_errors() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("nope.toml");
        let err = Manifest::load(&p).unwrap_err();
        assert!(matches!(err, ReplaceError::ManifestMissing(_)));
    }
}
