use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

use crate::model::{CrateInfo, Module};

/// Cache of all hashes for incremental rebuilds and staleness detection.
///
/// This is the single source of truth for change detection. LLM-facing files
/// (annotations.toml, api-surface.md, etc.) never contain hashes.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Cache {
    /// Per-file hashes (for incremental parse skipping)
    pub files: BTreeMap<String, CacheFileEntry>,
    /// Per-module hashes (file hash of the module's source)
    #[serde(default)]
    pub modules: BTreeMap<String, String>,
    /// Per-item content hashes (hash of the item's source lines)
    #[serde(default)]
    pub items: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheFileEntry {
    pub hash: String,
    pub last_indexed: String,
}

impl Cache {
    /// Load cache from the output directory
    pub fn load(output_dir: &Path) -> Result<Self> {
        let path = output_dir.join("cache.json");
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Cannot read {}", path.display()))?;
        let cache: Cache = serde_json::from_str(&content).context("Failed to parse cache.json")?;
        Ok(cache)
    }

    /// Save cache to the output directory
    pub fn save(&self, output_dir: &Path) -> Result<()> {
        let path = output_dir.join("cache.json");
        let content =
            serde_json::to_string_pretty(self).context("Failed to serialize cache")?;
        std::fs::write(&path, content)
            .with_context(|| format!("Cannot write {}", path.display()))?;
        Ok(())
    }

    /// Build cache from parsed crate data
    pub fn from_crates(crates: &[CrateInfo]) -> Self {
        let mut cache = Cache::default();
        let now = chrono::Utc::now().to_rfc3339();

        for crate_info in crates {
            collect_hashes(&crate_info.root_module, &mut cache, &now);
        }

        cache
    }

    /// Check if a file is unchanged since last indexing
    pub fn is_file_unchanged(&self, file_path: &str, current_hash: &str) -> bool {
        self.files
            .get(file_path)
            .map(|entry| entry.hash == current_hash)
            .unwrap_or(false)
    }

    /// Check if a module's hash changed between this cache and another
    pub fn module_hash_changed(&self, other: &Cache, module_path: &str) -> bool {
        match (self.modules.get(module_path), other.modules.get(module_path)) {
            (Some(old), Some(new)) => old != new,
            (None, Some(_)) => true, // new module
            _ => false,
        }
    }

    /// Check if an item's hash changed between this cache and another
    pub fn item_hash_changed(&self, other: &Cache, item_path: &str) -> bool {
        match (self.items.get(item_path), other.items.get(item_path)) {
            (Some(old), Some(new)) => old != new,
            (None, Some(_)) => true, // new item
            _ => false,
        }
    }
}

fn collect_hashes(module: &Module, cache: &mut Cache, now: &str) {
    // File hash
    let path_str = module.file_path.display().to_string();
    cache.files.entry(path_str).or_insert_with(|| CacheFileEntry {
        hash: module.file_hash.clone(),
        last_indexed: now.to_string(),
    });

    // Module hash
    cache
        .modules
        .insert(module.path.clone(), module.file_hash.clone());

    // Item hashes
    for item in &module.items {
        let item_path = format!("{}::{}", module.path, item.name);
        cache.items.insert(item_path, item.content_hash.clone());
    }

    for sub in &module.submodules {
        collect_hashes(sub, cache, now);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_roundtrip() {
        let mut cache = Cache::default();
        cache.files.insert(
            "src/lib.rs".to_string(),
            CacheFileEntry {
                hash: "abc123".to_string(),
                last_indexed: "2025-01-15T00:00:00Z".to_string(),
            },
        );
        cache
            .modules
            .insert("crate".to_string(), "abc123".to_string());
        cache
            .items
            .insert("crate::init".to_string(), "def456".to_string());

        let json = serde_json::to_string_pretty(&cache).unwrap();
        let loaded: Cache = serde_json::from_str(&json).unwrap();

        assert_eq!(loaded.files["src/lib.rs"].hash, "abc123");
        assert_eq!(loaded.modules["crate"], "abc123");
        assert_eq!(loaded.items["crate::init"], "def456");
    }

    #[test]
    fn test_is_file_unchanged() {
        let mut cache = Cache::default();
        cache.files.insert(
            "src/lib.rs".to_string(),
            CacheFileEntry {
                hash: "abc123".to_string(),
                last_indexed: "2025-01-15T00:00:00Z".to_string(),
            },
        );

        assert!(cache.is_file_unchanged("src/lib.rs", "abc123"));
        assert!(!cache.is_file_unchanged("src/lib.rs", "changed"));
        assert!(!cache.is_file_unchanged("src/main.rs", "abc123"));
    }

    #[test]
    fn test_staleness_detection() {
        let mut old_cache = Cache::default();
        old_cache
            .items
            .insert("crate::init".to_string(), "hash_v1".to_string());
        old_cache
            .modules
            .insert("crate".to_string(), "mod_v1".to_string());

        let mut new_cache = Cache::default();
        new_cache
            .items
            .insert("crate::init".to_string(), "hash_v2".to_string()); // changed
        new_cache
            .modules
            .insert("crate".to_string(), "mod_v1".to_string()); // same

        assert!(old_cache.item_hash_changed(&new_cache, "crate::init"));
        assert!(!old_cache.module_hash_changed(&new_cache, "crate"));

        // New item
        new_cache
            .items
            .insert("crate::run".to_string(), "hash_new".to_string());
        assert!(old_cache.item_hash_changed(&new_cache, "crate::run"));
    }
}
