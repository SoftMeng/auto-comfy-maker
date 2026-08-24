use std::fmt;

use anyhow::{anyhow, Result};

/// 5 字段 cron：分 时 日 月 周
/// 支持: * 数字 逗号列表 连续区间 步长 (*/)
/// 不支持: ? L W # （Quartz 扩展）
#[derive(Debug, Clone, PartialEq)]
pub struct CronExpr {
    minutes: FieldSet,
    hours: FieldSet,
    days: FieldSet,
    months: FieldSet,
    weekdays: FieldSet,
}

#[derive(Debug, Clone, PartialEq)]
struct FieldSet {
    values: Vec<u32>,
}

impl FieldSet {
    fn contains(&self, v: u32) -> bool {
        self.values.contains(&v)
    }
}

impl fmt::Display for CronExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {} {} {} {}",
            fmt_field(&self.minutes),
            fmt_field(&self.hours),
            fmt_field(&self.days),
            fmt_field(&self.months),
            fmt_field(&self.weekdays)
        )
    }
}

fn fmt_field(fs: &FieldSet) -> String {
    if fs.values.len() == fs.values.capacity() && fs.values.first() == Some(&0) {
        return "*".to_string();
    }
    fs.values.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(",")
}

pub fn parse_cron(expr: &str) -> Result<CronExpr> {
    let parts: Vec<&str> = expr.split_whitespace().collect();
    if parts.len() != 5 {
        return Err(anyhow!(
            "cron must have 5 fields (min hour day month weekday), got {}",
            parts.len()
        ));
    }
    Ok(CronExpr {
        minutes: parse_field(parts[0], 0, 59)?,
        hours: parse_field(parts[1], 0, 23)?,
        days: parse_field(parts[2], 1, 31)?,
        months: parse_field(parts[3], 1, 12)?,
        weekdays: parse_field(parts[4], 0, 6)?,
    })
}

impl CronExpr {
    pub fn matches(&self, t: &chrono::DateTime<chrono::Local>) -> bool {
        use chrono::{Datelike, Timelike};
        self.minutes.contains(t.minute())
            && self.hours.contains(t.hour())
            && self.days.contains(t.day())
            && self.months.contains(t.month())
            && self.weekdays.contains(t.weekday().num_days_from_sunday())
    }
}

fn parse_field(s: &str, min: u32, max: u32) -> Result<FieldSet> {
    let mut values: Vec<u32> = Vec::new();
    for piece in s.split(',') {
        let (range_part, step) = match piece.split_once('/') {
            Some((r, st)) => {
                let step: u32 = st
                    .parse()
                    .map_err(|_| anyhow!("invalid step in {piece:?}"))?;
                if step == 0 {
                    return Err(anyhow!("step must be > 0 in {piece:?}"));
                }
                (r, step)
            }
            None => (piece, 1),
        };
        let (lo, hi) = if range_part == "*" {
            (min, max)
        } else if let Some((a, b)) = range_part.split_once('-') {
            let a: u32 = a
                .parse()
                .map_err(|_| anyhow!("invalid range start in {piece:?}"))?;
            let b: u32 = b
                .parse()
                .map_err(|_| anyhow!("invalid range end in {piece:?}"))?;
            (a, b)
        } else {
            let n: u32 = range_part
                .parse()
                .map_err(|_| anyhow!("invalid number in {piece:?}"))?;
            (n, n)
        };
        if lo < min || hi > max || lo > hi {
            return Err(anyhow!(
                "field {piece:?} out of range [{min}, {max}] or reversed"
            ));
        }
        let mut v = lo;
        while v <= hi {
            values.push(v);
            v += step;
        }
    }
    values.sort_unstable();
    values.dedup();
    Ok(FieldSet { values })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matches(expr: &str, y: i32, mo: u32, d: u32, h: u32, mi: u32, wd: u32) -> bool {
        let e = parse_cron(expr).unwrap();
        use chrono::{Datelike, Duration as _, TimeZone};
        let dt = chrono::Local
            .with_ymd_and_hms(y, mo, d, h, mi, 0)
            .earliest()
            .expect("valid date");
        assert_eq!(dt.weekday().num_days_from_sunday(), wd, "test weekday mismatch");
        e.matches(&dt)
    }

    #[test]
    fn every_15_min() {
        assert!(matches("*/15 * * * *", 2026, 8, 24, 10, 0, 1));
        assert!(matches("*/15 * * * *", 2026, 8, 24, 10, 45, 1));
        assert!(!matches("*/15 * * * *", 2026, 8, 24, 10, 20, 1));
    }

    #[test]
    fn daily_at_9() {
        assert!(matches("0 9 * * *", 2026, 8, 24, 9, 0, 1));
        assert!(!matches("0 9 * * *", 2026, 8, 24, 10, 0, 1));
        assert!(!matches("0 9 * * *", 2026, 8, 24, 9, 30, 1));
    }

    #[test]
    fn weekly_monday() {
        // 2026-08-24 是周一（wd=1）
        assert!(matches("0 0 * * 1", 2026, 8, 24, 0, 0, 1));
        assert!(!matches("0 0 * * 1", 2026, 8, 25, 0, 0, 2));
    }

    #[test]
    fn range_and_list() {
        assert!(matches("1-5 * * * *", 2026, 8, 24, 10, 3, 1));
        assert!(!matches("1-5 * * * *", 2026, 8, 24, 10, 6, 1));
        assert!(matches("0,30 * * * *", 2026, 8, 24, 10, 30, 1));
    }

    #[test]
    fn reject_invalid() {
        assert!(parse_cron("* * * *").is_err());
        assert!(parse_cron("* * * * * *").is_err());
        assert!(parse_cron("60 * * * *").is_err());
        assert!(parse_cron("*/0 * * * *").is_err());
        assert!(parse_cron("5-1 * * * *").is_err());
        assert!(parse_cron("? * * * *").is_err());
    }
}
