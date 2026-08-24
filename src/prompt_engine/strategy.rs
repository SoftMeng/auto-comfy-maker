use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PromptError {
    #[error("unknown language: {0}")]
    UnknownLang(String),
    #[error("unknown category: {0}")]
    UnknownCategory(String),
    #[error("category has no usable elements: {0}")]
    EmptyCategory(String),
    #[error("invalid file path in theme: {0}")]
    InvalidFile(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CombineStrategy {
    Comma,
    Newline,
    Natural,
}

impl CombineStrategy {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "comma" => Some(Self::Comma),
            "newline" => Some(Self::Newline),
            "natural" => Some(Self::Natural),
            _ => None,
        }
    }

    pub fn join(&self, items: &[String]) -> String {
        match self {
            Self::Comma => items.join(", "),
            Self::Newline => items.join("\n"),
            Self::Natural => items.join(" "),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Lcg(u64);

impl Lcg {
    pub fn new(seed: u64) -> Self {
        let s = if seed == 0 { 0x9E3779B97F4A7C15 } else { seed };
        Self(s)
    }

    pub fn next_u32(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(2862933555777941757)
            .wrapping_add(3037000493);
        (self.0 >> 32) as u32
    }

    pub fn gen_range(&mut self, upper: usize) -> usize {
        if upper == 0 {
            0
        } else {
            (self.next_u32() as usize) % upper
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lcg_is_deterministic() {
        let mut a = Lcg::new(42);
        let mut b = Lcg::new(42);
        for _ in 0..100 {
            assert_eq!(a.next_u32(), b.next_u32());
        }
    }

    #[test]
    fn lcg_different_seeds_differ() {
        let mut a = Lcg::new(1);
        let mut b = Lcg::new(2);
        let mut diff = 0;
        for _ in 0..20 {
            if a.next_u32() != b.next_u32() {
                diff += 1;
            }
        }
        assert!(diff > 10, "expected divergence between seeds");
    }

    #[test]
    fn strategy_join() {
        assert_eq!(
            CombineStrategy::Comma.join(&["a".into(), "b".into()]),
            "a, b"
        );
        assert_eq!(
            CombineStrategy::Newline.join(&["a".into(), "b".into()]),
            "a\nb"
        );
        assert_eq!(
            CombineStrategy::Natural.join(&["a".into(), "b".into()]),
            "a b"
        );
    }
}
