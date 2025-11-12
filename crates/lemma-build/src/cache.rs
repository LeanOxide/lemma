//! Build cache for incremental builds

use crate::error::Result;
use crate::module::Module;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

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

    /// Compute transitive hash for a module
    ///
    /// The transitive hash is computed from:
    /// 1. The module's own content hash
    /// 2. The transitive hashes of all its dependencies (sorted for determinism)
    ///
    /// This creates a Merkle tree where changes propagate through dependencies.
    pub fn compute_transitive_hash(
        &self,
        module: &Module,
        dependency_hashes: &HashMap<String, Hash>,
    ) -> Result<Hash> {
        // Start with the module's content hash
        let content_hash = self.hash_file(&module.path)?;

        // Collect dependency hashes in sorted order for determinism
        let mut dep_hashes = Vec::new();
        for import in &module.imports {
            if let Some(&hash) = dependency_hashes.get(import) {
                dep_hashes.push(hash);
            }
            // If dependency not found, skip it (external dependency)
        }
        dep_hashes.sort_unstable();

        // Combine: hash(content_hash || sorted_dep_hashes)
        let mut hasher = blake3::Hasher::new();
        hasher.update(&content_hash);
        for dep_hash in dep_hashes {
            hasher.update(&dep_hash);
        }

        Ok(*hasher.finalize().as_bytes())
    }

    /// Compute transitive hashes for all modules in dependency order
    ///
    /// Modules must be provided in topological order (dependencies first).
    /// Returns a map from module name to transitive hash.
    pub fn compute_all_transitive_hashes(
        &self,
        modules: &[Module],
    ) -> Result<HashMap<String, Hash>> {
        let mut transitive_hashes = HashMap::new();

        // Process modules in order (dependencies come first)
        for module in modules {
            let trans_hash = self.compute_transitive_hash(module, &transitive_hashes)?;
            transitive_hashes.insert(module.name.clone(), trans_hash);
        }

        Ok(transitive_hashes)
    }

    /// Determine which modules need rebuilding
    ///
    /// A module needs rebuilding if:
    /// - It's new (not in cache)
    /// - Its source file has changed
    /// - Any of its dependencies need rebuilding (transitive)
    ///
    /// Modules must be in topological order (dependencies first).
    /// Returns a vector of module names that need rebuilding.
    pub fn modules_needing_rebuild(
        &self,
        modules: &[Module],
        build_dir: &Path,
        package_name: &str,
    ) -> Result<Vec<String>> {
        let transitive_hashes = self.compute_all_transitive_hashes(modules)?;
        let mut needs_rebuild = Vec::new();

        for module in modules {
            let trans_hash = transitive_hashes[&module.name];

            // Check if artifact exists
            let artifact_path = Self::artifact_path(build_dir, &module.name, package_name);
            if !artifact_path.exists() {
                needs_rebuild.push(module.name.clone());
                continue;
            }

            // Check if transitive hash changed
            let artifact_key = format!("{}.olean", module.name);
            if self.needs_rebuild(&artifact_key, trans_hash) {
                needs_rebuild.push(module.name.clone());
            }
        }

        Ok(needs_rebuild)
    }

    /// Get the artifact path for a module
    ///
    /// Example: "Foo.Bar" with package "mypackage" -> "build/lib/mypackage/Foo/Bar.olean"
    fn artifact_path(build_dir: &Path, module_name: &str, package_name: &str) -> PathBuf {
        let parts: Vec<&str> = module_name.split('.').collect();
        let mut path = build_dir.join("lib").join(package_name);
        for part in parts {
            path.push(part);
        }
        path.set_extension("olean");
        path
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
    use crate::module::Module;
    use std::fs;
    use tempfile::TempDir;

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

    #[test]
    fn test_hash_file() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.lean");
        fs::write(&test_file, "def hello : String := \"world\"").unwrap();

        let cache = BuildCache::new();
        let hash1 = cache.hash_file(&test_file).unwrap();

        // Same content should produce same hash
        let hash2 = cache.hash_file(&test_file).unwrap();
        assert_eq!(hash1, hash2);

        // Different content should produce different hash
        fs::write(&test_file, "def hello : String := \"changed\"").unwrap();
        let hash3 = cache.hash_file(&test_file).unwrap();
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_compute_transitive_hash() {
        let temp_dir = TempDir::new().unwrap();

        // Create test files
        let a_file = temp_dir.path().join("A.lean");
        let b_file = temp_dir.path().join("B.lean");

        fs::write(&a_file, "def a : Nat := 1").unwrap();
        fs::write(&b_file, "import A\n\ndef b : Nat := 2").unwrap();

        let cache = BuildCache::new();

        // Module A has no dependencies
        let module_a = Module::new("A".to_string(), a_file.clone(), vec![]);
        let deps_a = HashMap::new();
        let hash_a = cache.compute_transitive_hash(&module_a, &deps_a).unwrap();

        // Module B depends on A
        let module_b = Module::new("B".to_string(), b_file.clone(), vec!["A".to_string()]);
        let mut deps_b = HashMap::new();
        deps_b.insert("A".to_string(), hash_a);
        let hash_b = cache.compute_transitive_hash(&module_b, &deps_b).unwrap();

        // B's hash should differ from A's hash
        assert_ne!(hash_a, hash_b);

        // If A changes, B's transitive hash should change
        fs::write(&a_file, "def a : Nat := 999").unwrap();
        let hash_a_new = cache.compute_transitive_hash(&module_a, &deps_a).unwrap();
        assert_ne!(hash_a, hash_a_new);

        deps_b.insert("A".to_string(), hash_a_new);
        let hash_b_new = cache.compute_transitive_hash(&module_b, &deps_b).unwrap();
        assert_ne!(hash_b, hash_b_new);
    }

    #[test]
    fn test_compute_all_transitive_hashes() {
        let temp_dir = TempDir::new().unwrap();

        // Create A -> B -> C dependency chain
        let a_file = temp_dir.path().join("A.lean");
        let b_file = temp_dir.path().join("B.lean");
        let c_file = temp_dir.path().join("C.lean");

        fs::write(&a_file, "def a : Nat := 1").unwrap();
        fs::write(&b_file, "import A\n\ndef b : Nat := 2").unwrap();
        fs::write(&c_file, "import B\n\ndef c : Nat := 3").unwrap();

        let modules = vec![
            Module::new("A".to_string(), a_file, vec![]),
            Module::new("B".to_string(), b_file, vec!["A".to_string()]),
            Module::new("C".to_string(), c_file, vec!["B".to_string()]),
        ];

        let cache = BuildCache::new();
        let hashes = cache.compute_all_transitive_hashes(&modules).unwrap();

        assert_eq!(hashes.len(), 3);
        assert!(hashes.contains_key("A"));
        assert!(hashes.contains_key("B"));
        assert!(hashes.contains_key("C"));

        // All hashes should be distinct
        assert_ne!(hashes["A"], hashes["B"]);
        assert_ne!(hashes["B"], hashes["C"]);
        assert_ne!(hashes["A"], hashes["C"]);
    }

    #[test]
    fn test_modules_needing_rebuild() {
        let temp_dir = TempDir::new().unwrap();
        let build_dir = temp_dir.path().join("build");
        let package_name = "testpkg";

        let a_file = temp_dir.path().join("A.lean");
        let b_file = temp_dir.path().join("B.lean");

        fs::write(&a_file, "def a : Nat := 1").unwrap();
        fs::write(&b_file, "import A\n\ndef b : Nat := 2").unwrap();

        let modules = vec![
            Module::new("A".to_string(), a_file.clone(), vec![]),
            Module::new("B".to_string(), b_file.clone(), vec!["A".to_string()]),
        ];

        let cache = BuildCache::new();

        // Initially, all modules need rebuilding (no artifacts)
        let needs_rebuild = cache.modules_needing_rebuild(&modules, &build_dir, package_name).unwrap();
        assert_eq!(needs_rebuild.len(), 2);
        assert!(needs_rebuild.contains(&"A".to_string()));
        assert!(needs_rebuild.contains(&"B".to_string()));

        // Create artifacts with package name in path
        fs::create_dir_all(build_dir.join("lib").join(package_name)).unwrap();
        fs::write(build_dir.join("lib").join(package_name).join("A.olean"), "artifact").unwrap();
        fs::write(build_dir.join("lib").join(package_name).join("B.olean"), "artifact").unwrap();

        // Update cache with current hashes
        let mut updated_cache = BuildCache::new();
        let hashes = cache.compute_all_transitive_hashes(&modules).unwrap();
        updated_cache.update_artifact_hash("A.olean".to_string(), hashes["A"]);
        updated_cache.update_artifact_hash("B.olean".to_string(), hashes["B"]);

        // Now nothing needs rebuilding
        let needs_rebuild = updated_cache
            .modules_needing_rebuild(&modules, &build_dir, package_name)
            .unwrap();
        assert_eq!(needs_rebuild.len(), 0);

        // Modify A
        fs::write(&a_file, "def a : Nat := 999").unwrap();

        // Both A and B should need rebuilding (B transitively)
        let needs_rebuild = updated_cache
            .modules_needing_rebuild(&modules, &build_dir, package_name)
            .unwrap();
        assert_eq!(needs_rebuild.len(), 2);
        assert!(needs_rebuild.contains(&"A".to_string()));
        assert!(needs_rebuild.contains(&"B".to_string()));
    }

    #[test]
    fn test_save_and_load_cache() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path();

        let mut cache = BuildCache::new();
        cache.update_file_hash("test.lean".to_string(), [1u8; 32]);
        cache.update_artifact_hash("test.olean".to_string(), [2u8; 32]);

        // Save cache
        cache.save(project_dir).unwrap();

        // Load cache
        let loaded_cache = BuildCache::load(project_dir).unwrap();

        // Verify contents
        assert!(!loaded_cache.has_changed("test.lean", [1u8; 32]));
        assert!(!loaded_cache.needs_rebuild("test.olean", [2u8; 32]));
    }

    #[test]
    fn test_artifact_path() {
        let build_dir = PathBuf::from("/project/build");
        let package_name = "mypackage";

        assert_eq!(
            BuildCache::artifact_path(&build_dir, "Main", package_name),
            PathBuf::from("/project/build/lib/mypackage/Main.olean")
        );

        assert_eq!(
            BuildCache::artifact_path(&build_dir, "Foo.Bar", package_name),
            PathBuf::from("/project/build/lib/mypackage/Foo/Bar.olean")
        );

        assert_eq!(
            BuildCache::artifact_path(&build_dir, "A.B.C", package_name),
            PathBuf::from("/project/build/lib/mypackage/A/B/C.olean")
        );
    }
}
