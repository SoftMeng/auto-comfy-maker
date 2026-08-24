use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ThemeError {
    #[error("read theme file: {0}")]
    Io(#[from] std::io::Error),
    #[error("parse theme toml: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("theme not found: {0}")]
    NotFound(String),
    #[error("invalid theme field: {field}: {reason}")]
    Invalid { field: String, reason: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeMeta {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub lang: String,
    #[serde(default = "default_version")]
    pub version: String,
}

fn default_version() -> String {
    "1.0".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryRef {
    pub file: String,
    pub count: usize,
    #[serde(default)]
    pub max: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptionalCategoryRef {
    pub file: String,
    pub probability: f32,
    #[serde(default = "default_optional_count")]
    pub count: usize,
}

fn default_optional_count() -> usize {
    1
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OrderSections {
    #[serde(default)]
    pub fixed: HashMap<String, CategoryRef>,
    #[serde(default)]
    pub random: HashMap<String, CategoryRef>,
    #[serde(default)]
    pub optional: HashMap<String, OptionalCategoryRef>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Compatibility {
    #[serde(default)]
    pub conflicts: HashMap<String, Vec<Vec<String>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationOptions {
    #[serde(default = "default_max_elements")]
    pub max_elements: usize,
    #[serde(default = "default_max_length")]
    pub max_length: usize,
}

impl Default for GenerationOptions {
    fn default() -> Self {
        Self {
            max_elements: default_max_elements(),
            max_length: default_max_length(),
        }
    }
}

fn default_max_elements() -> usize {
    30
}
fn default_max_length() -> usize {
    800
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub meta: ThemeMeta,
    #[serde(default)]
    pub order: OrderSections,
    #[serde(default)]
    pub compatibility: Compatibility,
    #[serde(default)]
    pub generation: GenerationOptions,
}

impl Theme {
    pub fn load(themes_dir: &Path, id: &str) -> Result<Self, ThemeError> {
        let path = themes_dir.join(format!("{id}.toml"));
        if !path.exists() {
            return Err(ThemeError::NotFound(id.to_string()));
        }
        let text = std::fs::read_to_string(&path)?;
        let theme: Theme = toml::from_str(&text)?;
        theme.validate()?;
        Ok(theme)
    }

    pub fn validate(&self) -> Result<(), ThemeError> {
        match self.meta.lang.as_str() {
            "zh" | "en" | "mixed" => {}
            other => {
                return Err(ThemeError::Invalid {
                    field: "meta.lang".into(),
                    reason: format!("must be zh|en|mixed, got {other}"),
                })
            }
        }
        if self.meta.id.trim().is_empty() {
            return Err(ThemeError::Invalid {
                field: "meta.id".into(),
                reason: "must not be empty".into(),
            });
        }
        for (name, cat) in self.order.fixed.iter().chain(self.order.random.iter()) {
            if cat.count == 0 {
                return Err(ThemeError::Invalid {
                    field: format!("order.{name}.count"),
                    reason: "must be > 0 for fixed/random".into(),
                });
            }
            if let Some(m) = cat.max {
                if m < cat.count {
                    return Err(ThemeError::Invalid {
                        field: format!("order.{name}.max"),
                        reason: format!("max ({m}) must be >= count ({})", cat.count),
                    });
                }
            }
        }
        for (name, opt) in self.order.optional.iter() {
            if !(0.0..=1.0).contains(&opt.probability) {
                return Err(ThemeError::Invalid {
                    field: format!("order.optional.{name}.probability"),
                    reason: "must be in [0.0, 1.0]".into(),
                });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_theme(dir: &Path, name: &str, content: &str) {
        let path = dir.join(format!("{name}.toml"));
        let mut f = std::fs::File::create(path).unwrap();
        f.write_all(content.as_bytes()).unwrap();
    }

    #[test]
    fn load_valid_theme() {
        let dir = tempfile::tempdir().unwrap();
        write_theme(
            dir.path(),
            "demo",
            r#"
[meta]
id = "demo"
name = "Demo"
lang = "zh"

[order.fixed]
style = { file = "tags/zh/风格.txt", count = 1 }

[order.random]
hair = { file = "tags/zh/发型.txt", count = 1, max = 2 }
"#,
        );
        let t = Theme::load(dir.path(), "demo").expect("load");
        assert_eq!(t.meta.id, "demo");
        assert!(t.order.fixed.contains_key("style"));
        assert_eq!(t.order.random["hair"].count, 1);
    }

    #[test]
    fn missing_theme_returns_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let err = Theme::load(dir.path(), "nope").unwrap_err();
        assert!(matches!(err, ThemeError::NotFound(_)));
    }

    #[test]
    fn reject_invalid_lang() {
        let dir = tempfile::tempdir().unwrap();
        write_theme(
            dir.path(),
            "bad",
            r#"
[meta]
id = "bad"
name = "bad"
lang = "jp"
"#,
        );
        let err = Theme::load(dir.path(), "bad").unwrap_err();
        assert!(matches!(err, ThemeError::Invalid { .. }));
    }

    #[test]
    fn reject_invalid_probability() {
        let dir = tempfile::tempdir().unwrap();
        write_theme(
            dir.path(),
            "bad",
            r#"
[meta]
id = "bad"
name = "bad"
lang = "zh"

[order.optional]
scene = { file = "tags/zh/场景.txt", probability = 1.5 }
"#,
        );
        let err = Theme::load(dir.path(), "bad").unwrap_err();
        assert!(matches!(err, ThemeError::Invalid { .. }));
    }
}
