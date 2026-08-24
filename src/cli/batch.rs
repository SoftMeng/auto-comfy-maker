use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Args;

use super::pipeline::{run_pipeline, PipelineOpts};
use crate::config::AppConfig;

#[derive(Debug, Args)]
pub struct BatchArgs {
    #[arg(long, short = 'n', default_value_t = 10)]
    pub count: u32,

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

    #[arg(long, default_value = "anima-aesthetic")]
    pub template: String,

    #[arg(long)]
    pub no_send: bool,

    #[arg(long)]
    pub refine: bool,
}

pub async fn run(args: BatchArgs, project_root: PathBuf) -> Result<()> {
    let config = AppConfig::load(&project_root.join("config")).context("load config")?;

    if args.count == 0 {
        anyhow::bail!("--count must be > 0");
    }

    let mut generated = 0usize;
    let mut failed = 0usize;

    for i in 1..=args.count {
        // 每次迭代换 seed，保证组合不同（batch 语义：N 组不同随机 tags）
        let seed = if args.seed == 0 {
            chrono::Utc::now().timestamp_subsec_nanos() as u64 + i as u64
        } else {
            args.seed.wrapping_add(i as u64)
        };

        let opts = PipelineOpts {
            theme_name: args.theme.clone(),
            lang: args.lang.clone(),
            strategy: args.strategy.clone(),
            max_length: args.max_length,
            seed,
            template: args.template.clone(),
            no_send: args.no_send,
            use_refine: args.refine,
            width: None,
            height: None,
        };

        match run_pipeline(&opts, &config, &project_root).await {
            Ok(outcome) => {
                generated += 1;
                if outcome.final_prompt != outcome.combine_prompt {
                    println!("[{}/{}] [refined] {}", i, args.count, outcome.final_prompt);
                } else {
                    println!("[{}/{}] {}", i, args.count, outcome.final_prompt);
                }
                if let Some(p) = &outcome.image_path {
                    println!("    {}", p.display());
                }
            }
            Err(e) => {
                failed += 1;
                tracing::error!(index = i, error = %e, "batch item failed");
                eprintln!("[{}/{}] FAILED: {:#}", i, args.count, e);
            }
        }
    }

    println!("batch done: {} generated, {} failed", generated, failed);
    if failed > 0 && generated == 0 {
        anyhow::bail!("all {} batch items failed", failed);
    }
    Ok(())
}
