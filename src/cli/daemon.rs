use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use clap::Args;
use tokio::sync::Notify;

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
    /// tick 触发周期（如 5m / 2h），与 --cron/--at 互斥。
    /// 计时从 tick 完成开始，确保每个 tick 至少间隔该时间。
    #[arg(long, short = 'i', conflicts_with_all = ["cron", "at"])]
    pub interval: Option<String>,

    /// cron 表达式（分 时 日 月 周），与 --interval/--at 互斥
    #[arg(long, conflicts_with_all = ["interval", "at"])]
    pub cron: Option<String>,

    /// 具体时刻（RFC3339 或 YYYY-MM-DD HH:MM:SS），可多次；与 --interval/--cron 互斥
    #[arg(long = "at", value_name = "TIME", conflicts_with_all = ["interval", "cron"])]
    pub at: Vec<String>,

    /// 任务之间等待的间隔（如 5s / 1m），持续生成模式。
    /// 单用合法（与三选一触发器互斥）；每个 tick 完成后等待指定时长再生成下一个。
    #[arg(long, value_name = "DURATION", conflicts_with_all = ["interval", "cron", "at"])]
    pub task_interval: Option<String>,

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

    #[arg(long, default_value = "anima-aesthetic")]
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
    /// 持续模式：每完成一个任务后等待指定时长，然后立即执行下一个
    /// （无固定周期，靠 --task-interval 控制节奏）
    Continuous(Duration),
}

pub async fn run(args: DaemonArgs, project_root: PathBuf) -> Result<()> {
    let config = AppConfig::load(&project_root.join("config")).context("load config")?;

    let trigger = build_trigger(&args)?;
    println!(
        "daemon started: mode={:?}, trigger={}",
        args.mode,
        describe_trigger(&trigger)
    );

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

    // Signal abort handle shared between the outer select and run_tick.
    // notify_waiters wakes all .notified() awaiters immediately.
    let abort = Arc::new(Notify::new());
    // Loop-wide shutdown flag: set by any signal path or interrupted select.
    let shutdown = Arc::new(AtomicBool::new(false));

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
        if shutdown.load(Ordering::SeqCst) {
            break;
        }
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                println!("\nreceived Ctrl-C, exiting");
                shutdown.store(true, Ordering::SeqCst);
                abort.notify_waiters();
            }
            _ = sigterm.recv() => {
                println!("\nreceived SIGTERM, exiting");
                shutdown.store(true, Ordering::SeqCst);
                abort.notify_waiters();
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
                    // Continuous 模式：每秒都触发（节奏由外层 task-interval 等待控制）
                    Trigger::Continuous(_) => true,
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

                let task_interval = match &trigger {
                    Trigger::Continuous(d) => Some(*d),
                    _ => None,
                };

                // 二级 select：让 abort signal 能中断 tick 内部的 poll/download 等待
                tokio::select! {
                    _ = abort.notified() => {
                        println!("\ninterrupted during tick, exiting");
                        shutdown.store(true, Ordering::SeqCst);
                    }
                    res = run_tick(
                        &args,
                        &config,
                        &project_root,
                        &persist_path,
                        fixed_prompt.as_deref(),
                        now,
                        task_interval,
                    ) => {
                        if let Err(e) = res {
                            tracing::error!(error = %e, "tick failed");
                            eprintln!("tick failed: {:#}", e);
                        }
                    }
                }

                // 间隔结束后 break 出内层 select，外层 loop 重置 tick_interval、记录 last_fired 等

                // Continuous 模式：每个 tick 完成后等 task-interval 再开下一个
                if let Trigger::Continuous(d) = &trigger {
                    println!("waiting {:?} before next task...", d);
                    tokio::select! {
                        _ = abort.notified() => {
                            shutdown.store(true, Ordering::SeqCst);
                        }
                        _ = tokio::time::sleep(*d) => {}
                    }
                }

                // interval 模式：在 tick 完成后记录，避免任务耗时被计入
                // （这样 --interval 5m 表示"从 tick 完成到下一个 tick 触发至少 5m"）
                if let Trigger::Interval(_) = &trigger {
                    last_fired = Some(("interval".to_string(), chrono::Local::now()));
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
    task_interval: Option<Duration>,
) -> Result<()> {
    let tick_id = format!("tick-{}", now.format("%Y%m%d-%H%M%S"));

    for i in 0..args.count_per_tick {
        // count_per_tick > 1 时，多张图之间间隔 task-interval
        // （tick 之间的间隔在外层 loop 控制，这里只管一个 tick 内部）
        if i > 0 {
            if let Some(d) = task_interval {
                println!("waiting {:?} before next task...", d);
                tokio::time::sleep(d).await;
            }
        }

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
                    run_fixed_prompt(p, &args.template, 0, args.lang.as_deref(), config, project_root)
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
    if let Some(iv) = &args.task_interval {
        return Ok(Trigger::Continuous(
            crate::scheduler::interval::parse_duration(iv)?,
        ));
    }
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
    anyhow::bail!("must specify one of --interval / --cron / --at / --task-interval");
}

fn describe_trigger(t: &Trigger) -> String {
    match t {
        Trigger::Interval(d) => format!("every {:?}", d),
        Trigger::Cron(e) => format!("cron '{}'", e),
        Trigger::At(v) => format!("at {} time(s)", v.len()),
        Trigger::Continuous(d) => format!("continuous, task-interval {:?}", d),
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

    #[test]
    fn parse_task_interval() {
        let d = crate::scheduler::interval::parse_duration("10s").unwrap();
        assert_eq!(d, Duration::from_secs(10));
        let d = crate::scheduler::interval::parse_duration("1m").unwrap();
        assert_eq!(d, Duration::from_secs(60));
    }

    /// 校验四个调度参数的 clap 互斥矩阵。
    /// 期望：interval / cron / at 三者互斥；task-interval 单用合法；
    /// task-interval 与 interval/cron/at 仍冲突（不能与三者叠加）。
    use clap::Parser;

    #[derive(Parser, Debug)]
    struct Wrap {
        #[command(flatten)]
        args: DaemonArgs,
    }

    fn parse_ok(args: &[&str]) -> Result<Wrap, clap::Error> {
        Wrap::try_parse_from(args)
    }

    #[test]
    fn task_interval_alone_is_allowed() {
        assert!(parse_ok(&["x", "--mode", "fixed", "--prompt", "p", "--task-interval", "5s"]).is_ok());
    }

    #[test]
    fn interval_with_task_interval_rejected() {
        let r = parse_ok(&[
            "x", "--mode", "fixed", "--prompt", "p",
            "--interval", "1m", "--task-interval", "5s",
        ]);
        assert!(r.is_err(), "expected interval+task-interval to be rejected");
    }

    #[test]
    fn interval_with_cron_rejected() {
        let r = parse_ok(&[
            "x", "--mode", "fixed", "--prompt", "p",
            "--interval", "1m", "--cron", "0 * * * *",
        ]);
        assert!(r.is_err());
    }

    #[test]
    fn interval_with_at_rejected() {
        let r = parse_ok(&[
            "x", "--mode", "fixed", "--prompt", "p",
            "--interval", "1m", "--at", "2026-09-01T09:00:00+00:00",
        ]);
        assert!(r.is_err());
    }

    #[test]
    fn cron_with_task_interval_rejected() {
        let r = parse_ok(&[
            "x", "--mode", "fixed", "--prompt", "p",
            "--cron", "0 * * * *", "--task-interval", "5s",
        ]);
        assert!(r.is_err());
    }
}
