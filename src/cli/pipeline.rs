use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result};
use serde_json::Value;

use crate::comfyui::download::download_and_save;
use crate::comfyui::prompt::{poll_until_ready, submit_prompt};
use crate::comfyui::workflow::Manifest;
use crate::comfyui::{make_client_id, ComfyuiClient, WorkflowReplacer};
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
    let seed = if opts.seed == 0 { config.prompt.default_seed } else { opts.seed };

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
        project_root: project_root.to_path_buf(),
    };
    let out = combine(&ctx, &pool).context("combine prompt")?;

    let agent = if opts.use_refine { build_llm_agent(config) } else { None };
    let final_prompt = refine(agent.as_ref(), &out.prompt).await;

    if opts.no_send {
        tracing::info!("--no-send specified, skipping ComfyUI submission");
        return Ok(PipelineOutcome {
            combine_prompt: out.prompt,
            final_prompt,
            image_path: None,
        });
    }

    let workflow = build_workflow(config, project_root, &opts.template, &final_prompt, seed)?;
    let image_path = Some(submit_and_download(config, &workflow).await?);

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
    let workflow = build_workflow(config, project_root, template, prompt, seed)?;
    submit_and_download(config, &workflow).await
}

fn build_workflow(
    config: &AppConfig,
    project_root: &Path,
    template: &str,
    prompt: &str,
    seed: u64,
) -> Result<Value> {
    let template_path = config
        .templates_root(project_root)
        .join(format!("{}.json", template));
    if !template_path.exists() {
        anyhow::bail!(
            "template not found: {} (add templates/{}.json)",
            template_path.display(),
            template
        );
    }
    let template_text = std::fs::read_to_string(&template_path)
        .with_context(|| format!("read template {}", template_path.display()))?;
    let mut workflow: Value = serde_json::from_str(&template_text)
        .with_context(|| format!("parse template {}", template_path.display()))?;

    let manifest_path = config.templates_root(project_root).join("MANIFEST.toml");
    let manifest = Manifest::load(&manifest_path).with_context(|| {
        format!(
            "load MANIFEST.toml (required to map prompt/seed fields; see {})",
            manifest_path.display()
        )
    })?;
    let entry = manifest
        .get(template)
        .ok_or_else(|| anyhow::anyhow!("template '{}' not in MANIFEST.toml", template))?;

    {
        let mut replacer = WorkflowReplacer::new(&mut workflow);
        replacer
            .replace_text(&entry.positive_prompt_node, &entry.positive_prompt_field, prompt)
            .context("replace positive prompt in workflow")?;
        if let (Some(node), Some(field)) = (
            entry.negative_prompt_node.as_deref(),
            entry.negative_prompt_field.as_deref(),
        ) {
            replacer
                .replace_text(node, field, "")
                .context("replace negative prompt in workflow")?;
        }
        if let (Some(node), Some(field)) = (entry.seed_node.as_deref(), entry.seed_field.as_deref())
        {
            let seed_value =
                if seed == 0 { chrono::Utc::now().timestamp() as i64 } else { seed as i64 };
            replacer
                .replace_int(node, field, seed_value)
                .context("replace seed in workflow")?;
        }
    }

    Ok(workflow)
}

async fn submit_and_download(config: &AppConfig, workflow: &Value) -> Result<PathBuf> {
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

    let prompt_text = String::new();
    let output_root = std::path::PathBuf::from(&config.paths.output_dir);
    let path = download_and_save(&client, &history, &prompt_id, &prompt_text, &output_root)
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
                tracing::warn!(env_var, "no api key in config or env, refine() will be identity");
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
    };
    match build_agent(&llm_cfg) {
        Ok(a) => Some(a),
        Err(e) => {
            tracing::warn!(error = %e, "build_agent failed, refine() will be identity");
            None
        }
    }
}
