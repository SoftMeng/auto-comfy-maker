use anyhow::{anyhow, Result};
use chrono::{DateTime, Local};

pub fn parse_at(s: &str) -> Result<DateTime<Local>> {
    // 接受 RFC3339 与 "YYYY-MM-DD HH:MM:SS"（本地时区）
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Ok(dt.with_timezone(&Local));
    }
    if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
        return naive
            .and_local_timezone(Local)
            .earliest()
            .ok_or_else(|| anyhow!("ambiguous or invalid local time: {s}"));
    }
    if let Ok(naive) = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        let naive_dt = naive.and_hms_opt(0, 0, 0).unwrap();
        return naive_dt
            .and_local_timezone(Local)
            .earliest()
            .ok_or_else(|| anyhow!("ambiguous or invalid local time: {s}"));
    }
    Err(anyhow!(
        "invalid --at time {s:?}; use RFC3339 or 'YYYY-MM-DD HH:MM:SS'"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_rfc3339() {
        let dt = parse_at("2026-09-01T09:00:00+08:00").unwrap();
        assert_eq!(dt.format("%Y-%m-%d %H:%M").to_string(), "2026-09-01 09:00");
    }

    #[test]
    fn parse_local_datetime() {
        let dt = parse_at("2026-09-01 09:00:00").unwrap();
        assert_eq!(dt.format("%Y-%m-%d %H:%M").to_string(), "2026-09-01 09:00");
    }

    #[test]
    fn parse_date_only() {
        let dt = parse_at("2026-09-01").unwrap();
        assert_eq!(dt.format("%Y-%m-%d %H:%M").to_string(), "2026-09-01 00:00");
    }

    #[test]
    fn reject_garbage() {
        assert!(parse_at("not a time").is_err());
        assert!(parse_at("").is_err());
    }
}
