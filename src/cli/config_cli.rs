use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Args, Subcommand};

use crate::config::AppConfig;

#[derive(Debug, Args)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub command: ConfigCommand,
}

#[derive(Debug, Subcommand)]
enum ConfigCommand {
    Show,
    Validate,
}

pub fn run(args: ConfigArgs, project_root: PathBuf) -> Result<()> {
    match args.command {
        ConfigCommand::Show => show(&project_root),
        ConfigCommand::Validate => validate(&project_root),
    }
}

fn show(project_root: &std::path::Path) -> Result<()> {
    let config = AppConfig::load(&project_root.join("config")).context("load config")?;
    println!("{}", toml::to_string_pretty(&config).context("serialize config")?);
    Ok(())
}

fn validate(project_root: &std::path::Path) -> Result<()> {
    let path = project_root.join("config/default.toml");
    match AppConfig::load(&project_root.join("config")) {
        Ok(cfg) => {
            cfg.validate().context("validate semantic fields")?;
            println!("OK: {} (loaded from {})", cfg.app.name, path.display());
            Ok(())
        }
        Err(e) => {
            anyhow::bail!("config invalid: {e}");
        }
    }
}
