use std::collections::HashSet;
use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::tags::{Lang, LangAwarePool, TagStore};
use crate::theme::Theme;

use super::llm::{call, AgentKind};
use super::strategy::{CombineStrategy, Lcg, PromptError};

#[derive(Debug, Clone)]
pub struct CombineContext {
    pub theme: Theme,
    pub lang: Lang,
    pub strategy: CombineStrategy,
    pub max_length: usize,
    pub seed: u64,
    pub project_root: PathBuf,
}

#[derive(Debug, Default, Clone)]
pub struct CombineOutput {
    pub prompt: String,
    pub selected: Vec<(String, String)>,
}

pub fn combine(ctx: &CombineContext, pool: &LangAwarePool) -> Result<CombineOutput, PromptError> {
    let lang_store = pool
        .get(ctx.lang)
        .ok_or(PromptError::UnknownLang(ctx.lang.as_str().into()))?;

    let mut rng = Lcg::new(ctx.seed);
    let mut selected: Vec<(String, String)> = Vec::new();
    let mut selected_set: HashSet<String> = HashSet::new();

    for (cat_name, cat) in &ctx.theme.order.fixed {
        let stem = stem_from_file(&cat.file)
            .ok_or_else(|| PromptError::InvalidFile(cat.file.clone()))?;
        let bucket = lang_store
            .get(&stem)
            .ok_or_else(|| PromptError::UnknownCategory(cat_name.clone()))?;
        pick_n(
            &ctx.theme,
            bucket,
            cat_name,
            cat.count,
            cat.max.unwrap_or(cat.count),
            false,
            &mut rng,
            &mut selected,
            &mut selected_set,
        )?;
    }

    for (cat_name, cat) in &ctx.theme.order.random {
        let stem = stem_from_file(&cat.file)
            .ok_or_else(|| PromptError::InvalidFile(cat.file.clone()))?;
        let bucket = lang_store
            .get(&stem)
            .ok_or_else(|| PromptError::UnknownCategory(cat_name.clone()))?;
        pick_n(
            &ctx.theme,
            bucket,
            cat_name,
            cat.count,
            cat.max.unwrap_or(cat.count),
            false,
            &mut rng,
            &mut selected,
            &mut selected_set,
        )?;
    }

    for (cat_name, opt) in &ctx.theme.order.optional {
        let stem = match stem_from_file(&opt.file) {
            Some(s) => s,
            None => continue,
        };
        let Some(bucket) = lang_store.get(&stem) else {
            continue;
        };
        let roll = (rng.next_u32() as f32) / (u32::MAX as f32);
        if roll > opt.probability {
            continue;
        }
        pick_n(
            &ctx.theme,
            bucket,
            cat_name,
            opt.count,
            opt.count,
            true,
            &mut rng,
            &mut selected,
            &mut selected_set,
        )?;
    }

    if selected.len() > ctx.theme.generation.max_elements {
        selected.truncate(ctx.theme.generation.max_elements);
    }

    let items: Vec<String> = selected.iter().map(|(_, v)| v.clone()).collect();
    let joined = ctx.strategy.join(&items);

    let prompt = if joined.len() > ctx.max_length {
        truncate(&joined, ctx.max_length)
    } else {
        joined
    };

    Ok(CombineOutput { prompt, selected })
}

fn pick_n(
    theme: &Theme,
    bucket: &indexmap::IndexSet<String>,
    cat_name: &str,
    count: usize,
    max: usize,
    optional: bool,
    rng: &mut Lcg,
    selected: &mut Vec<(String, String)>,
    selected_set: &mut HashSet<String>,
) -> Result<(), PromptError> {
    let upper = (count.max(max)).min(bucket.len());
    if upper == 0 {
        if optional {
            return Ok(());
        }
        return Err(PromptError::EmptyCategory(cat_name.into()));
    }
    let n_pick = count + rng.gen_range(upper - count + 1);
    let max_attempts = n_pick * 32 + 64;
    let mut attempts = 0;

    let mut picked: Vec<String> = Vec::new();
    while picked.len() < n_pick && attempts < max_attempts {
        attempts += 1;
        let item = match bucket.get_index(rng.gen_range(bucket.len())) {
            Some(s) => s.clone(),
            None => break,
        };
        if selected_set.contains(&item) {
            continue;
        }
        if violates_conflict(theme, cat_name, &item, &picked) {
            continue;
        }
        picked.push(item.clone());
        selected_set.insert(item);
    }

    if picked.len() < count && !optional {
        return Err(PromptError::EmptyCategory(cat_name.into()));
    }

    for item in picked {
        selected.push((cat_name.to_string(), item));
    }
    Ok(())
}

fn violates_conflict(theme: &Theme, cat_name: &str, candidate: &str, picked: &[String]) -> bool {
    let Some(groups) = theme.compatibility.conflicts.get(cat_name) else {
        return false;
    };
    for group in groups {
        if group.iter().any(|g| g == candidate) {
            if group.iter().any(|g| picked.iter().any(|p| p == g)) {
                return true;
            }
        }
    }
    false
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    let mut cut = max;
    while cut > 0 && !s.is_char_boundary(cut) {
        cut -= 1;
    }
    s[..cut].to_string()
}

pub fn stem_from_file(file: &str) -> Option<String> {
    let p = Path::new(file);
    p.file_stem().and_then(|s| s.to_str()).map(|s| s.to_string())
}

