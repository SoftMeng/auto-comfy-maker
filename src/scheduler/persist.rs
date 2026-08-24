use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobRecord {
    pub id: String,
    pub scheduled_at: String,
    pub status: JobStatus,
    pub mode: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_path: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JobFile {
    #[serde(default)]
    pub jobs: Vec<JobRecord>,
}

pub fn load_jobs(path: &Path) -> Result<JobFile> {
    if !path.exists() {
        return Ok(JobFile::default());
    }
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("read {}", path.display()))?;
    if text.trim().is_empty() {
        return Ok(JobFile::default());
    }
    toml::from_str(&text).with_context(|| format!("parse {}", path.display()))
}

pub fn save_jobs(path: &Path, file: &JobFile) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create parent of {}", path.display()))?;
    }
    let text = toml::to_string_pretty(file).context("serialize jobs")?;

    // atomic write: 先写临时文件再 rename，避免半写状态
    let tmp: PathBuf = path.with_extension("toml.tmp");
    std::fs::write(&tmp, &text).with_context(|| format!("write tmp {}", tmp.display()))?;
    std::fs::rename(&tmp, path)
        .with_context(|| format!("rename {} -> {}", tmp.display(), path.display()))?;
    Ok(())
}

pub fn mark_completed(file: &mut JobFile, id: &str, output: Option<String>) -> bool {
    for job in file.jobs.iter_mut() {
        if job.id == id {
            job.status = JobStatus::Completed;
            job.completed_at = Some(Local::now().to_rfc3339());
            job.output_path = output;
            return true;
        }
    }
    false
}

pub fn mark_failed(file: &mut JobFile, id: &str) -> bool {
    for job in file.jobs.iter_mut() {
        if job.id == id {
            job.status = JobStatus::Failed;
            job.completed_at = Some(Local::now().to_rfc3339());
            return true;
        }
    }
    false
}

use chrono::Local;

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(id: &str) -> JobRecord {
        JobRecord {
            id: id.into(),
            scheduled_at: "2026-09-01T09:00:00+08:00".into(),
            status: JobStatus::Pending,
            mode: "auto".into(),
            prompt: None,
            completed_at: None,
            output_path: None,
        }
    }

    #[test]
    fn load_missing_returns_empty() {
        let f = load_jobs(Path::new("/nonexistent/schedule.toml")).unwrap();
        assert!(f.jobs.is_empty());
    }

    #[test]
    fn save_then_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("schedule.toml");
        let mut f = JobFile::default();
        f.jobs.push(sample("a"));
        f.jobs.push(sample("b"));
        save_jobs(&p, &f).unwrap();

        let loaded = load_jobs(&p).unwrap();
        assert_eq!(loaded.jobs.len(), 2);
        assert_eq!(loaded.jobs[0].id, "a");
        assert_eq!(loaded.jobs[0].status, JobStatus::Pending);
    }

    #[test]
    fn atomic_write_leaves_no_tmp() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("schedule.toml");
        save_jobs(&p, &JobFile::default()).unwrap();
        assert!(!dir.path().join("schedule.toml.tmp").exists());
        assert!(p.exists());
    }

    #[test]
    fn mark_completed_updates_status() {
        let mut f = JobFile::default();
        f.jobs.push(sample("x"));
        assert!(mark_completed(&mut f, "x", Some("out.png".into())));
        assert_eq!(f.jobs[0].status, JobStatus::Completed);
        assert_eq!(f.jobs[0].output_path.as_deref(), Some("out.png"));
    }

    #[test]
    fn mark_failed_updates_status() {
        let mut f = JobFile::default();
        f.jobs.push(sample("y"));
        assert!(mark_failed(&mut f, "y"));
        assert_eq!(f.jobs[0].status, JobStatus::Failed);
    }

    #[test]
    fn mark_unknown_id_returns_false() {
        let mut f = JobFile::default();
        assert!(!mark_completed(&mut f, "nope", None));
    }
}
