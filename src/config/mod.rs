use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("read config file: {0}")]
    Io(#[from] std::io::Error),
    #[error("parse config toml: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("invalid value: {field} = {value}: {reason}")]
    Invalid {
        field: String,
        value: String,
        reason: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub app: AppSection,
    pub prompt: PromptSection,
    pub comfyui: ComfyuiSection,
    #[serde(default)]
    pub llm: LlmSection,
    pub paths: PathsSection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSection {
    pub name: String,
    pub log_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptSection {
    pub default_lang: String,
    pub default_strategy: String,
    pub default_max_length: usize,
    pub default_seed: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComfyuiSection {
    pub url: String,
    pub poll_interval_secs: u64,
    pub timeout_secs: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LlmSection {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_provider")]
    pub provider: String,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub base_url: String,
}

fn default_provider() -> String {
    "openai".to_string()
}
fn default_model() -> String {
    "gpt-4o-mini".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathsSection {
    pub themes_dir: String,
    pub tags_dir: String,
    pub output_dir: String,
    pub logs_dir: String,
    pub templates_dir: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            app: AppSection {
                name: "auto-comfy-maker".to_string(),
                log_level: "info".to_string(),
            },
            prompt: PromptSection {
                default_lang: "zh".to_string(),
                default_strategy: "comma".to_string(),
                default_max_length: 800,
                default_seed: 0,
            },
            comfyui: ComfyuiSection {
                url: "http://127.0.0.1:8188".to_string(),
                poll_interval_secs: 2,
                timeout_secs: 300,
            },
            llm: LlmSection::default(),
            paths: PathsSection {
                themes_dir: "themes".to_string(),
                tags_dir: "tags".to_string(),
                output_dir: "output".to_string(),
                logs_dir: "logs".to_string(),
                templates_dir: "templates".to_string(),
            },
        }
    }
}

impl AppConfig {
    pub fn load(config_dir: &Path) -> Result<Self, ConfigError> {
        let mut cfg = if config_dir.join("default.toml").exists() {
            let text = std::fs::read_to_string(config_dir.join("default.toml"))?;
            toml::from_str(&text)?
        } else {
            Self::default()
        };
        cfg.merge_local(config_dir)?;
        cfg.validate()?;
        Ok(cfg)
    }

    /// 以"段（section）为单位"合并 local.toml：local 中存在的段整体覆盖 default。
    fn merge_local(&mut self, config_dir: &Path) -> Result<(), ConfigError> {
        let local_path = config_dir.join("local.toml");
        if !local_path.exists() {
            return Ok(());
        }
        let text = std::fs::read_to_string(&local_path)?;
        let local: toml::Value = toml::from_str(&text)?;

        let mut base: toml::Value = toml::Value::try_from(&*self)
            .map_err(|e| ConfigError::Invalid {
                field: "(serialize)".into(),
                value: "AppConfig".into(),
                reason: e.to_string(),
            })?;

        if let Some(local_table) = local.as_table() {
            let base_table = base
                .as_table_mut()
                .ok_or_else(|| ConfigError::Invalid {
                    field: "(root)".into(),
                    value: "(non-table)".into(),
                    reason: "local.toml must be a TOML table at root".into(),
                })?;
            for (key, local_val) in local_table {
                // 段级 = 字段级合并：local 段中只列了要覆盖的字段
                if let (Some(base_val), Some(local_inner)) =
                    (base_table.get_mut(key), local_val.as_table())
                {
                    if let Some(base_inner) = base_val.as_table_mut() {
                        for (k, v) in local_inner {
                            base_inner.insert(k.clone(), v.clone());
                        }
                        continue;
                    }
                }
                // 段不存在 / 类型不匹配：整段覆盖
                base_table.insert(key.clone(), local_val.clone());
            }
        }

        let merged: AppConfig = base
            .try_into()
            .map_err(|e: toml::de::Error| ConfigError::Parse(e))?;
        *self = merged;
        Ok(())
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        match self.app.log_level.as_str() {
            "trace" | "debug" | "info" | "warn" | "error" => {}
            other => {
                return Err(ConfigError::Invalid {
                    field: "app.log_level".into(),
                    value: other.into(),
                    reason: "must be one of trace|debug|info|warn|error".into(),
                })
            }
        }
        match self.prompt.default_lang.as_str() {
            "zh" | "en" | "mixed" => {}
            other => {
                return Err(ConfigError::Invalid {
                    field: "prompt.default_lang".into(),
                    value: other.into(),
                    reason: "must be one of zh|en|mixed".into(),
                })
            }
        }
        match self.prompt.default_strategy.as_str() {
            "comma" | "newline" | "natural" => {}
            other => {
                return Err(ConfigError::Invalid {
                    field: "prompt.default_strategy".into(),
                    value: other.into(),
                    reason: "must be one of comma|newline|natural".into(),
                })
            }
        }
        Ok(())
    }

    pub fn themes_root(&self, project_root: &Path) -> PathBuf {
        project_root.join(&self.paths.themes_dir)
    }

    pub fn tags_root(&self, project_root: &Path) -> PathBuf {
        project_root.join(&self.paths.tags_dir)
    }

    pub fn output_root(&self, project_root: &Path) -> PathBuf {
        project_root.join(&self.paths.output_dir)
    }

    pub fn templates_root(&self, project_root: &Path) -> PathBuf {
        project_root.join(&self.paths.templates_dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_valid() {
        AppConfig::default().validate().expect("default valid");
    }

    #[test]
    fn load_from_existing_dir() {
        let cfg = AppConfig::load(Path::new("config")).expect("load config");
        assert_eq!(cfg.app.name, "auto-comfy-maker");
    }

    #[test]
    fn load_from_missing_dir_returns_default() {
        let cfg = AppConfig::load(Path::new("/nonexistent/dir")).expect("fallback default");
        assert_eq!(cfg.prompt.default_lang, "zh");
    }

    #[test]
    fn local_overrides_default_section() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("default.toml"),
            r#"
[app]
name = "auto-comfy-maker"
log_level = "info"

[prompt]
default_lang = "zh"
default_strategy = "comma"
default_max_length = 800
default_seed = 0

[comfyui]
url = "http://default-host:8188"
poll_interval_secs = 2
timeout_secs = 300

[llm]
enabled = false
provider = "openai"
model = "gpt-4o-mini"

[paths]
themes_dir = "themes"
tags_dir = "tags"
output_dir = "output"
logs_dir = "logs"
templates_dir = "templates"
"#,
        )
        .unwrap();
        std::fs::write(
            dir.path().join("local.toml"),
            r#"
[comfyui]
url = "http://user-host:9999"
"#,
        )
        .unwrap();

        let cfg = AppConfig::load(dir.path()).expect("load");
        assert_eq!(cfg.comfyui.url, "http://user-host:9999");
        // 未覆盖的段保持 default
        assert_eq!(cfg.comfyui.poll_interval_secs, 2);
        assert_eq!(cfg.prompt.default_lang, "zh");
    }

    #[test]
    fn local_only_overrides_present_sections() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("local.toml"),
            r#"
[prompt]
default_lang = "en"
"#,
        )
        .unwrap();

        let cfg = AppConfig::load(dir.path()).expect("load with no default");
        // 没有 default.toml → fallback default；local 覆盖 prompt
        assert_eq!(cfg.prompt.default_lang, "en");
        assert_eq!(cfg.app.name, "auto-comfy-maker"); // default 值
    }

    #[test]
    fn local_rejects_invalid_toml() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("local.toml"),
            "this is not valid toml [[[",
        )
        .unwrap();
        let r = AppConfig::load(dir.path());
        assert!(r.is_err());
    }

    #[test]
    fn reject_invalid_log_level() {
        let mut cfg = AppConfig::default();
        cfg.app.log_level = "verbose".into();
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn reject_invalid_lang() {
        let mut cfg = AppConfig::default();
        cfg.prompt.default_lang = "klingon".into();
        assert!(cfg.validate().is_err());
    }
}