pub async fn refine(agent: Option<&AgentKind>, prompt: &str) -> String {
    let Some(agent) = agent else {
        return prompt.to_string();
    };
    match call(agent, prompt).await {
        Ok(s) if !s.trim().is_empty() => s,
        Ok(_) => {
            tracing::warn!("llm returned empty response, falling back to combine output");
            prompt.to_string()
        }
        Err(e) => {
            tracing::warn!(error = %e, "llm refine failed, falling back to combine output");
            prompt.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tags::LangAwarePool;
    use std::io::Write;
    use std::path::PathBuf;

    fn write_pool(dir: &Path, lang: Lang, files: &[(&str, &[&str])]) {
        std::fs::create_dir_all(dir.join(lang.as_str())).unwrap();
        for (name, items) in files {
            let p = dir.join(lang.as_str()).join(name);
            let mut f = std::fs::File::create(p).unwrap();
            for it in *items {
                writeln!(f, "{it}").unwrap();
            }
        }
    }

    fn load_pool(dir: &Path, langs: &[Lang]) -> LangAwarePool {
        let mut pool = LangAwarePool::new();
        for l in langs {
            pool.load_dir(*l, &dir.join(l.as_str())).unwrap();
        }
        pool
    }

    fn load_theme_str(content: &str) -> Theme {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("demo.toml");
        std::fs::write(&p, content).unwrap();
        Theme::load(dir.path(), "demo").unwrap()
    }

    fn ctx(theme: Theme, lang: Lang, seed: u64) -> CombineContext {
        CombineContext {
            theme,
            lang,
            strategy: CombineStrategy::Comma,
            max_length: 800,
            seed,
            project_root: PathBuf::from("/nonexistent"),
        }
    }

    #[tokio::test]
    async fn refine_with_none_agent_returns_input() {
        let out = refine(None, "original prompt").await;
        assert_eq!(out, "original prompt");
    }

    #[test]
    fn combine_uses_fixed_and_random() {
        let dir = tempfile::tempdir().unwrap();
        write_pool(
            dir.path(),
            Lang::Zh,
            &[
                ("风格.txt", &["3D 渲染", "写实", "油画"]),
                ("主体.txt", &["女性", "男性"]),
                ("发型.txt", &["长发", "短发"]),
            ],
        );
        let pool = load_pool(dir.path(), &[Lang::Zh]);
        let theme = load_theme_str(
            r#"
[meta]
id = "demo"
name = "d"
lang = "zh"

[order.fixed]
style = { file = "tags/zh/风格.txt", count = 1 }
subject = { file = "tags/zh/主体.txt", count = 1 }

[order.random]
hair = { file = "tags/zh/发型.txt", count = 1, max = 1 }
"#,
        );
        let out = combine(&ctx(theme, Lang::Zh, 1), &pool).unwrap();
        assert!(!out.prompt.is_empty());
        assert_eq!(out.selected.len(), 3);
    }

    #[test]
    fn conflict_drops_candidate() {
        let dir = tempfile::tempdir().unwrap();
        write_pool(
            dir.path(),
            Lang::Zh,
            &[(
                "服装.txt",
                &["比基尼", "毛衣", "汉服", "牛仔裤", "T恤"],
            )],
        );
        let pool = load_pool(dir.path(), &[Lang::Zh]);
        let theme = load_theme_str(
            r#"
[meta]
id = "demo"
name = "d"
lang = "zh"

[order.random]
clothing = { file = "tags/zh/服装.txt", count = 2, max = 2 }

[compatibility.conflicts]
clothing = [["比基尼", "毛衣"]]
"#,
        );
        for seed in 0..30 {
            let out = combine(&ctx(theme.clone(), Lang::Zh, seed), &pool).unwrap();
            assert_eq!(out.selected.len(), 2, "seed={seed}");
            let items: Vec<&str> = out.selected.iter().map(|(_, v)| v.as_str()).collect();
            let has_conflict = items.contains(&"比基尼") && items.contains(&"毛衣");
            assert!(!has_conflict, "conflict violated: {items:?}");
        }
    }

    #[test]
    fn truncate_when_too_long() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("zh")).unwrap();
        let f = dir.path().join("zh").join("a.txt");
        std::fs::write(&f, "很长的元素".repeat(20)).unwrap();
        let mut pool = LangAwarePool::new();
        pool.load_dir(Lang::Zh, &dir.path().join("zh")).unwrap();

        let theme = load_theme_str(
            r#"
[meta]
id = "demo"
name = "d"
lang = "zh"

[order.fixed]
a = { file = "tags/zh/a.txt", count = 1 }
"#,
        );
        let mut c = ctx(theme, Lang::Zh, 1);
        c.max_length = 10;
        let out = combine(&c, &pool).unwrap();
        assert!(out.prompt.chars().count() <= 10);
    }

    #[test]
    fn optional_skipped_when_roll_fails() {
        let dir = tempfile::tempdir().unwrap();
        write_pool(dir.path(), Lang::Zh, &[("a.txt", &["x"])]);
        let pool = load_pool(dir.path(), &[Lang::Zh]);
        let theme = load_theme_str(
            r#"
[meta]
id = "demo"
name = "d"
lang = "zh"

[order.optional]
a = { file = "tags/zh/a.txt", probability = 0.0 }
"#,
        );
        let out = combine(&ctx(theme, Lang::Zh, 1), &pool).unwrap();
        assert!(out.selected.is_empty());
    }

    #[test]
    fn unknown_lang_returns_error() {
        let pool = LangAwarePool::new();
        let theme = load_theme_str(
            r#"
[meta]
id = "x"
name = "x"
lang = "zh"
"#,
        );
        let err = combine(&ctx(theme, Lang::En, 1), &pool).unwrap_err();
        assert!(matches!(err, PromptError::UnknownLang(_)));
    }
}
