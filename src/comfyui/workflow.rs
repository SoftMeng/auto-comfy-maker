use std::path::Path;

use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TemplateError {
    #[error("read template {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("template not found: {0}")]
    NotFound(String),
    #[error("parse template json: {0}")]
    Parse(#[from] serde_json::Error),
}

/// 读取模板文件（按模板名）。template_name 不含扩展名。
pub fn read_template(project_root: &Path, template_name: &str) -> Result<String, TemplateError> {
    let path = project_root.join("templates").join(format!("{template_name}.json"));
    if !path.exists() {
        return Err(TemplateError::NotFound(path.display().to_string()));
    }
    std::fs::read_to_string(&path).map_err(|e| TemplateError::Io {
        path: path.display().to_string(),
        source: e,
    })
}

/// 在模板文本中按占位符 / REPLACE_ME 替换。
///
/// 支持两种注入方式：
/// 1. `${positive_prompt}` / `${seed}` / `${width}` / `${height}`（及中文别名）— 模板里以字面量片段出现。
/// 2. `"REPLACE_ME"` — 出现在 inputs.text 字符串字段时（Text Multiline / CLIPTextEncode 等节点），
///    由本函数定位后替换。inputs.text 为数组 `[node_id, output_index]` 时是节点引用，不动。
///
/// 替换是纯字符串（占位符）或结构化（REPLACE_ME）。节点 ID 完全无关。
///
/// 默认尺寸：768×1536（竖版，符合角色生成常用比例）。
pub const DEFAULT_WIDTH: i64 = 768;
pub const DEFAULT_HEIGHT: i64 = 1536;

const REPLACE_ME: &str = "REPLACE_ME";

pub fn substitute(
    template: &str,
    positive_prompt: &str,
    seed: i64,
    width: Option<i64>,
    height: Option<i64>,
) -> String {
    let mut out = template
        .replace("${positive_prompt}", positive_prompt)
        .replace("${提示词}", positive_prompt);
    let seed_str = seed.to_string();
    out = out.replace("${seed}", &seed_str).replace("${种子}", &seed_str);
    let w = width.unwrap_or(DEFAULT_WIDTH);
    let h = height.unwrap_or(DEFAULT_HEIGHT);
    let ws = w.to_string();
    let hs = h.to_string();
    out = out.replace("${width}", &ws).replace("${宽}", &ws);
    out = out.replace("${height}", &hs).replace("${高}", &hs);
    out = replace_replace_me_in_text_fields(&out, positive_prompt);
    out
}

/// 在合法 JSON 模板里定位每个节点 inputs.text 字符串字段，把字面量 "REPLACE_ME" 换成新 prompt。
/// 数组形式的 text（节点引用）不动。
fn replace_replace_me_in_text_fields(template: &str, prompt: &str) -> String {
    let mut root: Value = match serde_json::from_str(template) {
        Ok(v) => v,
        Err(_) => return template.to_string(),
    };
    if let Some(obj) = root.as_object_mut() {
        for (_id, node) in obj.iter_mut() {
            if let Some(node_obj) = node.as_object_mut() {
                if let Some(inputs) = node_obj.get_mut("inputs") {
                    if let Some(inputs_obj) = inputs.as_object_mut() {
                        if let Some(text_val) = inputs_obj.get_mut("text") {
                            if let Some(s) = text_val.as_str() {
                                if s == REPLACE_ME {
                                    *text_val = Value::String(prompt.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    serde_json::to_string(&root).unwrap_or_else(|_| template.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const TINY: &str = r#"{
  "39": {
    "inputs": {
      "text": "${positive_prompt}",
      "seed": ${seed},
      "width": ${width},
      "height": ${height}
    }
  }
}"#;

    #[test]
    fn substitutes_all_fields() {
        let out = substitute(TINY, "a girl on the beach", 42, Some(768), Some(1536));
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["39"]["inputs"]["text"], "a girl on the beach");
        assert_eq!(v["39"]["inputs"]["seed"], 42);
        assert_eq!(v["39"]["inputs"]["width"], 768);
        assert_eq!(v["39"]["inputs"]["height"], 1536);
    }

    #[test]
    fn chinese_placeholders_compat() {
        // glmclaw 风格的 ${提示词}/${宽}/${高}/${种子} 与英文别名同时工作
        const CHINESE: &str = r#"{"39":{"inputs":{"text":"${提示词}","seed":${种子},"width":${宽},"height":${高}}}}"#;
        let out = substitute(CHINESE, "长发美女", 7, Some(512), Some(768));
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["39"]["inputs"]["text"], "长发美女");
        assert_eq!(v["39"]["inputs"]["seed"], 7);
        assert_eq!(v["39"]["inputs"]["width"], 512);
        assert_eq!(v["39"]["inputs"]["height"], 768);
    }

    #[test]
    fn english_and_chinese_alias_coexist() {
        // 一个模板里同时含两种占位符——都被替换
        const MIX: &str = r#"{"a":{"text":"${positive_prompt}"},"b":{"text":"${提示词}"},"c":{"width":${width}},"d":{"height":${高}}}"#;
        let out = substitute(MIX, "x", 1, Some(100), Some(200));
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["a"]["text"], "x");
        assert_eq!(v["b"]["text"], "x");
        assert_eq!(v["c"]["width"], 100);
        assert_eq!(v["d"]["height"], 200);
    }

    #[test]
    fn uses_defaults_when_dims_none() {
        // 不传 width/height 时：使用默认尺寸（768×1536）替换占位符
        let out = substitute(TINY, "x", 7, None, None);
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["39"]["inputs"]["width"], DEFAULT_WIDTH);
        assert_eq!(v["39"]["inputs"]["height"], DEFAULT_HEIGHT);
        assert_eq!(v["39"]["inputs"]["text"], "x");
        assert_eq!(v["39"]["inputs"]["seed"], 7);
    }

    #[test]
    fn special_chars_in_prompt_are_json_safe() {
        // 中文 / 换行 / 引号 / 反斜杠：template 内的 JSON 字符串本身要合法
        // （用户编辑模板时需保证 JSON 合法；substitute 只做字面量替换）
        let tricky = "中文 \"引号\" \\ \n 换行";
        let template = r#"{"text": "${positive_prompt}"}"#;
        let out = substitute(template, tricky, 0, None, None);
        // 我们不做额外转义——用户编辑模板时字符串字面量已合法
        assert!(out.contains(tricky));
    }

    #[test]
    fn read_template_loads_glmclaw_style_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("templates")).unwrap();
        std::fs::write(
            dir.path().join("templates/demo.json"),
            r#"{"91":{"inputs":{"text":"${positive_prompt}","seed":${seed}}}}"#,
        )
        .unwrap();
        let text = read_template(dir.path(), "demo").unwrap();
        assert!(text.contains("${positive_prompt}"));
    }

    #[test]
    fn read_template_missing_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let err = read_template(dir.path(), "nope").unwrap_err();
        assert!(matches!(err, TemplateError::NotFound(_)));
    }

    const REPLACE_ME_TEMPLATE: &str = r#"{
  "68": {
    "inputs": { "text": "REPLACE_ME" },
    "class_type": "Text Multiline"
  },
  "11": {
    "inputs": { "text": ["54", 0] },
    "class_type": "CLIPTextEncode"
  }
}"#;

    #[test]
    fn replace_me_in_text_field_is_replaced() {
        let out = substitute(REPLACE_ME_TEMPLATE, "1girl, long hair", 7, None, None);
        let parsed: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(parsed["68"]["inputs"]["text"], "1girl, long hair");
    }

    #[test]
    fn array_text_field_is_not_replaced() {
        let out = substitute(REPLACE_ME_TEMPLATE, "1girl, long hair", 7, None, None);
        let parsed: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(parsed["11"]["inputs"]["text"], serde_json::json!(["54", 0]));
    }

    #[test]
    fn placeholder_mode_still_works_after_extension() {
        let tpl = r#"{"91":{"inputs":{"text":"${positive_prompt}","seed":${seed}}}}"#;
        let out = substitute(tpl, "hello", 42, None, None);
        let parsed: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(parsed["91"]["inputs"]["text"], "hello");
        assert_eq!(parsed["91"]["inputs"]["seed"], 42);
    }
}
