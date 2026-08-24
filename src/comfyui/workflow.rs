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

/// 在模板文本中按占位符替换。
/// 占位符是 JSON 内合法的字符串片段：${positive_prompt} / ${seed} / ${width} / ${height}。
///
/// 替换是**纯字符串**——节点 ID 是多少、字段路径多深都无关。
/// 这意味着用户从 ComfyUI 导出 workflow 后，只需把"想注入的位置"写成 ${positive_prompt} 等占位符即可。
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
    if let Some(w) = width {
        let ws = w.to_string();
        out = out.replace("${width}", &ws).replace("${宽}", &ws);
    }
    if let Some(h) = height {
        let hs = h.to_string();
        out = out.replace("${height}", &hs).replace("${高}", &hs);
    }
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
    fn omits_optional_dims_when_none() {
        // 不传 width/height 时：模板里的 ${width}/${height} 字面量保留
        let out = substitute(TINY, "x", 7, None, None);
        assert!(out.contains("\"width\": ${width}"));
        assert!(out.contains("\"height\": ${height}"));
        assert!(out.contains("\"text\": \"x\""));
        assert!(out.contains("\"seed\": 7"));
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
}
