use std::time::Duration;

use anyhow::{anyhow, Result};

pub fn parse_duration(s: &str) -> Result<Duration> {
    let s = s.trim();
    let (num_part, unit_part) = s.split_at(s.len().saturating_sub(1));
    let n: u64 = num_part
        .parse()
        .map_err(|_| anyhow!("invalid duration number: {num_part:?}"))?;
    let secs = match unit_part {
        "s" => n,
        "m" => n * 60,
        "h" => n * 3600,
        _ => return Err(anyhow!("unknown duration unit: {unit_part:?} (use s/m/h)")),
    };
    Ok(Duration::from_secs(secs))
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
