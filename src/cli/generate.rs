use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Args;

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
}

pub fn run(args: GenerateArgs, project_root: PathBuf) -> Result<()> {
    let config = AppConfig::load(&project_root.join("config"))
        .context("load config")?;

    let lang_str = args
        .lang
        .as_deref()
        .unwrap_or(&config.prompt.default_lang);
    let lang = Lang::parse(lang_str)
        .with_context(|| format!("unknown language: {lang_str}"))?;

    let strategy_str = args
        .strategy
        .as_deref()
        .unwrap_or(&config.prompt.default_strategy);
    let strategy = CombineStrategy::parse(strategy_str)
        .with_context(|| format!("unknown strategy: {strategy_str}"))?;

    let max_length = args
        .max_length
        .unwrap_or(config.prompt.default_max_length);
    let seed = if args.seed == 0 {
        config.prompt.default_seed
    } else {
        args.seed
    };

    let themes_dir = config.themes_root(&project_root);
    let theme = Theme::load(&themes_dir, &args.theme)
        .with_context(|| format!("load theme '{}'", args.theme))?;

    let tags_root = config.tags_root(&project_root);
    let mut pool = LangAwarePool::new();
    pool.load_dir(lang, &tags_root.join(lang.as_str()))
        .with_context(|| format!("load tags dir {:?}", tags_root.join(lang.as_str())))?;

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
    Ok(())
}
