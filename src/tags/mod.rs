use std::collections::HashMap;
use std::path::Path;

use indexmap::IndexSet;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TagsError {
    #[error("read tags file {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("tags dir not found: {0}")]
    DirNotFound(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    Zh,
    En,
}

impl Lang {
    pub fn as_str(&self) -> &'static str {
        match self {
            Lang::Zh => "zh",
            Lang::En => "en",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "zh" => Some(Lang::Zh),
            "en" => Some(Lang::En),
            _ => None,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct TagStore {
    by_category: HashMap<String, IndexSet<String>>,
}

impl TagStore {
    pub fn load_file(&mut self, category: &str, path: &Path) -> Result<(), TagsError> {
        let text = std::fs::read_to_string(path).map_err(|e| TagsError::Io {
            path: path.display().to_string(),
            source: e,
        })?;
        let bucket = self.by_category.entry(category.to_string()).or_default();
        for line in text.lines() {
            let t = line.trim();
            if t.is_empty() || t.starts_with('#') {
                continue;
            }
            bucket.insert(t.to_string());
        }
        Ok(())
    }

    pub fn get(&self, category: &str) -> Option<&IndexSet<String>> {
        self.by_category.get(category)
    }
}

#[derive(Debug, Default, Clone)]
pub struct LangAwarePool {
    pools: HashMap<String, TagStore>,
}

impl LangAwarePool {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load_dir(&mut self, lang: Lang, dir: &Path) -> Result<(), TagsError> {
        if !dir.exists() {
            return Err(TagsError::DirNotFound(dir.display().to_string()));
        }
        let store = self.pools.entry(lang.as_str().to_string()).or_default();
        for entry_fs in std::fs::read_dir(dir).map_err(|e| TagsError::Io {
            path: dir.display().to_string(),
            source: e,
        })? {
            let entry_fs = entry_fs.map_err(|e| TagsError::Io {
                path: dir.display().to_string(),
                source: e,
            })?;
            let path = entry_fs.path();
            if !path.is_file() {
                continue;
            }
            if path.extension().and_then(|s| s.to_str()) != Some("txt") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            store.load_file(stem, &path)?;
        }
        Ok(())
    }

    pub fn get(&self, lang: Lang) -> Option<&TagStore> {
        self.pools.get(lang.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_file(dir: &Path, name: &str, content: &str) {
        let p = dir.join(name);
        let mut f = std::fs::File::create(p).unwrap();
        f.write_all(content.as_bytes()).unwrap();
    }

    #[test]
    fn load_categories_by_filename() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("发型.txt");
        std::fs::write(&f, "# comment\n\n长发\n 短发 \n# another\n盘发\n").unwrap();
        let mut store = TagStore::default();
        store.load_file("发型", &f).unwrap();
        let bucket = store.get("发型").unwrap();
        assert_eq!(bucket.len(), 3);
        assert!(bucket.contains("长发"));
    }

    #[test]
    fn dedup_within_category() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("a.txt");
        std::fs::write(&f, "长发\n长发\n短发\n").unwrap();
        let mut store = TagStore::default();
        store.load_file("a", &f).unwrap();
        assert_eq!(store.get("a").unwrap().len(), 2);
    }

    #[test]
    fn pool_separates_languages() {
        let zh = tempfile::tempdir().unwrap();
        let en = tempfile::tempdir().unwrap();
        write_file(zh.path(), "hair.txt", "长发\n短发\n");
        write_file(en.path(), "hair.txt", "long hair\nshort hair\n");

        let mut pool = LangAwarePool::new();
        pool.load_dir(Lang::Zh, zh.path()).unwrap();
        pool.load_dir(Lang::En, en.path()).unwrap();

        assert_eq!(pool.get(Lang::Zh).unwrap().get("hair").unwrap().len(), 2);
        assert_eq!(pool.get(Lang::En).unwrap().get("hair").unwrap().len(), 2);
    }

    #[test]
    fn missing_dir_returns_error() {
        let mut pool = LangAwarePool::new();
        let r = pool.load_dir(Lang::Zh, Path::new("/no/such/dir"));
        assert!(r.is_err());
    }
}
