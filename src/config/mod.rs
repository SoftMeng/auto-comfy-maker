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
        let path = config_dir.join("default.toml");
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(&path)?;
        let cfg: AppConfig = toml::from_str(&text)?;
        cfg.validate()?;
        Ok(cfg)
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
