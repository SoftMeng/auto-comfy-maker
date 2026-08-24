use std::path::{Path, PathBuf};

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

/// 解析"项目根目录"，即 themes / tags / templates / config 任一存在的祖先。
///
/// 解析顺序：
/// 1. 环境变量 `AUTO_COMFY_PROJECT_ROOT`（部署场景下显式指定）；
/// 2. 从 `current_exe()` 向上探测，找到任一标记目录的祖先；
/// 3. 当前工作目录。
fn project_root() -> PathBuf {
    if let Ok(env_root) = std::env::var("AUTO_COMFY_PROJECT_ROOT") {
        let p = PathBuf::from(env_root);
        if !p.as_os_str().is_empty() {
            return p;
        }
    }

    let markers = ["themes", "tags", "templates", "config"];

    if let Ok(exe) = std::env::current_exe() {
        if let Some(resolved) = locate_root_from(&exe, &markers) {
            return resolved;
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        if let Some(resolved) = locate_root_from(&cwd, &markers) {
            return resolved;
        }
    }

    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// 从 `start` 出发，向上逐层查找包含任一 marker 子目录的祖先。
fn locate_root_from(start: &Path, markers: &[&str]) -> Option<PathBuf> {
    let mut cur: Option<&Path> = Some(start);
    let mut visited: Vec<PathBuf> = Vec::new();
    while let Some(dir) = cur {
        for m in markers {
            if dir.join(m).exists() {
                return Some(dir.to_path_buf());
            }
        }
        let next = dir.parent();
        if next.is_none() || visited.iter().any(|p| p == dir) {
            break;
        }
        visited.push(dir.to_path_buf());
        cur = next;
    }
    None
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
