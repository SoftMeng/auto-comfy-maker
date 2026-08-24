use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Args, Subcommand};

use crate::config::AppConfig;
use crate::tags::Lang;

#[derive(Debug, Args)]
pub struct TagsArgs {
    #[command(subcommand)]
    pub command: TagsCommand,

    #[arg(long, short = 'l')]
    pub lang: Option<String>,
}

#[derive(Debug, Subcommand)]
pub enum TagsCommand {
    List,
    Show { category: String },
    Add { category: String, tag: String },
    Remove { category: String, tag: String },
}

pub fn run(args: TagsArgs, project_root: PathBuf) -> Result<()> {
    let config = AppConfig::load(&project_root.join("config")).context("load config")?;
    let lang_str = args.lang.as_deref().unwrap_or(&config.prompt.default_lang);
    let lang = Lang::parse(lang_str)
        .with_context(|| format!("unknown language: {lang_str}"))?;
    let tags_dir = config.tags_root(&project_root).join(lang.as_str());

    match args.command {
        TagsCommand::List => list(&tags_dir),
        TagsCommand::Show { category } => show(&tags_dir, &category),
        TagsCommand::Add { category, tag } => add(&tags_dir, &category, &tag),
        TagsCommand::Remove { category, tag } => remove(&tags_dir, &category, &tag),
    }
}

fn list(dir: &std::path::Path) -> Result<()> {
    if !dir.exists() {
        anyhow::bail!("tags dir not found: {}", dir.display());
    }
    let mut entries: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("txt"))
        .collect();
    entries.sort_by_key(|e| e.file_name());

    println!("{:<20} {:>6}  {}", "CATEGORY", "COUNT", "FILE");
    for e in entries {
        let path = e.path();
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("?");
        let count = std::fs::read_to_string(&path)?
            .lines()
            .filter(|l| {
                let t = l.trim();
                !t.is_empty() && !t.starts_with('#')
            })
            .count();
        println!("{:<20} {:>6}  {}", stem, count, path.display());
    }
    Ok(())
}

fn show(dir: &std::path::Path, category: &str) -> Result<()> {
    let path = dir.join(format!("{category}.txt"));
    if !path.exists() {
        anyhow::bail!("category '{category}' not found: {}", path.display());
    }
    for line in std::fs::read_to_string(&path)?.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        println!("{t}");
    }
    Ok(())
}

fn add(dir: &std::path::Path, category: &str, tag: &str) -> Result<()> {
    let path = dir.join(format!("{category}.txt"));
    let existing = if path.exists() {
        std::fs::read_to_string(&path)?
    } else {
        String::new()
    };
    let already = existing
        .lines()
        .any(|l| l.trim() == tag);
    if already {
        println!("already exists: {category}/{tag}");
        return Ok(());
    }
    let mut new_content = existing;
    if !new_content.is_empty() && !new_content.ends_with('\n') {
        new_content.push('\n');
    }
    new_content.push_str(tag);
    new_content.push('\n');
    std::fs::write(&path, &new_content).with_context(|| format!("write {}", path.display()))?;
    println!("added: {category}/{tag}");
    Ok(())
}

fn remove(dir: &std::path::Path, category: &str, tag: &str) -> Result<()> {
    let path = dir.join(format!("{category}.txt"));
    if !path.exists() {
        anyhow::bail!("category '{category}' not found: {}", path.display());
    }
    let existing = std::fs::read_to_string(&path)?;
    let filtered: Vec<&str> = existing
        .lines()
        .filter(|l| l.trim() != tag)
        .collect();
    if filtered.len() == existing.lines().count() {
        println!("not found: {category}/{tag}");
        return Ok(());
    }
    let mut out = filtered.join("\n");
    if !out.is_empty() {
        out.push('\n');
    }
    std::fs::write(&path, &out).with_context(|| format!("write {}", path.display()))?;
    println!("removed: {category}/{tag}");
    Ok(())
}
