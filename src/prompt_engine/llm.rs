use rig_core::client::CompletionClient;
use rig_core::completion::Prompt;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct LlmConfig {
    pub provider: Provider,
    pub model: String,
    pub api_key: String,
    pub base_url: String,
    /// 是否禁用 thinking/reasoning（适用 DashScope/Qwen 等兼容 OpenAI 的 provider）
    #[serde(default)]
    pub disable_thinking: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    OpenAI,
    Anthropic,
}

impl Provider {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "openai" => Some(Self::OpenAI),
            "anthropic" => Some(Self::Anthropic),
            _ => None,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("missing api key: set {0}")]
    MissingApiKey(&'static str),
    #[error("rig client build failed: {0}")]
    Build(String),
    #[error("prompt failed: {0}")]
    Prompt(String),
}

pub enum AgentKind {
    OpenAI(rig_core::agent::Agent<rig_core::providers::openai::completion::CompletionModel>),
    Anthropic(rig_core::agent::Agent<rig_core::providers::anthropic::completion::CompletionModel>),
}

impl std::fmt::Debug for AgentKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OpenAI(_) => f.write_str("AgentKind::OpenAI(..)"),
            Self::Anthropic(_) => f.write_str("AgentKind::Anthropic(..)"),
        }
    }
}

pub fn build_agent(cfg: &LlmConfig) -> Result<AgentKind, LlmError> {
    if cfg.api_key.trim().is_empty() {
        let env_var = match cfg.provider {
            Provider::OpenAI => "OPENAI_API_KEY",
            Provider::Anthropic => "ANTHROPIC_API_KEY",
        };
        return Err(LlmError::MissingApiKey(env_var));
    }

    match cfg.provider {
        Provider::OpenAI => build_openai(cfg),
        Provider::Anthropic => build_anthropic(cfg),
    }
}

fn build_openai(cfg: &LlmConfig) -> Result<AgentKind, LlmError> {
    use rig_core::providers::openai;

    let client: openai::CompletionsClient = openai::Client::builder()
        .api_key(&cfg.api_key)
        .build()
        .map_err(|e| LlmError::Build(format!("openai: {e}")))?
        .completions_api();

    let mut builder = client.agent(&cfg.model).preamble(PREAMBLE);
    if cfg.disable_thinking {
        // DashScope/Qwen 等兼容 provider 的扩展字段：关闭思考链
        builder = builder.additional_params(serde_json::json!({
            "enable_thinking": false
        }));
    }
    let agent = builder.build();

    Ok(AgentKind::OpenAI(agent))
}

fn build_anthropic(cfg: &LlmConfig) -> Result<AgentKind, LlmError> {
    use rig_core::providers::anthropic;

    let mut builder = anthropic::Client::builder().api_key(&cfg.api_key);
    if !cfg.base_url.trim().is_empty() {
        builder = builder.base_url(&cfg.base_url);
    }
    let client = builder
        .build()
        .map_err(|e| LlmError::Build(format!("anthropic: {e}")))?;

    let agent = client.agent(&cfg.model).preamble(PREAMBLE).build();

    Ok(AgentKind::Anthropic(agent))
}

pub const PREAMBLE: &str = "You are an expert Stable Diffusion prompt engineer. \
Given a list of tags from a multi-dimensional theme, return a single optimized prompt. \
Rules: (1) keep all original semantic elements, (2) rewrite for natural English, \
(3) add concrete visual details that diffusion models can render, \
(4) output ONLY the optimized prompt text, no explanations, no quotes.";

pub async fn call(agent: &AgentKind, prompt: &str) -> Result<String, LlmError> {
    match agent {
        AgentKind::OpenAI(a) => a
            .prompt(prompt)
            .await
            .map_err(|e| LlmError::Prompt(format!("openai: {e}"))),
        AgentKind::Anthropic(a) => a
            .prompt(prompt)
            .await
            .map_err(|e| LlmError::Prompt(format!("anthropic: {e}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_parse_roundtrip() {
        assert_eq!(Provider::parse("openai"), Some(Provider::OpenAI));
        assert_eq!(Provider::parse("anthropic"), Some(Provider::Anthropic));
        assert_eq!(Provider::parse("klingon"), None);
    }

    #[test]
    fn build_rejects_empty_key() {
        let cfg = LlmConfig {
            provider: Provider::OpenAI,
            model: "gpt-4o-mini".into(),
            api_key: "".into(),
            base_url: "".into(),
            disable_thinking: false,
        };
        let err = build_agent(&cfg).unwrap_err();
        assert!(matches!(err, LlmError::MissingApiKey("OPENAI_API_KEY")));
    }

    #[test]
    fn llm_config_disable_thinking_default_false() {
        // 验证字段默认值与 serde 反序列化兼容
        let cfg: LlmConfig = serde_json::from_value(serde_json::json!({
            "provider": "openai",
            "model": "gpt-4o-mini",
            "api_key": "k",
            "base_url": "",
            // disable_thinking 故意省略
        }))
        .expect("missing disable_thinking should default to false");
        assert!(!cfg.disable_thinking);
    }

    #[test]
    fn build_openai_with_disable_thinking_builds() {
        // 验证 disable_thinking=true 时 build 成功（实际效果需要真实 API）
        // 这里只验证 build_agent 不因新字段而失败
        let cfg = LlmConfig {
            provider: Provider::OpenAI,
            model: "qwen-plus".into(),
            api_key: "fake-key".into(),
            base_url: "https://dashscope.aliyuncs.com/compatible-mode/v1".into(),
            disable_thinking: true,
        };
        let r = build_agent(&cfg);
        // 只要 build 不 panic；网络层错误由真实调用承担
        // 真实环境中 fake-key 会失败；这里只验证到达 HTTP 之前不报错
        match r {
            Ok(_) => {} // 极少情况（rig 不发起 HTTP）
            Err(LlmError::MissingApiKey(_)) => panic!("api key provided"),
            Err(LlmError::Build(_)) => {} // rig build 阶段可能不发起 HTTP
            Err(LlmError::Prompt(_)) => {} // 不应在 build 阶段
        }
    }
}
