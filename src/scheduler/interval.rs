use std::time::Duration;

use anyhow::{anyhow, Result};

#[derive(Debug, Clone, Copy)]
pub struct IntervalSpec {
    pub every: Duration,
}

pub fn parse_duration(s: &str) -> Result<Duration> {
    let s = s.trim();
    let (num_part, unit_part) = s.split_at(s.len().saturating_sub(1));
    let n: u64 = num_part
        .parse()
        .map_err(|_| anyhow!("invalid duration number: {num_part:?}"))?;
    match unit_part {
        "s" => Ok(Duration::from_secs(n)),
        "m" => Ok(Duration::from_mins(n)),
        "h" => Ok(Duration::from_hours(n)),
        _ => Err(anyhow!("unknown duration unit: {unit_part:?} (use s/m/h)")),
    }
}

trait DurationExt {
    fn from_mins(mins: u64) -> Duration;
    fn from_hours(hours: u64) -> Duration;
}

impl DurationExt for Duration {
    fn from_mins(mins: u64) -> Duration {
        Duration::from_secs(mins * 60)
    }
    fn from_hours(hours: u64) -> Duration {
        Duration::from_secs(hours * 3600)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple() {
        assert_eq!(parse_duration("30s").unwrap(), Duration::from_secs(30));
        assert_eq!(parse_duration("5m").unwrap(), Duration::from_secs(300));
        assert_eq!(parse_duration("2h").unwrap(), Duration::from_secs(7200));
    }

    #[test]
    fn parse_rejects_garbage() {
        assert!(parse_duration("abc").is_err());
        assert!(parse_duration("5x").is_err());
        assert!(parse_duration("").is_err());
    }
}
