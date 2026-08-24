use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

mod cli;
mod comfyui;
mod config;
mod prompt_engine;
mod scheduler;
mod tags;
mod theme;

#[derive(Debug, Parser)]
#[command(name = "auto-comfy-maker", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Generate(cli::GenerateArgs),
    Batch(cli::BatchArgs),
    Daemon(cli::DaemonArgs),
    Tags(cli::TagsArgs),
    Config(cli::ConfigArgs),
}

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();
    let root = project_root();

    match cli.command {
        Commands::Generate(args) => cli::run_generate(args, root).await.context("generate")?,
        Commands::Batch(args) => cli::run_batch(args, root).await.context("batch")?,
        Commands::Daemon(args) => cli::run_daemon(args, root).await.context("daemon")?,
        Commands::Tags(args) => cli::run_tags(args, root).context("tags")?,
        Commands::Config(args) => cli::run_config(args, root).context("config")?,
    }
    Ok(())
}
