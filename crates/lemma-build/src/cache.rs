//! Build cache for incremental builds

use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Hash type using BLAKE3 (32 bytes)
pub type Hash = [u8; 32];

/// The build cache tracks file hashes to determine when rebuilds are needed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildCache {
    /// Map from file path to its content hash
    file_hashes: HashMap<String, Hash>,

    /// Map from build artifact to its input hash
    /// The input hash is computed from the content hash of the source file
    /// and all its transitive dependencies
    artifact_hashes: HashMap<String, Hash>,
}

impl BuildCache {
    /// Create a new empty cache
    pub fn new() -> Self {
        Self {
            file_hashes: HashMap::new(),
            artifact_hashes: HashMap::new(),
        }
    }

    /// Load cache from disk
    ///
    /// Loads from `.lake/build_cache.json` if it exists, otherwise returns empty cache.
    pub fn load(project_dir: &Path) -> Result<Self> {
        let cache_path = project_dir.join(".lake/build_cache.json");

        if cache_path.exists() {
            let content = std::fs::read_to_string(&cache_path)?;
            let cache = serde_json::from_str(&content)?;
            Ok(cache)
        } else {
            Ok(Self::new())
        }
    }

    /// Save cache to disk
    pub fn save(&self, project_dir: &Path) -> Result<()> {
        let cache_dir = project_dir.join(".lake");
        std::fs::create_dir_all(&cache_dir)?;

        let cache_path = cache_dir.join("build_cache.json");
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(&cache_path, content)?;

        Ok(())
    }

    /// Compute the BLAKE3 hash of a file
    pub fn hash_file(&self, path: &Path) -> Result<Hash> {
        let content = std::fs::read(path)?;
        let hash = blake3::hash(&content);
        Ok(*hash.as_bytes())
    }

    /// Check if a file has changed since last build
    ///
    /// Returns true if the file is new or has been modified.
    pub fn has_changed(&self, path: &str, current_hash: Hash) -> bool {
        self.file_hashes
            .get(path)
            .map(|&old_hash| old_hash != current_hash)
            .unwrap_or(true)
    }

    /// Update the hash for a file
    pub fn update_file_hash(&mut self, path: String, hash: Hash) {
        self.file_hashes.insert(path, hash);
    }

    /// Update the hash for an artifact
    pub fn update_artifact_hash(&mut self, artifact: String, hash: Hash) {
        self.artifact_hashes.insert(artifact, hash);
    }

    /// Check if an artifact needs to be rebuilt
    ///
    /// An artifact needs rebuilding if:
    /// - It doesn't exist
    /// - Its input hash has changed
    pub fn needs_rebuild(&self, artifact: &str, input_hash: Hash) -> bool {
        self.artifact_hashes
            .get(artifact)
            .map(|&old_hash| old_hash != input_hash)
            .unwrap_or(true)
    }
}

impl Default for BuildCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_cache() {
        let cache = BuildCache::new();
        assert!(cache.file_hashes.is_empty());
        assert!(cache.artifact_hashes.is_empty());
    }

    #[test]
    fn test_has_changed() {
        let mut cache = BuildCache::new();
        let hash1 = [0u8; 32];
        let hash2 = [1u8; 32];

        // New file should be marked as changed
        assert!(cache.has_changed("test.lean", hash1));

        // After updating, same hash should not be changed
        cache.update_file_hash("test.lean".to_string(), hash1);
        assert!(!cache.has_changed("test.lean", hash1));

        // Different hash should be changed
        assert!(cache.has_changed("test.lean", hash2));
    }
}
