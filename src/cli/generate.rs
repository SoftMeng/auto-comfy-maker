use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Args;

use super::pipeline::{run_pipeline, PipelineOpts};
use crate::config::AppConfig;

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

    #[arg(long, default_value = "zimage")]
    pub template: String,

    #[arg(long)]
    pub no_send: bool,

    #[arg(long)]
    pub refine: bool,

    #[arg(long)]
    pub width: Option<u32>,

    #[arg(long)]
    pub height: Option<u32>,
}

pub async fn run(args: GenerateArgs, project_root: PathBuf) -> Result<()> {
    let config = AppConfig::load(&project_root.join("config")).context("load config")?;

    let opts = PipelineOpts {
        theme_name: args.theme,
        lang: args.lang,
        strategy: args.strategy,
        max_length: args.max_length,
        seed: args.seed,
        template: args.template,
        no_send: args.no_send,
        use_refine: args.refine,
        width: args.width,
        height: args.height,
    };

    let outcome = run_pipeline(&opts, &config, &project_root).await?;

    if outcome.final_prompt != outcome.combine_prompt {
        println!("[refined] {}", outcome.final_prompt);
    } else {
        println!("{}", outcome.final_prompt);
    }
    if let Some(p) = &outcome.image_path {
        println!("{}", p.display());
    }
    Ok(())
}
