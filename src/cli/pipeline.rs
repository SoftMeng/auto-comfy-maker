use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result};
use serde_json::Value;

use crate::comfyui::download::download_and_save;
use crate::comfyui::prompt::{poll_until_ready, submit_prompt};
use crate::comfyui::{make_client_id, read_template, substitute, ComfyuiClient};
use crate::config::AppConfig;
use crate::prompt_engine::{
    build_agent, combine, refine, AgentKind, CombineContext, CombineStrategy, LlmConfig, Provider,
};
use crate::tags::{Lang, LangAwarePool};
use crate::theme::Theme;

pub struct PipelineOpts {
    pub theme_name: String,
    pub lang: Option<String>,
    pub strategy: Option<String>,
    pub max_length: Option<usize>,
    pub seed: u64,
    pub template: String,
    pub no_send: bool,
    pub use_refine: bool,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

pub struct PipelineOutcome {
    pub combine_prompt: String,
    pub final_prompt: String,
    pub image_path: Option<PathBuf>,
}

pub async fn run_pipeline(
    opts: &PipelineOpts,
    config: &AppConfig,
    project_root: &Path,
) -> Result<PipelineOutcome> {
    let lang = resolve_lang(opts.lang.as_deref(), config)?;
    let strategy = resolve_strategy(opts.strategy.as_deref(), config)?;
    let max_length = opts.max_length.unwrap_or(config.prompt.default_max_length);
    let seed = if opts.seed == 0 {
        config.prompt.default_seed
    } else {
        opts.seed
    };

    let theme = Theme::load(&config.themes_root(project_root), &opts.theme_name)
        .with_context(|| format!("load theme '{}'", opts.theme_name))?;
    let mut pool = LangAwarePool::new();
    pool.load_dir(lang, &config.tags_root(project_root).join(lang.as_str()))
        .context("load tags")?;

    let ctx = CombineContext {
        theme,
        lang,
        strategy,
        max_length,
        seed,
    };
    let out = combine(&ctx, &pool).context("combine prompt")?;
    for (cat, val) in &out.selected {
        tracing::debug!(category = %cat, value = %val, "selected");
    }

    let agent = if opts.use_refine {
        build_llm_agent(config)
    } else {
        None
    };
    let final_prompt = refine(agent.as_ref(), &out.prompt).await;

    if opts.no_send {
        tracing::info!("--no-send specified, skipping ComfyUI submission");
        return Ok(PipelineOutcome {
            combine_prompt: out.prompt,
            final_prompt,
            image_path: None,
        });
    }

    let workflow = build_workflow(config, project_root, &opts.template, &final_prompt, seed, opts.width, opts.height)?;
    let image_path =
        Some(submit_and_download(config, &workflow, project_root, &final_prompt).await?);

    Ok(PipelineOutcome {
        combine_prompt: out.prompt,
        final_prompt,
        image_path,
    })
}

pub async fn run_fixed_prompt(
    prompt: &str,
    template: &str,
    seed: u64,
    config: &AppConfig,
    project_root: &Path,
) -> Result<PathBuf> {
    let workflow = build_workflow(config, project_root, template, prompt, seed, None, None)?;
    submit_and_download(config, &workflow, project_root, prompt).await
}

fn build_workflow(
    _config: &AppConfig,
    project_root: &Path,
    template: &str,
    prompt: &str,
    seed: u64,
    width: Option<u32>,
    height: Option<u32>,
) -> Result<Value> {
    let template_text = read_template(project_root, template)
        .with_context(|| format!("read template '{template}'"))?;
    let seed_value: i64 = if seed == 0 {
        chrono::Utc::now().timestamp()
    } else {
        seed as i64
    };
    let substituted = substitute(
        &template_text,
        prompt,
        seed_value,
        width.map(|w| w as i64),
        height.map(|h| h as i64),
    );
    let workflow: Value = serde_json::from_str(&substituted)
        .with_context(|| format!("parse substituted template '{template}'"))?;
    Ok(workflow)
}

async fn submit_and_download(
    config: &AppConfig,
    workflow: &Value,
    project_root: &Path,
    prompt: &str,
) -> Result<PathBuf> {
    let client = ComfyuiClient::new(&config.comfyui.url).context("init comfyui client")?;
    let client_id = make_client_id();
    let prompt_id = submit_prompt(&client, workflow, &client_id)
        .await
        .context("submit to comfyui")?;
    tracing::info!(prompt_id = %prompt_id, "submitted to comfyui");

    let history = poll_until_ready(
        &client,
        &prompt_id,
        Duration::from_secs(config.comfyui.timeout_secs),
        Duration::from_secs(config.comfyui.poll_interval_secs),
    )
    .await
    .context("poll comfyui")?;

    let output_root = config.output_root(project_root);
    let path = download_and_save(&client, &history, &prompt_id, prompt, &output_root)
        .await
        .context("download and save image")?;
    Ok(path)
}

pub fn resolve_lang(s: Option<&str>, cfg: &AppConfig) -> Result<Lang> {
    let raw = s.unwrap_or(&cfg.prompt.default_lang);
    Lang::parse(raw).with_context(|| format!("unknown language: {raw}"))
}

pub fn resolve_strategy(s: Option<&str>, cfg: &AppConfig) -> Result<CombineStrategy> {
    let raw = s.unwrap_or(&cfg.prompt.default_strategy);
    CombineStrategy::parse(raw).with_context(|| format!("unknown strategy: {raw}"))
}

pub fn build_llm_agent(cfg: &AppConfig) -> Option<AgentKind> {
    let llm = &cfg.llm;
    if !llm.enabled {
        tracing::info!("llm disabled in config; refine() will be identity");
        return None;
    }
    let provider = match Provider::parse(&llm.provider) {
        Some(p) => p,
        None => {
            tracing::warn!(provider = %llm.provider, "unknown llm provider, refine() will be identity");
            return None;
        }
    };
    let api_key = if llm.api_key.trim().is_empty() {
        let env_var = match provider {
            Provider::OpenAI => "OPENAI_API_KEY",
            Provider::Anthropic => "ANTHROPIC_API_KEY",
        };
        match std::env::var(env_var) {
            Ok(v) => v,
            Err(_) => {
                tracing::warn!(
                    env_var,
                    "no api key in config or env, refine() will be identity"
                );
                return None;
            }
        }
    } else {
        llm.api_key.clone()
    };

    let llm_cfg = LlmConfig {
        provider,
        model: llm.model.clone(),
        api_key,
        base_url: llm.base_url.clone(),
        disable_thinking: llm.disable_thinking,
    };
    match build_agent(&llm_cfg) {
        Ok(a) => Some(a),
        Err(e) => {
            tracing::warn!(error = %e, "build_agent failed, refine() will be identity");
            None
        }
    }
}
