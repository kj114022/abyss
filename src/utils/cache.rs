use anyhow::Result;
use dashmap::DashMap;
use md5;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CacheEntry {
    pub hash: String,
    pub tokens: usize,
    pub modified: u64,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Cache {
    pub entries: DashMap<String, CacheEntry>,
}

impl Cache {
    pub fn load() -> Self {
        if let Ok(content) = fs::read_to_string(".abyss-cache.json") {
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            Cache::default()
        }
    }

    pub fn save(&self) -> Result<()> {
        let content = serde_json::to_string(&self)?;
        fs::write(".abyss-cache.json", content)?;
        Ok(())
    }

    /// Returns a clone of the cache entry if it exists.
    /// Cloning avoids holding the shard lock for too long.
    pub fn get(&self, path: &str) -> Option<CacheEntry> {
        self.entries.get(path).map(|r| r.value().clone())
    }

    pub fn update(&self, path: String, entry: CacheEntry) {
        self.entries.insert(path, entry);
    }

    pub fn compute_hash(content: &str, config_str: &str) -> String {
        let mut context = md5::Context::new();
        context.consume(content.as_bytes());
        context.consume(config_str.as_bytes());
        format!("{:x}", context.finalize())
    }
}
