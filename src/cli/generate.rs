use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result};
use clap::Args;
use serde_json::Value;

use crate::comfyui::download::download_and_save;
use crate::comfyui::prompt::{poll_until_ready, submit_prompt};
use crate::comfyui::workflow::Manifest;
use crate::comfyui::{make_client_id, ComfyuiClient, WorkflowReplacer};
use crate::config::AppConfig;
use crate::prompt_engine::{combine, CombineContext, CombineStrategy};
use crate::tags::{Lang, LangAwarePool};
use crate::theme::Theme;

#[derive(Debug, Args)]
pub struct GenerateArgs {
    #[arg(long, default_value = "portrait")]
    pub theme: String,

    #[arg(long, short = 'l')]
    pub lang: Option<String>,

    #[arg(long)]
    pub strategy: Option<String>,

    #[arg(long)]
    pub max_length: Option<usize>,

    #[arg(long, default_value_t = 0)]
    pub seed: u64,

    #[arg(long, default_value = "default")]
    pub template: String,

    #[arg(long)]
    pub no_send: bool,
}

pub async fn run(args: GenerateArgs, project_root: PathBuf) -> Result<()> {
    let config = AppConfig::load(&project_root.join("config")).context("load config")?;

    let lang = resolve_lang(args.lang.as_deref(), &config)?;
    let strategy = resolve_strategy(args.strategy.as_deref(), &config)?;
    let max_length = args.max_length.unwrap_or(config.prompt.default_max_length);
    let seed = if args.seed == 0 { config.prompt.default_seed } else { args.seed };

    let theme = Theme::load(&config.themes_root(&project_root), &args.theme)
        .with_context(|| format!("load theme '{}'", args.theme))?;
    let mut pool = LangAwarePool::new();
    pool.load_dir(lang, &config.tags_root(&project_root).join(lang.as_str()))
        .with_context(|| "load tags")?;

    let ctx = CombineContext {
        theme,
        lang,
        strategy,
        max_length,
        seed,
        project_root: project_root.clone(),
    };
    let out = combine(&ctx, &pool).context("combine prompt")?;

    println!("{}", out.prompt);
    for (cat, val) in &out.selected {
        tracing::debug!(category = %cat, value = %val, "selected");
    }

    if args.no_send {
        tracing::info!("--no-send specified, skipping ComfyUI submission");
        return Ok(());
    }

    let template_path = config
        .templates_root(&project_root)
        .join(format!("{}.json", args.template));
    if !template_path.exists() {
        anyhow::bail!(
            "template not found: {} (add templates/{}.json)",
            template_path.display(),
            args.template
        );
    }
    let template_text = std::fs::read_to_string(&template_path)
        .with_context(|| format!("read template {}", template_path.display()))?;
    let mut workflow: Value = serde_json::from_str(&template_text)
        .with_context(|| format!("parse template {}", template_path.display()))?;

    let manifest_path = config
        .templates_root(&project_root)
        .join("MANIFEST.toml");
    let manifest = Manifest::load(&manifest_path).with_context(|| {
        format!(
            "load MANIFEST.toml (required to map prompt/seed fields; see {})",
            manifest_path.display()
        )
    })?;
    let entry = manifest
        .get(&args.template)
        .ok_or_else(|| anyhow::anyhow!("template '{}' not in MANIFEST.toml", args.template))?;

    let client_id = make_client_id();
    {
        let mut replacer = WorkflowReplacer::new(&mut workflow);
        replacer
            .replace_text(&entry.positive_prompt_node, &entry.positive_prompt_field, &out.prompt)
            .with_context(|| "replace positive prompt in workflow")?;
        if let (Some(node), Some(field)) = (
            entry.negative_prompt_node.as_deref(),
            entry.negative_prompt_field.as_deref(),
        ) {
            replacer
                .replace_text(node, field, "")
                .with_context(|| "replace negative prompt in workflow")?;
        }
        if let (Some(node), Some(field)) = (
            entry.seed_node.as_deref(),
            entry.seed_field.as_deref(),
        ) {
            let seed_value = if seed == 0 { chrono::Utc::now().timestamp() as i64 } else { seed as i64 };
            replacer
                .replace_int(node, field, seed_value)
                .with_context(|| "replace seed in workflow")?;
        }
    }

    let client = ComfyuiClient::new(&config.comfyui.url).context("init comfyui client")?;
    let prompt_id = submit_prompt(&client, &workflow, &client_id)
        .await
        .with_context(|| "submit to comfyui")?;
    tracing::info!(prompt_id = %prompt_id, "submitted to comfyui");

    let history = poll_until_ready(
        &client,
        &prompt_id,
        Duration::from_secs(config.comfyui.timeout_secs),
        Duration::from_secs(config.comfyui.poll_interval_secs),
    )
    .await
    .with_context(|| "poll comfyui")?;

    let output_root = config.output_root(&project_root);
    let path = download_and_save(&client, &history, &prompt_id, &out.prompt, &output_root)
        .await
        .with_context(|| "download and save image")?;

    println!("{}", path.display());
    Ok(())
}

fn resolve_lang(s: Option<&str>, cfg: &AppConfig) -> Result<Lang> {
    let raw = s.unwrap_or(&cfg.prompt.default_lang);
    Lang::parse(raw).with_context(|| format!("unknown language: {raw}"))
}

fn resolve_strategy(s: Option<&str>, cfg: &AppConfig) -> Result<CombineStrategy> {
    let raw = s.unwrap_or(&cfg.prompt.default_strategy);
    CombineStrategy::parse(raw).with_context(|| format!("unknown strategy: {raw}"))
}
