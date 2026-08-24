use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result};
use clap::Args;

use super::pipeline::{run_fixed_prompt, run_pipeline, PipelineOpts};
use crate::config::AppConfig;
use crate::scheduler::parse_cron;
use crate::scheduler::persist::{
    load_jobs, mark_completed, mark_failed, save_jobs, JobRecord, JobStatus,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum DaemonMode {
    Fixed,
    Auto,
}

#[derive(Debug, Args)]
pub struct DaemonArgs {
    /// 固定间隔（如 30s / 5m / 2h），与 --cron/--at 互斥
    #[arg(long, short = 'i', conflicts_with_all = ["cron", "at"])]
    pub interval: Option<String>,

    /// cron 表达式（分 时 日 月 周），与 --interval/--at 互斥
    #[arg(long, conflicts_with_all = ["interval", "at"])]
    pub cron: Option<String>,

    /// 具体时刻（RFC3339 或 YYYY-MM-DD HH:MM:SS），可多次；与 --interval/--cron 互斥
    #[arg(long = "at", value_name = "TIME", conflicts_with_all = ["interval", "cron"])]
    pub at: Vec<String>,

    #[arg(long, short = 'm')]
    pub mode: DaemonMode,

    /// fixed 模式：固定 prompt 文本
    #[arg(long, required_if_eq("mode", "fixed"))]
    pub prompt: Option<String>,

    /// fixed 模式：从文件加载 prompt（与 --prompt 互斥）
    #[arg(long, conflicts_with = "prompt")]
    pub prompt_file: Option<PathBuf>,

    /// auto 模式使用的 theme
    #[arg(long, default_value = "portrait")]
    pub theme: String,

    #[arg(long, short = 'l')]
    pub lang: Option<String>,

    #[arg(long, default_value_t = 1)]
    pub count_per_tick: u32,

    #[arg(long, default_value = "default")]
    pub template: String,

    #[arg(long)]
    pub no_send: bool,

    #[arg(long)]
    pub refine: bool,

    #[arg(long, default_value = "config/schedule.toml")]
    pub persist: PathBuf,
}

enum Trigger {
    Interval(Duration),
    Cron(crate::scheduler::CronExpr),
    At(Vec<chrono::DateTime<chrono::Local>>),
}

pub async fn run(args: DaemonArgs, project_root: PathBuf) -> Result<()> {
    let config = AppConfig::load(&project_root.join("config")).context("load config")?;

    let trigger = build_trigger(&args)?;

    let fixed_prompt = match args.mode {
        DaemonMode::Fixed => Some(load_fixed_prompt(&args)?),
        DaemonMode::Auto => None,
    };
    if args.mode == DaemonMode::Auto && (args.prompt.is_some() || args.prompt_file.is_some()) {
        anyhow::bail!("--mode auto 禁止提供 --prompt / --prompt-file");
    }

    let persist_path = if args.persist.is_absolute() {
        args.persist.clone()
    } else {
        project_root.join(&args.persist)
    };

    println!(
        "daemon started: mode={:?}, trigger={}",
        args.mode,
        describe_trigger(&trigger)
    );
    println!("persist: {}", persist_path.display());
    println!("press Ctrl-C to stop");

    let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .context("install SIGTERM handler")?;

    let mut tick_interval = tokio::time::interval(Duration::from_secs(1));
    tick_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    let mut at_queue: Vec<chrono::DateTime<chrono::Local>> = match &trigger {
        Trigger::At(v) => {
            let mut q = v.clone();
            q.sort();
            q
        }
        _ => Vec::new(),
    };

    let mut last_fired: Option<(String, chrono::DateTime<chrono::Local>)> = None;

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                println!("\nreceived Ctrl-C, exiting");
                break;
            }
            _ = sigterm.recv() => {
                println!("\nreceived SIGTERM, exiting");
                break;
            }
            _ = tick_interval.tick() => {
                let now = chrono::Local::now();
                let should_fire = match &trigger {
                    Trigger::Interval(d) => {
                        match last_fired.as_ref() {
                            Some((_, t)) => {
                                now.signed_duration_since(*t).to_std().unwrap_or_default() >= *d
                            }
                            _ => true,
                        }
                    }
                    Trigger::Cron(e) => e.matches(&now),
                    Trigger::At(_) => {
                        !at_queue.is_empty() && now >= at_queue[0]
                    }
                };

                if !should_fire {
                    continue;
                }

                // at 模式：弹出已到时刻
                if let Trigger::At(_) = &trigger {
                    while !at_queue.is_empty() && now >= at_queue[0] {
                        at_queue.remove(0);
                    }
                }

                // interval 模式记录上次触发时间（cron 按 1s 轮询匹配）
                if let Trigger::Interval(_) = &trigger {
                    last_fired = Some(("interval".to_string(), now));
                }

                if let Err(e) = run_tick(
                    &args,
                    &config,
                    &project_root,
                    &persist_path,
                    fixed_prompt.as_deref(),
                    now,
                )
                .await
                {
                    tracing::error!(error = %e, "tick failed");
                    eprintln!("tick failed: {:#}", e);
                }

                if at_queue.is_empty() {
                    if let Trigger::At(_) = &trigger {
                        println!("all --at times consumed, exiting");
                        break;
                    }
                }
            }
        }
    }

    println!("daemon stopped");
    Ok(())
}

