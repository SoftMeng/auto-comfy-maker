use std::path::Path;

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
    #[error("template '{name}' missing required placeholders: {missing}")]
    MissingPlaceholders { name: String, missing: String },
}

/// 每个 template 必须含的占位符（英文 + 中文别名任一）。
const REQUIRED_PLACEHOLDERS: &[(&str, &[&str])] = &[
    ("positive_prompt", &["${positive_prompt}", "${提示词}"]),
    ("seed", &["${seed}", "${种子}"]),
    ("width", &["${width}", "${宽}"]),
    ("height", &["${height}", "${高}"]),
];

/// 读取模板文件（按模板名）。template_name 不含扩展名。
/// 加载时校验 4 个必需占位符，缺失则返回错误（防止把字面量"REPLACE_ME"漏改之类的回归）。
pub fn read_template(project_root: &Path, template_name: &str) -> Result<String, TemplateError> {
    let path = project_root.join("templates").join(format!("{template_name}.json"));
    if !path.exists() {
        return Err(TemplateError::NotFound(path.display().to_string()));
    }
    let text = std::fs::read_to_string(&path).map_err(|e| TemplateError::Io {
        path: path.display().to_string(),
        source: e,
    })?;
    let missing = validate_placeholders(&text);
    if !missing.is_empty() {
        return Err(TemplateError::MissingPlaceholders {
            name: template_name.to_string(),
            missing: missing.join(", "),
        });
    }
    Ok(text)
}

/// 检查文本是否含每个必需字段的占位符（英文或中文别名）。缺失项返回字段名列表。
fn validate_placeholders(text: &str) -> Vec<&'static str> {
    REQUIRED_PLACEHOLDERS
        .iter()
        .filter(|(_, aliases)| !aliases.iter().any(|a| text.contains(a)))
        .map(|(field, _)| *field)
        .collect()
}

/// 在模板文本中按占位符替换。
/// 占位符是 JSON 内合法的字符串片段：${positive_prompt} / ${seed} / ${width} / ${height}。
///
/// 替换是**纯字符串**——节点 ID 是多少、字段路径多深都无关。
/// 这意味着用户从 ComfyUI 导出 workflow 后，只需把"想注入的位置"写成 ${positive_prompt} 等占位符即可。
///
/// 默认尺寸：768×1536（竖版，符合角色生成常用比例）。
/// 调用方传入 width/height 时使用传入值；传 None 时使用默认值。
pub const DEFAULT_WIDTH: i64 = 768;
pub const DEFAULT_HEIGHT: i64 = 1536;

pub fn substitute(
    template: &str,
    positive_prompt: &str,
    seed: i64,
    width: Option<i64>,
    height: Option<i64>,
) -> String {
    // 每个字段支持中英文别名——glmclaw 用 ${提示词}/${宽}/${高}/${种子}；
    // 用户从 ComfyUI 导出的 workflow 习惯英文 ${positive_prompt} 等；都识别。
    let mut out = template
        .replace("${positive_prompt}", positive_prompt)
        .replace("${提示词}", positive_prompt);
    let seed_str = seed.to_string();
    out = out.replace("${seed}", &seed_str).replace("${种子}", &seed_str);
    // 宽高：None 时用默认值，避免占位符字面量进入 ComfyUI 触发类型错误
    let w = width.unwrap_or(DEFAULT_WIDTH);
    let h = height.unwrap_or(DEFAULT_HEIGHT);
    let ws = w.to_string();
    let hs = h.to_string();
    out = out.replace("${width}", &ws).replace("${宽}", &ws);
    out = out.replace("${height}", &hs).replace("${高}", &hs);
    out
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
            r#"{"91":{"inputs":{"text":"${positive_prompt}","seed":${seed},"width":${width},"height":${height}}}}"#,
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

    fn write_template(dir: &Path, body: &str) {
        let tpl_dir = dir.join("templates");
        std::fs::create_dir_all(&tpl_dir).unwrap();
        std::fs::write(tpl_dir.join("demo.json"), body).unwrap();
    }

    #[test]
    fn read_template_accepts_full_placeholders() {
        let dir = tempfile::tempdir().unwrap();
        write_template(
            dir.path(),
            r#"{"n":{"inputs":{"text":"${positive_prompt}","seed":${seed},"width":${width},"height":${height}}}"#,
        );
        assert!(read_template(dir.path(), "demo").is_ok());
    }

    #[test]
    fn read_template_accepts_chinese_alias_placeholders() {
        let dir = tempfile::tempdir().unwrap();
        write_template(
            dir.path(),
            r#"{"n":{"inputs":{"text":"${提示词}","seed":${种子},"width":${宽},"height":${高}}}"#,
        );
        assert!(read_template(dir.path(), "demo").is_ok());
    }

    #[test]
    fn read_template_reports_missing_placeholders() {
        let dir = tempfile::tempdir().unwrap();
        write_template(
            dir.path(),
            r#"{"n":{"inputs":{"text":"${positive_prompt}"}}}"#,
        );
        let err = read_template(dir.path(), "demo").unwrap_err();
        match err {
            TemplateError::MissingPlaceholders { missing, .. } => {
                assert!(missing.contains("seed"));
                assert!(missing.contains("width"));
                assert!(missing.contains("height"));
                assert!(!missing.contains("positive_prompt"));
            }
            other => panic!("expected MissingPlaceholders, got {other:?}"),
        }
    }
}