async fn run_tick(
    args: &DaemonArgs,
    config: &AppConfig,
    project_root: &std::path::Path,
    persist_path: &std::path::Path,
    fixed_prompt: Option<&str>,
    now: chrono::DateTime<chrono::Local>,
) -> Result<()> {
    let tick_id = format!("tick-{}", now.format("%Y%m%d-%H%M%S"));

    for i in 0..args.count_per_tick {
        let record = JobRecord {
            id: format!("{tick_id}-{i}"),
            scheduled_at: now.to_rfc3339(),
            status: JobStatus::Running,
            mode: match args.mode {
                DaemonMode::Fixed => "fixed".to_string(),
                DaemonMode::Auto => "auto".to_string(),
            },
            prompt: fixed_prompt.map(|s| s.to_string()),
            completed_at: None,
            output_path: None,
        };

        let mut file = load_jobs(persist_path).context("load schedule")?;
        file.jobs.push(record.clone());
        save_jobs(persist_path, &file).context("save schedule")?;

        let result: std::result::Result<Option<String>, anyhow::Error> = match args.mode {
            DaemonMode::Fixed => {
                let p = fixed_prompt.expect("fixed mode has prompt");
                if args.no_send {
                    println!("[{}] {}", record.id, p);
                    Ok(None)
                } else {
                    run_fixed_prompt(p, &args.template, 0, config, project_root)
                        .await
                        .map(|path| Some(path.display().to_string()))
                }
            }
            DaemonMode::Auto => {
                let opts = PipelineOpts {
                    theme_name: args.theme.clone(),
                    lang: args.lang.clone(),
                    strategy: None,
                    max_length: None,
                    seed: chrono::Utc::now().timestamp_subsec_nanos() as u64 + i as u64,
                    template: args.template.clone(),
                    no_send: args.no_send,
                    use_refine: args.refine,
                    width: None,
                    height: None,
                };
                match run_pipeline(&opts, config, project_root).await {
                    Ok(outcome) => {
                        println!("[{}] {}", record.id, outcome.final_prompt);
                        if let Some(p) = &outcome.image_path {
                            println!("    {}", p.display());
                        }
                        Ok(outcome.image_path.map(|p| p.display().to_string()))
                    }
                    Err(e) => Err(e),
                }
            }
        };

        let mut file = load_jobs(persist_path).context("reload schedule")?;
        match result {
            Ok(output) => {
                mark_completed(&mut file, &record.id, output);
            }
            Err(ref e) => {
                tracing::error!(job = %record.id, error = %e, "job failed");
                mark_failed(&mut file, &record.id);
            }
        }
        save_jobs(persist_path, &file).context("save schedule after job")?;
    }
    Ok(())
}

fn build_trigger(args: &DaemonArgs) -> Result<Trigger> {
    if let Some(iv) = &args.interval {
        return Ok(Trigger::Interval(
            crate::scheduler::interval::parse_duration(iv)?,
        ));
    }
    if let Some(c) = &args.cron {
        return Ok(Trigger::Cron(parse_cron(c)?));
    }
    if !args.at.is_empty() {
        let mut times = Vec::new();
        for t in &args.at {
            times.push(crate::scheduler::at::parse_at(t)?);
        }
        return Ok(Trigger::At(times));
    }
    anyhow::bail!("must specify one of --interval / --cron / --at");
}

fn describe_trigger(t: &Trigger) -> String {
    match t {
        Trigger::Interval(d) => format!("every {:?}", d),
        Trigger::Cron(e) => format!("cron '{}'", e),
        Trigger::At(v) => format!("at {} time(s)", v.len()),
    }
}

fn load_fixed_prompt(args: &DaemonArgs) -> Result<String> {
    if let Some(p) = &args.prompt {
        return Ok(p.clone());
    }
    if let Some(f) = &args.prompt_file {
        let text = std::fs::read_to_string(f)
            .with_context(|| format!("read prompt file {}", f.display()))?;
        return Ok(text.trim().to_string());
    }
    anyhow::bail!("fixed mode requires --prompt or --prompt-file");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_at_list() {
        let a = crate::scheduler::at::parse_at("2026-09-01 09:00:00").unwrap();
        assert_eq!(a.format("%H").to_string(), "09");
    }

    #[test]
    fn describe_trigger_formats() {
        let t = Trigger::Interval(Duration::from_secs(60));
        assert!(describe_trigger(&t).contains("60s"));
    }
}
