# Lemma Build System Implementation Plan

**Version:** 1.0  
**Date:** 2025-11-11  
**Status:** Planning

## Executive Summary

This document outlines a comprehensive plan to implement a native build system for lemma that reimplements Lake's build functionality from scratch in Rust. The goal is to provide a fast, reliable, and feature-complete alternative to Lake that integrates seamlessly with lemma's existing toolchain management capabilities.

**Key Goals:**
- Parse lakefile.toml and lakefile.lean configurations
- Implement incremental builds with hash-based caching
- Support parallel compilation
- Handle Lake's facet-based build system
- Maintain compatibility with existing Lean projects
- Provide superior performance through Rust's speed and parallelism

---

## 1. Architecture Overview

### 1.1 High-Level Design

The build system follows a pipeline architecture with clear separation of concerns:

```
┌─────────────────────────────────────────────────────────────┐
│                     Lemma Build System                      │
└─────────────────────────────────────────────────────────────┘
                              │
        ┌─────────────────────┼─────────────────────┐
        │                     │                     │
        ▼                     ▼                     ▼
┌──────────────┐    ┌──────────────┐    ┌──────────────┐
│   Lakefile   │    │   Module     │    │    Build     │
│   Parser     │───▶│  Resolver    │───▶│   Planner    │
└──────────────┘    └──────────────┘    └──────────────┘
        │                     │                     │
        │                     │                     ▼
        │                     │          ┌──────────────┐
        │                     │          │  Job         │
        │                     │          │  Scheduler   │
        │                     │          └──────────────┘
        │                     │                     │
        │                     │          ┌──────────────┐
        │                     └─────────▶│   Build      │
        │                                │   Cache      │
        │                                └──────────────┘
        │                                       │
        ▼                                       ▼
┌──────────────┐                     ┌──────────────┐
│ Compilation  │◀────────────────────│  Execution   │
│  Driver      │                     │  Engine      │
└──────────────┘                     └──────────────┘
        │
        ▼
┌──────────────┐
│   Linker     │
└──────────────┘
```

### 1.2 Core Components

#### 1.2.1 Lakefile Parser
**Purpose:** Parse and validate lakefile.toml and lakefile.lean configurations  
**Responsibilities:**
- Load and parse lakefile.toml (primary focus)
- Extract package metadata, dependencies, targets
- Validate configuration schema
- Optional: Parse lakefile.lean (Phase 2)

#### 1.2.2 Module Resolver
**Purpose:** Discover module dependencies and build the dependency graph  
**Responsibilities:**
- Scan .lean source files for imports
- Build directed acyclic graph (DAG) of module dependencies
- Detect circular dependencies
- Resolve transitive dependencies
- Handle Lake package imports

#### 1.2.3 Build Planner
**Purpose:** Determine what needs to be built and in what order  
**Responsibilities:**
- Topologically sort dependency graph
- Identify targets to build (modules, libraries, executables)
- Determine build order respecting dependencies
- Create build plan with tasks

#### 1.2.4 Build Cache
**Purpose:** Track file hashes and determine incremental build needs  
**Responsibilities:**
- Store content hashes of source files
- Store content hashes of build outputs
- Track dependency hashes (transitive)
- Detect when rebuilds are needed
- Persist cache to disk (.lake/build_cache.json)

#### 1.2.5 Job Scheduler
**Purpose:** Execute build tasks in parallel while respecting dependencies  
**Responsibilities:**
- Schedule jobs based on dependency constraints
- Manage worker thread pool
- Handle job queuing and execution
- Track job status (pending, running, completed, failed)
- Implement backpressure and load balancing

#### 1.2.6 Compilation Driver
**Purpose:** Invoke the Lean compiler to build modules  
**Responsibilities:**
- Execute lean compiler for each module
- Pass correct flags and options
- Generate .olean files (compiled modules)
- Generate .c files (C code)
- Generate .o files (object files)
- Capture and report compilation errors

#### 1.2.7 Linker
**Purpose:** Link compiled objects into executables and libraries  
**Responsibilities:**
- Link object files into executables
- Link object files into static/dynamic libraries
- Handle external libraries and link flags
- Use system linker (ld, clang, etc.)

#### 1.2.8 Target System
**Purpose:** Define what to build (modules, libraries, executables)  
**Responsibilities:**
- Represent different target types
- Track target metadata (name, type, dependencies)
- Support target selection (build specific targets)

### 1.3 Data Flow

```
lakefile.toml
    │
    ▼
[Parser] → Package Config
    │
    ▼
[Module Resolver]
    │
    ├─→ Scan source files
    ├─→ Parse imports
    └─→ Build dependency graph
    │
    ▼
Dependency Graph (DAG)
    │
    ▼
[Build Planner]
    │
    ├─→ Topological sort
    └─→ Create build tasks
    │
    ▼
Build Plan (ordered tasks)
    │
    ▼
[Build Cache] → Check hashes
    │
    ├─→ Cache hit: skip
    └─→ Cache miss: build
    │
    ▼
Filtered Build Plan (only what needs building)
    │
    ▼
[Job Scheduler]
    │
    ├─→ Schedule tasks
    ├─→ Manage parallelism
    └─→ Execute tasks
    │
    ▼
[Compilation Driver] → Execute lean compiler
    │
    ├─→ .lean → .olean
    ├─→ .lean → .c
    └─→ .c → .o
    │
    ▼
[Linker] → Link objects
    │
    └─→ Executable or library
```

### 1.4 Integration with Existing Lemma Crates

**New crates to create:**
- `lemma-build`: Main build system logic
- `lemma-lakefile`: Lakefile parsing
- `lemma-graph`: Dependency graph data structures

**Integration points with existing crates:**
- `lemma-config`: Read build settings, resolve toolchains
- `lemma-toolchain`: Find and execute lean/lake binaries
- `lemma-output`: Progress reporting and error display
- `lemma-cli`: Add build subcommand

---

## 2. Implementation Phases

### Phase 0: Foundation (2-3 weeks)
**Goal:** Set up core infrastructure and data structures

**Deliverables:**
- New crates: `lemma-build`, `lemma-lakefile`, `lemma-graph`
- Basic data structures (Package, Target, Module, etc.)
- Lakefile.toml parser with basic schema
- File system utilities (hash computation, file tracking)

**Complexity:** Medium  
**Dependencies:** None

### Phase 1: Module Discovery (2-3 weeks)
**Goal:** Scan source files and build dependency graph

**Deliverables:**
- Import parser (extract imports from .lean files)
- Dependency graph builder
- Cycle detection
- Graph visualization (for debugging)
- Module resolver tests

**Complexity:** Medium-High  
**Dependencies:** Phase 0

### Phase 2: Build Planning (2 weeks)
**Goal:** Determine what to build and in what order

**Deliverables:**
- Topological sort implementation
- Build plan generation
- Target selection logic
- Build plan visualization

**Complexity:** Medium  
**Dependencies:** Phase 1

### Phase 3: Build Cache (2-3 weeks)
**Goal:** Implement hash-based incremental builds

**Deliverables:**
- Content hash computation (BLAKE3)
- Transitive dependency hashing
- Cache persistence (.lake/build_cache.json)
- Cache invalidation logic
- Cache hit/miss tracking

**Complexity:** Medium-High  
**Dependencies:** Phase 2

### Phase 4: Job Scheduling (3-4 weeks)
**Goal:** Parallel execution of build tasks

**Deliverables:**
- Worker thread pool
- Dependency-aware task scheduler
- Job queue management
- Progress tracking
- Cancellation support

**Complexity:** High  
**Dependencies:** Phase 3

### Phase 5: Compilation Driver (2-3 weeks)
**Goal:** Execute lean compiler for modules

**Deliverables:**
- Lean compiler invocation
- Flag and option handling
- Error capture and reporting
- Support for .olean, .c, .o generation
- Compiler output parsing

**Complexity:** Medium  
**Dependencies:** Phase 4

### Phase 6: Linking (1-2 weeks)
**Goal:** Link compiled objects into final artifacts

**Deliverables:**
- Executable linking
- Library linking (static/dynamic)
- External library handling
- Platform-specific linker support

**Complexity:** Medium  
**Dependencies:** Phase 5

### Phase 7: CLI Integration (1 week)
**Goal:** Integrate with lemma CLI

**Deliverables:**
- `lemma build` command
- `lemma clean` command
- Command-line options (--target, --jobs, --verbose)
- Help text and documentation

**Complexity:** Low  
**Dependencies:** Phase 6

### Phase 8: Advanced Features (3-4 weeks)
**Goal:** Support advanced Lake features

**Deliverables:**
- lakefile.lean parsing (optional)
- Custom build scripts
- External tool integration
- Watch mode (continuous builds)
- Build server (daemon mode)

**Complexity:** High  
**Dependencies:** Phase 7

### Phase 9: Testing & Polish (2-3 weeks)
**Goal:** Comprehensive testing and bug fixes

**Deliverables:**
- Unit tests for all components
- Integration tests with real Lean projects
- Performance benchmarks
- Documentation
- Bug fixes

**Complexity:** Medium  
**Dependencies:** Phase 8

**Total estimated time:** 20-27 weeks (5-7 months)

---

## 3. Core Components Design

### 3.1 Lakefile Parser

#### Purpose
Parse lakefile.toml and extract package configuration.

#### Key Data Structures

```rust
// lemma-lakefile/src/lib.rs

/// Root configuration from lakefile.toml
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LakefileConfig {
    pub name: String,
    pub version: Option<String>,
    pub lean_version: Option<String>,
    pub dependencies: Vec<Dependency>,
    pub libraries: Vec<LibraryTarget>,
    pub executables: Vec<ExecutableTarget>,
    pub lean_options: Vec<String>,
    pub more_server_args: Vec<String>,
}

/// Package dependency
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Dependency {
    pub name: String,
    pub git: Option<String>,
    pub rev: Option<String>,
    pub path: Option<PathBuf>,
}

/// Library target
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LibraryTarget {
    pub name: String,
    pub roots: Vec<PathBuf>,
    pub globs: Vec<Glob>,
    pub lean_options: Vec<String>,
}

/// Executable target
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExecutableTarget {
    pub name: String,
    pub root: PathBuf,
    pub support_interpreter: bool,
}

/// Glob pattern for module discovery
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Glob {
    pub pattern: String,
    pub exclude: Vec<String>,
}
```

#### Algorithms

**Parse lakefile.toml:**
```rust
pub fn parse_lakefile(path: &Path) -> Result<LakefileConfig> {
    let content = fs::read_to_string(path)?;
    let config: LakefileConfig = toml::from_str(&content)?;
    validate_config(&config)?;
    Ok(config)
}

fn validate_config(config: &LakefileConfig) -> Result<()> {
    // Validate package name
    if config.name.is_empty() {
        bail!("Package name cannot be empty");
    }
    
    // Validate targets don't have duplicate names
    let mut names = HashSet::new();
    for lib in &config.libraries {
        if !names.insert(&lib.name) {
            bail!("Duplicate library name: {}", lib.name);
        }
    }
    for exe in &config.executables {
        if !names.insert(&exe.name) {
            bail!("Duplicate executable name: {}", exe.name);
        }
    }
    
    Ok(())
}
```

#### External Dependencies
- `toml` (v0.8): TOML parsing
- `serde` (v1.0): Serialization/deserialization
- `glob` (v0.3): Pattern matching for module discovery

---

### 3.2 Module Resolver

#### Purpose
Scan .lean files, extract imports, and build dependency graph.

#### Key Data Structures

```rust
// lemma-graph/src/lib.rs

/// A module in the dependency graph
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct ModuleId {
    pub package: String,
    pub path: Vec<String>, // e.g., ["Std", "Data", "List"]
}

impl ModuleId {
    /// Create from import path like "Std.Data.List"
    pub fn from_import(package: &str, import: &str) -> Self {
        ModuleId {
            package: package.to_string(),
            path: import.split('.').map(String::from).collect(),
        }
    }
    
    /// Convert to file path (e.g., "Std/Data/List.lean")
    pub fn to_file_path(&self) -> PathBuf {
        let mut path = PathBuf::new();
        for component in &self.path {
            path.push(component);
        }
        path.with_extension("lean")
    }
}

/// Module with its dependencies
#[derive(Debug, Clone)]
pub struct Module {
    pub id: ModuleId,
    pub file_path: PathBuf,
    pub imports: Vec<ModuleId>,
    pub hash: Option<Hash>,
}

/// Dependency graph
pub struct DependencyGraph {
    modules: HashMap<ModuleId, Module>,
    edges: HashMap<ModuleId, Vec<ModuleId>>, // id -> dependencies
    reverse_edges: HashMap<ModuleId, Vec<ModuleId>>, // id -> dependents
}

impl DependencyGraph {
    pub fn new() -> Self {
        DependencyGraph {
            modules: HashMap::new(),
            edges: HashMap::new(),
            reverse_edges: HashMap::new(),
        }
    }
    
    pub fn add_module(&mut self, module: Module) {
        let id = module.id.clone();
        let imports = module.imports.clone();
        
        self.modules.insert(id.clone(), module);
        self.edges.insert(id.clone(), imports.clone());
        
        // Update reverse edges
        for import in imports {
            self.reverse_edges
                .entry(import)
                .or_insert_with(Vec::new)
                .push(id.clone());
        }
    }
    
    pub fn get_module(&self, id: &ModuleId) -> Option<&Module> {
        self.modules.get(id)
    }
    
    pub fn dependencies(&self, id: &ModuleId) -> &[ModuleId] {
        self.edges.get(id).map(|v| v.as_slice()).unwrap_or(&[])
    }
    
    pub fn dependents(&self, id: &ModuleId) -> &[ModuleId] {
        self.reverse_edges.get(id).map(|v| v.as_slice()).unwrap_or(&[])
    }
    
    /// Topological sort of modules
    pub fn topological_sort(&self) -> Result<Vec<ModuleId>> {
        let mut sorted = Vec::new();
        let mut visited = HashSet::new();
        let mut visiting = HashSet::new();
        
        for id in self.modules.keys() {
            if !visited.contains(id) {
                self.visit(id, &mut visited, &mut visiting, &mut sorted)?;
            }
        }
        
        Ok(sorted)
    }
    
    fn visit(
        &self,
        id: &ModuleId,
        visited: &mut HashSet<ModuleId>,
        visiting: &mut HashSet<ModuleId>,
        sorted: &mut Vec<ModuleId>,
    ) -> Result<()> {
        if visiting.contains(id) {
            bail!("Circular dependency detected: {:?}", id);
        }
        
        if visited.contains(id) {
            return Ok(());
        }
        
        visiting.insert(id.clone());
        
        for dep in self.dependencies(id) {
            self.visit(dep, visited, visiting, sorted)?;
        }
        
        visiting.remove(id);
        visited.insert(id.clone());
        sorted.push(id.clone());
        
        Ok(())
    }
}
```

#### Algorithms

**Extract imports from .lean file:**
```rust
// lemma-build/src/import_parser.rs

use regex::Regex;

/// Parse imports from a .lean file
pub fn parse_imports(content: &str) -> Vec<String> {
    let mut imports = Vec::new();
    
    // Match "import Foo.Bar.Baz"
    let import_re = Regex::new(r"^\s*import\s+([A-Z][A-Za-z0-9.]*)")
        .expect("invalid regex");
    
    for line in content.lines() {
        // Stop at first non-import/non-comment line
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("--") {
            continue;
        }
        if !trimmed.starts_with("import") {
            break;
        }
        
        if let Some(caps) = import_re.captures(line) {
            if let Some(import) = caps.get(1) {
                imports.push(import.as_str().to_string());
            }
        }
    }
    
    imports
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_imports() {
        let content = r#"
import Std.Data.List
import Std.Data.HashMap
-- comment
import MyPackage.Utils

def main := IO.println "hello"
"#;
        
        let imports = parse_imports(content);
        assert_eq!(imports, vec![
            "Std.Data.List",
            "Std.Data.HashMap",
            "MyPackage.Utils",
        ]);
    }
}
```

**Build dependency graph:**
```rust
// lemma-build/src/module_resolver.rs

use anyhow::{Context, Result};
use std::collections::{HashMap, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};

pub struct ModuleResolver {
    root_dir: PathBuf,
    package_name: String,
    source_roots: Vec<PathBuf>,
}

impl ModuleResolver {
    pub fn new(
        root_dir: PathBuf,
        package_name: String,
        source_roots: Vec<PathBuf>,
    ) -> Self {
        ModuleResolver {
            root_dir,
            package_name,
            source_roots,
        }
    }
    
    /// Resolve all modules starting from entry points
    pub fn resolve(&self, entry_points: &[ModuleId]) -> Result<DependencyGraph> {
        let mut graph = DependencyGraph::new();
        let mut queue = VecDeque::new();
        let mut seen = HashSet::new();
        
        // Start with entry points
        for id in entry_points {
            queue.push_back(id.clone());
            seen.insert(id.clone());
        }
        
        // BFS to discover all modules
        while let Some(id) = queue.pop_front() {
            let module = self.load_module(&id)?;
            
            for import in &module.imports {
                if !seen.contains(import) {
                    queue.push_back(import.clone());
                    seen.insert(import.clone());
                }
            }
            
            graph.add_module(module);
        }
        
        Ok(graph)
    }
    
    /// Load a single module
    fn load_module(&self, id: &ModuleId) -> Result<Module> {
        let file_path = self.find_module_file(id)
            .with_context(|| format!("Module not found: {:?}", id))?;
        
        let content = fs::read_to_string(&file_path)
            .with_context(|| format!("Failed to read {}", file_path.display()))?;
        
        let import_strings = crate::import_parser::parse_imports(&content);
        let imports = import_strings
            .into_iter()
            .map(|s| ModuleId::from_import(&id.package, &s))
            .collect();
        
        Ok(Module {
            id: id.clone(),
            file_path,
            imports,
            hash: None, // Computed later
        })
    }
    
    /// Find the file path for a module
    fn find_module_file(&self, id: &ModuleId) -> Option<PathBuf> {
        let rel_path = id.to_file_path();
        
        for root in &self.source_roots {
            let full_path = self.root_dir.join(root).join(&rel_path);
            if full_path.exists() {
                return Some(full_path);
            }
        }
        
        None
    }
}
```

#### External Dependencies
- `regex` (v1.10): Pattern matching for import statements
- `walkdir` (v2.5): Recursive directory traversal
- `petgraph` (v0.6): Optional, for advanced graph algorithms

---

### 3.3 Build Cache

#### Purpose
Track file hashes and determine when rebuilds are needed.

#### Key Data Structures

```rust
// lemma-build/src/cache.rs

use blake3::Hash as Blake3Hash;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Content hash (256-bit BLAKE3)
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct Hash([u8; 32]);

impl Hash {
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Hash(bytes)
    }
    
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }
}

impl From<Blake3Hash> for Hash {
    fn from(hash: Blake3Hash) -> Self {
        Hash(*hash.as_bytes())
    }
}

/// Cache entry for a single module
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    /// Hash of source file content
    pub source_hash: Hash,
    
    /// Hashes of direct imports (transitive hash)
    pub import_hashes: Vec<Hash>,
    
    /// Combined hash (source + imports)
    pub combined_hash: Hash,
    
    /// Hash of build output (.olean file)
    pub output_hash: Option<Hash>,
    
    /// Timestamp of last build
    pub build_time: SystemTime,
}

/// Build cache
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct BuildCache {
    entries: HashMap<ModuleId, CacheEntry>,
}

impl BuildCache {
    pub fn new() -> Self {
        BuildCache {
            entries: HashMap::new(),
        }
    }
    
    /// Load cache from disk
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(BuildCache::new());
        }
        
        let content = fs::read_to_string(path)?;
        let cache = serde_json::from_str(&content)?;
        Ok(cache)
    }
    
    /// Save cache to disk
    pub fn save(&self, path: &Path) -> Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }
    
    /// Get cache entry for a module
    pub fn get(&self, id: &ModuleId) -> Option<&CacheEntry> {
        self.entries.get(id)
    }
    
    /// Update cache entry for a module
    pub fn update(&mut self, id: ModuleId, entry: CacheEntry) {
        self.entries.insert(id, entry);
    }
    
    /// Check if a module needs rebuilding
    pub fn needs_rebuild(
        &self,
        module: &Module,
        graph: &DependencyGraph,
    ) -> Result<bool> {
        // No cache entry = needs rebuild
        let Some(entry) = self.get(&module.id) else {
            return Ok(true);
        };
        
        // Check if source file changed
        let current_source_hash = compute_file_hash(&module.file_path)?;
        if current_source_hash != entry.source_hash {
            return Ok(true);
        }
        
        // Check if any import changed (transitive)
        let current_import_hashes = self.compute_import_hashes(module, graph)?;
        if current_import_hashes != entry.import_hashes {
            return Ok(true);
        }
        
        // Check if output exists
        let output_path = module.file_path.with_extension("olean");
        if !output_path.exists() {
            return Ok(true);
        }
        
        Ok(false)
    }
    
    /// Compute hashes of all imports (transitive)
    fn compute_import_hashes(
        &self,
        module: &Module,
        graph: &DependencyGraph,
    ) -> Result<Vec<Hash>> {
        let mut hashes = Vec::new();
        
        for import_id in &module.imports {
            if let Some(import_module) = graph.get_module(import_id) {
                if let Some(entry) = self.get(import_id) {
                    hashes.push(entry.combined_hash);
                } else {
                    // Import not in cache, use current hash
                    let hash = compute_file_hash(&import_module.file_path)?;
                    hashes.push(hash);
                }
            }
        }
        
        Ok(hashes)
    }
}

/// Compute BLAKE3 hash of a file
pub fn compute_file_hash(path: &Path) -> Result<Hash> {
    let content = fs::read(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    let hash = blake3::hash(&content);
    Ok(hash.into())
}

/// Compute combined hash (source + imports)
pub fn compute_combined_hash(source_hash: Hash, import_hashes: &[Hash]) -> Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&source_hash.0);
    for import_hash in import_hashes {
        hasher.update(&import_hash.0);
    }
    hasher.finalize().into()
}
```

#### Algorithms

**Incremental build detection:**
1. For each module in build order:
   - Compute current source hash
   - Compare with cached source hash
   - If different: needs rebuild
   - If same: check import hashes
     - For each import: get its combined hash from cache
     - Compare with cached import hashes
     - If any different: needs rebuild
   - If all same: check output exists
   - If output missing: needs rebuild
   - Otherwise: skip build

**Cache invalidation:**
- When a module is rebuilt, its combined hash changes
- All modules that depend on it (directly or transitively) need rebuilding
- Use dependency graph's reverse edges to find dependents

#### External Dependencies
- `blake3` (v1.5): Fast cryptographic hashing
- `serde_json` (v1.0): Cache serialization
- `hex` (v0.4): Hash display

---

### 3.4 Job Scheduler

#### Purpose
Execute build tasks in parallel while respecting dependencies.

#### Key Data Structures

```rust
// lemma-build/src/scheduler.rs

use std::sync::{Arc, Mutex};
use std::collections::{HashMap, VecDeque};
use tokio::sync::Semaphore;
use tokio::task::JoinHandle;

/// A build task
#[derive(Debug, Clone)]
pub struct BuildTask {
    pub id: ModuleId,
    pub dependencies: Vec<ModuleId>,
    pub kind: TaskKind,
}

#[derive(Debug, Clone)]
pub enum TaskKind {
    CompileModule,
    LinkExecutable { name: String },
    LinkLibrary { name: String },
}

/// Task status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

/// Job scheduler
pub struct JobScheduler {
    tasks: Vec<BuildTask>,
    status: HashMap<ModuleId, TaskStatus>,
    results: HashMap<ModuleId, Result<BuildOutput>>,
    max_jobs: usize,
}

impl JobScheduler {
    pub fn new(tasks: Vec<BuildTask>, max_jobs: usize) -> Self {
        let status = tasks
            .iter()
            .map(|t| (t.id.clone(), TaskStatus::Pending))
            .collect();
        
        JobScheduler {
            tasks,
            status,
            results: HashMap::new(),
            max_jobs,
        }
    }
    
    /// Execute all tasks
    pub async fn execute<F>(&mut self, executor: F) -> Result<()>
    where
        F: Fn(BuildTask) -> Result<BuildOutput> + Send + Sync + Clone + 'static,
    {
        let semaphore = Arc::new(Semaphore::new(self.max_jobs));
        let status = Arc::new(Mutex::new(self.status.clone()));
        let results = Arc::new(Mutex::new(HashMap::new()));
        
        let mut handles = Vec::new();
        
        // Launch tasks as dependencies complete
        for task in self.tasks.clone() {
            let semaphore = semaphore.clone();
            let status = status.clone();
            let results = results.clone();
            let executor = executor.clone();
            
            let handle = tokio::spawn(async move {
                // Wait for dependencies
                loop {
                    let deps_ready = {
                        let status = status.lock().unwrap();
                        task.dependencies.iter().all(|dep| {
                            matches!(
                                status.get(dep),
                                Some(TaskStatus::Completed)
                            )
                        })
                    };
                    
                    if deps_ready {
                        break;
                    }
                    
                    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                }
                
                // Acquire semaphore (rate limiting)
                let _permit = semaphore.acquire().await.unwrap();
                
                // Update status to running
                {
                    let mut status = status.lock().unwrap();
                    status.insert(task.id.clone(), TaskStatus::Running);
                }
                
                // Execute task
                let result = executor(task.clone());
                
                // Update status and results
                {
                    let mut status = status.lock().unwrap();
                    let mut results = results.lock().unwrap();
                    
                    match &result {
                        Ok(_) => {
                            status.insert(task.id.clone(), TaskStatus::Completed);
                        }
                        Err(_) => {
                            status.insert(task.id.clone(), TaskStatus::Failed);
                        }
                    }
                    
                    results.insert(task.id.clone(), result);
                }
            });
            
            handles.push(handle);
        }
        
        // Wait for all tasks
        for handle in handles {
            handle.await?;
        }
        
        // Check for failures
        let results = results.lock().unwrap();
        for (id, result) in results.iter() {
            if let Err(e) = result {
                bail!("Task {:?} failed: {}", id, e);
            }
        }
        
        Ok(())
    }
}
```

#### Algorithms

**Parallel task execution:**
1. Create task list with dependencies
2. For each task:
   - Wait for all dependencies to complete
   - Acquire semaphore permit (limit parallelism)
   - Execute task
   - Mark as completed
   - Release permit
3. Use tokio async runtime for efficient waiting
4. Use semaphore to limit max concurrent jobs

#### External Dependencies
- `tokio` (v1.35): Async runtime
- `rayon` (v1.10): Data parallelism (alternative approach)

---

### 3.5 Compilation Driver

#### Purpose
Invoke the Lean compiler to build modules.

#### Key Data Structures

```rust
// lemma-build/src/compiler.rs

use std::process::{Command, Stdio};
use std::path::{Path, PathBuf};

/// Compilation options
#[derive(Debug, Clone)]
pub struct CompileOptions {
    pub lean_path: Vec<PathBuf>,
    pub output_dir: PathBuf,
    pub extra_flags: Vec<String>,
}

/// Compilation output
#[derive(Debug, Clone)]
pub struct BuildOutput {
    pub olean_file: PathBuf,
    pub c_file: Option<PathBuf>,
    pub o_file: Option<PathBuf>,
}

/// Lean compiler driver
pub struct Compiler {
    lean_binary: PathBuf,
}

impl Compiler {
    pub fn new(lean_binary: PathBuf) -> Self {
        Compiler { lean_binary }
    }
    
    /// Compile a single module
    pub fn compile_module(
        &self,
        module: &Module,
        options: &CompileOptions,
    ) -> Result<BuildOutput> {
        let mut cmd = Command::new(&self.lean_binary);
        
        // Set LEAN_PATH for import resolution
        let lean_path = options.lean_path
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(":");
        cmd.env("LEAN_PATH", lean_path);
        
        // Output directory
        cmd.arg("--output-dir");
        cmd.arg(&options.output_dir);
        
        // Generate C code
        cmd.arg("-c");
        cmd.arg(&module.file_path);
        
        // Extra flags
        for flag in &options.extra_flags {
            cmd.arg(flag);
        }
        
        // Execute
        let output = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .with_context(|| format!("Failed to execute lean compiler"))?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("Compilation failed:\n{}", stderr);
        }
        
        // Determine output files
        let olean_file = module.file_path.with_extension("olean");
        let c_file = module.file_path.with_extension("c");
        let o_file = module.file_path.with_extension("o");
        
        Ok(BuildOutput {
            olean_file,
            c_file: Some(c_file),
            o_file: Some(o_file),
        })
    }
}
```

#### External Dependencies
- None (uses std::process)

---

### 3.6 Linker

#### Purpose
Link compiled objects into executables and libraries.

#### Key Data Structures

```rust
// lemma-build/src/linker.rs

use std::process::Command;

/// Linker for creating executables and libraries
pub struct Linker {
    cc: String, // C compiler/linker (clang or gcc)
}

impl Linker {
    pub fn new() -> Self {
        let cc = std::env::var("CC").unwrap_or_else(|_| "clang".to_string());
        Linker { cc }
    }
    
    /// Link object files into an executable
    pub fn link_executable(
        &self,
        name: &str,
        object_files: &[PathBuf],
        output_path: &Path,
    ) -> Result<()> {
        let mut cmd = Command::new(&self.cc);
        
        // Output file
        cmd.arg("-o");
        cmd.arg(output_path);
        
        // Input object files
        for obj in object_files {
            cmd.arg(obj);
        }
        
        // Link flags
        cmd.arg("-lm"); // Math library
        cmd.arg("-lpthread"); // Threading
        
        // Execute
        let output = cmd.output()?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("Linking failed:\n{}", stderr);
        }
        
        Ok(())
    }
    
    /// Link object files into a static library
    pub fn link_static_library(
        &self,
        name: &str,
        object_files: &[PathBuf],
        output_path: &Path,
    ) -> Result<()> {
        let mut cmd = Command::new("ar");
        
        cmd.arg("rcs");
        cmd.arg(output_path);
        
        for obj in object_files {
            cmd.arg(obj);
        }
        
        let output = cmd.output()?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("Archive creation failed:\n{}", stderr);
        }
        
        Ok(())
    }
}
```

---

## 4. Technical Challenges & Solutions

### 4.1 Parsing lakefile.lean vs lakefile.toml

**Challenge:** lakefile.lean is Lean code that defines build configuration using a DSL. Parsing requires understanding Lean syntax.

**Solutions:**

**Approach 1: TOML-first (Recommended for Phase 1)**
- Focus on lakefile.toml initially
- lakefile.toml is straightforward TOML format
- Most projects can use lakefile.toml
- Defer lakefile.lean support to later phase

**Approach 2: Lean interpreter (Future)**
- Use lean itself to evaluate lakefile.lean
- Execute `lean --eval` to extract configuration
- Parse JSON output from evaluation
- More complex but fully compatible

**Approach 3: Limited DSL parser (Future)**
- Parse subset of Lean syntax used in lakefiles
- Use tree-sitter-lean for parsing
- Extract key configuration calls
- Won't handle arbitrary Lean code

**Recommendation:** Start with TOML-only, add Lean support in Phase 8.

### 4.2 Discovering Module Imports

**Challenge:** Need to parse .lean files to extract import statements without fully parsing Lean syntax.

**Solution:**
- Imports are always at the top of files
- Simple regex matching: `^\s*import\s+([A-Z][A-Za-z0-9.]*)`
- Stop parsing after first non-import statement
- Fast and reliable for 99% of cases
- Edge case: imports in comments (rare, low priority)

**Implementation:**
```rust
fn parse_imports(content: &str) -> Vec<String> {
    let import_re = Regex::new(r"^\s*import\s+([A-Z][A-Za-z0-9.]*)").unwrap();
    content
        .lines()
        .take_while(|line| {
            let trimmed = line.trim();
            trimmed.is_empty() 
                || trimmed.starts_with("--") 
                || trimmed.starts_with("import")
        })
        .filter_map(|line| {
            import_re.captures(line)
                .and_then(|caps| caps.get(1))
                .map(|m| m.as_str().to_string())
        })
        .collect()
}
```

### 4.3 Detecting When Rebuilds Are Needed

**Challenge:** Determine minimal set of modules that need rebuilding.

**Solution: Hash-based incremental builds**

**Algorithm:**
1. Compute content hash of each source file (BLAKE3)
2. Compute transitive dependency hash:
   - Combined hash = hash(source_content + hash(dep1) + hash(dep2) + ...)
3. Compare with cached hashes
4. If any hash differs: rebuild
5. If output file missing: rebuild
6. Otherwise: skip

**Why BLAKE3:**
- Extremely fast (multiple GB/s)
- Cryptographically secure
- Better than SHA256 or BLAKE2

**Cache format (JSON):**
```json
{
  "Std.Data.List": {
    "source_hash": "abc123...",
    "import_hashes": ["def456...", "ghi789..."],
    "combined_hash": "jkl012...",
    "output_hash": "mno345...",
    "build_time": "2025-11-11T10:30:00Z"
  }
}
```

### 4.4 Handling Parallel Builds Safely

**Challenge:** Execute builds in parallel without race conditions.

**Solution: Dependency-aware job scheduling**

**Key insights:**
- Builds are data parallel (different modules)
- Dependencies create ordering constraints
- Use topological sort to respect dependencies
- Use semaphore to limit concurrency
- Use tokio for async scheduling

**Guarantees:**
- Module only builds after all dependencies complete
- No more than N jobs run concurrently (configurable)
- Failures propagate correctly
- Progress reporting is thread-safe

**Implementation approach:**
```rust
async fn execute_task(task: BuildTask) {
    // Wait for dependencies
    for dep in task.dependencies {
        await_completion(dep).await;
    }
    
    // Acquire permit (rate limiting)
    let _permit = semaphore.acquire().await;
    
    // Execute build
    compile_module(task.module)?;
    
    // Mark complete
    mark_complete(task.id);
}
```

### 4.5 Integrating with Different Lean Versions

**Challenge:** Different Lean versions may have different compiler flags, output formats, etc.

**Solution: Version detection and adaptation**

**Approach:**
1. Detect Lean version: `lean --version`
2. Parse version string
3. Adapt behavior based on version:
   - Compiler flags
   - Output file locations
   - Import resolution paths

**Example:**
```rust
fn get_compile_flags(lean_version: &Version) -> Vec<String> {
    let mut flags = Vec::new();
    
    if lean_version >= &Version::new(4, 3, 0) {
        flags.push("--new-flag".to_string());
    }
    
    flags
}
```

**Compatibility targets:**
- Lean 4.0.0+ (primary)
- Lake 4.0.0+ (for lakefile compatibility)
- Test against multiple versions in CI

### 4.6 Error Handling and Reporting

**Challenge:** Provide clear, actionable error messages.

**Solution: Layered error handling**

**Principles:**
1. Use `anyhow::Result` for propagating errors
2. Add context at each layer: `.context("helpful message")`
3. Pretty-print errors with source location
4. Show compiler output verbatim
5. Suggest fixes when possible

**Example:**
```rust
fn compile_module(module: &Module) -> Result<()> {
    let output = run_compiler(module)
        .with_context(|| format!("Failed to compile {}", module.id))?;
    
    if !output.success {
        // Show compiler errors
        eprintln!("{}", output.stderr);
        
        // Add hint
        eprintln!("hint: Check for syntax errors in {}", module.file_path.display());
        
        bail!("Compilation failed");
    }
    
    Ok(())
}
```

**Error categories:**
- Configuration errors (invalid lakefile)
- Dependency errors (circular, missing)
- Compilation errors (syntax, type errors)
- Linking errors (missing symbols)
- IO errors (file not found, permission denied)

---

## 5. File Structure

### 5.1 Proposed Crate Structure

```
lemma-rs/
├── crates/
│   ├── lemma-rs/           # Main binary (existing)
│   │   └── src/
│   │       ├── commands/
│   │       │   ├── build.rs    # Build command (enhanced)
│   │       │   └── clean.rs    # Clean command
│   │       └── main.rs
│   │
│   ├── lemma-build/        # NEW: Core build system
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── builder.rs       # High-level build orchestration
│   │   │   ├── cache.rs         # Build cache
│   │   │   ├── compiler.rs      # Lean compiler driver
│   │   │   ├── linker.rs        # Linker for executables/libraries
│   │   │   ├── scheduler.rs     # Job scheduler
│   │   │   ├── import_parser.rs # Import extraction
│   │   │   ├── module_resolver.rs # Module discovery
│   │   │   └── target.rs        # Build targets
│   │   ├── tests/
│   │   │   ├── integration.rs
│   │   │   └── fixtures/
│   │   │       └── sample_project/
│   │   └── Cargo.toml
│   │
│   ├── lemma-lakefile/     # NEW: Lakefile parsing
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── toml_parser.rs   # lakefile.toml parser
│   │   │   ├── lean_parser.rs   # lakefile.lean parser (future)
│   │   │   ├── schema.rs        # Configuration schema
│   │   │   └── validation.rs    # Configuration validation
│   │   ├── tests/
│   │   └── Cargo.toml
│   │
│   ├── lemma-graph/        # NEW: Dependency graph
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── graph.rs         # Graph data structure
│   │   │   ├── module.rs        # Module representation
│   │   │   ├── topo_sort.rs     # Topological sorting
│   │   │   └── visualization.rs # Debugging visualization
│   │   ├── tests/
│   │   └── Cargo.toml
│   │
│   ├── lemma-config/       # Existing (may need extensions)
│   ├── lemma-toolchain/    # Existing
│   ├── lemma-output/       # Existing
│   └── ...                 # Other existing crates
│
└── Cargo.toml              # Workspace manifest
```

### 5.2 Module Organization

**lemma-build/src/lib.rs:**
```rust
//! Core build system for Lean projects

mod builder;
mod cache;
mod compiler;
mod import_parser;
mod linker;
mod module_resolver;
mod scheduler;
mod target;

pub use builder::{Builder, BuildOptions};
pub use cache::{BuildCache, Hash};
pub use compiler::{Compiler, CompileOptions, BuildOutput};
pub use linker::Linker;
pub use module_resolver::ModuleResolver;
pub use target::{Target, TargetKind};
```

**Public API:**
```rust
use lemma_build::{Builder, BuildOptions};
use lemma_lakefile::LakefileConfig;

// High-level API
let config = LakefileConfig::load("lakefile.toml")?;
let builder = Builder::new(config)?;
builder.build(&BuildOptions::default())?;
```

---

## 6. Dependencies (Rust Crates)

### 6.1 Core Dependencies

| Crate | Version | Purpose | Justification |
|-------|---------|---------|---------------|
| `serde` | 1.0 | Serialization | Config parsing, cache persistence |
| `toml` | 0.8 | TOML parsing | lakefile.toml parsing |
| `serde_json` | 1.0 | JSON serialization | Cache format |
| `anyhow` | 1.0 | Error handling | Ergonomic error propagation |
| `thiserror` | 1.0 | Error types | Custom error types |
| `blake3` | 1.5 | Hashing | Fast content hashing for cache |
| `hex` | 0.4 | Hex encoding | Display hashes |
| `regex` | 1.10 | Pattern matching | Import parsing |
| `walkdir` | 2.5 | Directory traversal | Module discovery |
| `tokio` | 1.35 | Async runtime | Parallel job execution |
| `tracing` | 0.1 | Logging | Debugging and diagnostics |
| `fs-err` | 2.11 | Better file errors | Improved error messages |

### 6.2 Optional Dependencies

| Crate | Version | Purpose | Phase |
|-------|---------|---------|-------|
| `petgraph` | 0.6 | Graph algorithms | Phase 1 (optional) |
| `tree-sitter` | 0.20 | Parsing | Phase 8 (lakefile.lean) |
| `tree-sitter-lean` | - | Lean syntax | Phase 8 |
| `rayon` | 1.10 | Data parallelism | Alternative to tokio |
| `crossbeam` | 0.8 | Concurrent data structures | If needed |
| `notify` | 6.1 | File watching | Phase 8 (watch mode) |
| `indicatif` | 0.17 | Progress bars | Already used |

### 6.3 Why These Choices?

**BLAKE3 over SHA256:**
- 10x faster than SHA256
- Better security properties
- Widely used in build systems (e.g., Bazel)

**tokio over rayon:**
- Better for I/O-bound tasks (running compiler)
- Async/await is more natural for task dependencies
- Can integrate with async file I/O if needed
- rayon is great for CPU-bound data parallelism, but builds are I/O-bound

**toml over JSON:**
- More human-friendly for configuration
- Better comments support
- Standard for Rust projects
- Lake already uses TOML

**anyhow over custom error types:**
- Ergonomic for application code
- Great context chains
- Use thiserror for library error types

---

## 7. Compatibility Considerations

### 7.1 Lake Compatibility

**Goal:** lemma build should work as a drop-in replacement for `lake build` for most projects.

**Compatibility strategy:**

**Phase 1 (Essential):**
- lakefile.toml parsing
- Basic library targets
- Basic executable targets
- Module compilation
- Standard project layout

**Phase 2 (Important):**
- Dependency management (git dependencies)
- Custom build scripts
- External library linking
- Platform-specific builds

**Phase 3 (Advanced):**
- lakefile.lean support
- Lake plugins
- Custom facets
- Advanced scripting

### 7.2 Lake Features Priority

**Must have (Phase 1-2):**
- [x] Parse lakefile.toml
- [x] Compile Lean modules (.lean → .olean)
- [x] Generate C code (.lean → .c)
- [x] Compile C code (.c → .o)
- [x] Link executables
- [x] Link libraries
- [x] Incremental builds
- [x] Parallel builds
- [x] Git dependencies (basic)

**Should have (Phase 3-4):**
- [ ] lakefile.lean support
- [ ] Custom build scripts
- [ ] External library linking
- [ ] Precompiled binaries
- [ ] Cross-compilation
- [ ] Custom facets

**Nice to have (Phase 5+):**
- [ ] Lake plugins
- [ ] Build server (daemon)
- [ ] Watch mode
- [ ] Distributed builds
- [ ] Build tracing/profiling

### 7.3 Lake Version Differences

**Strategy:** Support Lake 4.0+

**Version detection:**
```rust
fn detect_lake_version(lakefile: &Path) -> Result<Version> {
    // Check lakefile format
    // Lake 3.x uses different TOML schema
    // Lake 4.x uses current schema
    
    // Parse version from lakefile if present
    // Fall back to lean-toolchain version
    
    Ok(Version::new(4, 0, 0))
}
```

**Adaptation:**
- Different default source directories
- Different output directory structure
- Different dependency formats

### 7.4 Migration Path

**For existing Lake projects:**
1. Keep lakefile.toml unchanged
2. `lemma build` should just work
3. If issues: report as bug
4. Can fall back to `lake build`

**For new projects:**
- Use `lemma init` to create project
- Generates lakefile.toml optimized for lemma
- But still compatible with Lake

---

## 8. Testing Strategy

### 8.1 Unit Tests

**Coverage target:** 80%+

**Test structure:**
```
crates/lemma-build/tests/
├── cache_tests.rs
├── compiler_tests.rs
├── graph_tests.rs
├── import_parser_tests.rs
├── linker_tests.rs
├── module_resolver_tests.rs
├── scheduler_tests.rs
└── fixtures/
    ├── sample_imports.lean
    └── sample_lakefile.toml
```

**Key test cases:**

**Import parser:**
- [ ] Single import
- [ ] Multiple imports
- [ ] Imports with comments
- [ ] Empty file
- [ ] Non-import lines

**Dependency graph:**
- [ ] Simple linear dependency
- [ ] Diamond dependency
- [ ] Circular dependency (error)
- [ ] Disconnected modules
- [ ] Large graph (1000+ modules)

**Build cache:**
- [ ] Cache miss (new file)
- [ ] Cache hit (unchanged)
- [ ] Invalidation (source changed)
- [ ] Transitive invalidation (dependency changed)
- [ ] Missing output file

**Job scheduler:**
- [ ] Simple sequential tasks
- [ ] Parallel independent tasks
- [ ] Tasks with dependencies
- [ ] Max concurrency limit
- [ ] Task failure propagation

### 8.2 Integration Tests

**Test with real Lean projects:**

**Tier 1 (Small projects):**
- [ ] Hello world (single file)
- [ ] Multi-file library
- [ ] Executable with dependencies
- [ ] Library with executable

**Tier 2 (Medium projects):**
- [ ] Project with git dependencies
- [ ] Project with external libraries
- [ ] Project with custom build scripts
- [ ] Multi-package workspace

**Tier 3 (Large projects):**
- [ ] Mathlib4 (very large library)
- [ ] Lean4 compiler itself
- [ ] ProofWidgets
- [ ] Aesop

**Test methodology:**
```rust
#[test]
fn test_build_hello_world() {
    let temp = TempDir::new()?;
    
    // Copy fixture project
    copy_fixture("hello_world", temp.path())?;
    
    // Run build
    let result = Builder::new(temp.path())?.build(&BuildOptions::default());
    
    // Check success
    assert!(result.is_ok());
    
    // Check output exists
    assert!(temp.path().join("build/bin/hello").exists());
    
    // Run executable
    let output = Command::new(temp.path().join("build/bin/hello"))
        .output()?;
    assert_eq!(output.stdout, b"Hello, world!\n");
}
```

### 8.3 Performance Tests

**Benchmarks:**
- [ ] Import parsing (1000 files)
- [ ] Dependency graph building (1000 modules)
- [ ] Cache lookup (1000 entries)
- [ ] Full build (clean)
- [ ] Incremental build (1 file changed)
- [ ] Incremental build (10 files changed)
- [ ] Parallel scaling (1, 2, 4, 8 cores)

**Comparison with Lake:**
```
Benchmark: Build Mathlib4
┌─────────────┬──────────┬──────────┬──────────┐
│             │ Clean    │ Incr (1) │ Incr (10)│
├─────────────┼──────────┼──────────┼──────────┤
│ Lake        │ 120 min  │ 0.5 min  │ 2 min    │
│ lemma build │ 100 min  │ 0.3 min  │ 1.5 min  │
│ Speedup     │ 1.2x     │ 1.7x     │ 1.3x     │
└─────────────┴──────────┴──────────┴──────────┘
```

### 8.4 CI/CD Testing

**GitHub Actions:**
```yaml
test-build-system:
  strategy:
    matrix:
      os: [ubuntu-latest, macos-latest, windows-latest]
      lean-version: [v4.0.0, v4.3.0, stable, nightly]
  steps:
    - uses: actions/checkout@v4
    - uses: actions-rs/toolchain@v1
    - name: Run tests
      run: cargo test --package lemma-build
    - name: Integration tests
      run: cargo test --test integration
```

**Coverage:**
```yaml
coverage:
  steps:
    - run: cargo tarpaulin --out Xml
    - uses: codecov/codecov-action@v3
```

### 8.5 Compatibility Testing

**Lake compatibility matrix:**
```
                Lake 4.0  Lake 4.3  Lake 4.5
lemma build      ✓         ✓         ✓
lakefile.toml    ✓         ✓         ✓
lakefile.lean    ✗         ✗         ✗ (Phase 8)
```

**Test procedure:**
1. Create project with Lake
2. Build with Lake
3. Build with lemma
4. Compare outputs (binaries should be identical or functionally equivalent)

---

## 9. Success Metrics

### 9.1 Functional Metrics

**Phase completion:**
- [ ] Phase 0-3: Basic build works
- [ ] Phase 4-6: Full feature parity with Lake (TOML)
- [ ] Phase 7-9: Advanced features and polish

**Project compatibility:**
- [ ] 90% of existing Lake projects build without changes
- [ ] 100% of lakefile.toml projects work
- [ ] 50% of lakefile.lean projects work (Phase 8)

### 9.2 Performance Metrics

**Build speed:**
- Clean builds: Within 20% of Lake
- Incremental builds: 1.5-2x faster than Lake (goal)
- Cache lookup: < 10ms for 1000 modules

**Resource usage:**
- Memory: < 500MB for typical project
- Disk: Cache < 10% of build output size
- CPU: Efficient parallel scaling

### 9.3 Quality Metrics

**Code quality:**
- Test coverage: > 80%
- No unsafe code (enforced by workspace lints)
- Zero known security vulnerabilities

**User experience:**
- Clear error messages
- Progress indication
- Good documentation
- Responsive to feedback

---

## 10. Risk Assessment

### 10.1 Technical Risks

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| Lake compatibility issues | High | Medium | Test against many projects, iterative fixes |
| Performance below Lake | Medium | Low | Benchmark early, optimize hot paths |
| Complexity underestimated | High | Medium | Phased approach, reassess after each phase |
| Lean version incompatibilities | Medium | Medium | Version detection, feature flags |
| Parallel build bugs | High | Medium | Extensive testing, conservative defaults |

### 10.2 Resource Risks

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| Development time too long | High | Medium | Phased delivery, MVP first |
| Insufficient testing | High | Low | Automated CI, community testing |
| Maintenance burden | Medium | Medium | Good documentation, modular design |

### 10.3 Adoption Risks

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| Users prefer Lake | High | Low | Ensure compatibility, better performance |
| Breaking changes in Lake | Medium | Medium | Monitor Lake development, adapt quickly |
| Insufficient documentation | Medium | Medium | Write docs alongside code |

---

## 11. Open Questions

### 11.1 Design Decisions

1. **Should we support lakefile.lean in Phase 1?**
   - Pro: Full Lake compatibility
   - Con: Significant complexity
   - **Decision:** No, focus on lakefile.toml first

2. **Should we use tokio or rayon for parallelism?**
   - tokio: Better for I/O, async/await
   - rayon: Better for CPU, simpler
   - **Decision:** tokio (builds are I/O-bound)

3. **Should we maintain our own dependency resolver?**
   - Pro: Full control, can optimize
   - Con: Complex, needs git integration
   - **Decision:** Phase 2, basic git deps first

4. **Should we generate identical binaries to Lake?**
   - Pro: Drop-in replacement
   - Con: May limit optimizations
   - **Decision:** Functionally equivalent, not bit-identical

### 11.2 Implementation Questions

1. How do we handle Lake plugins?
   - Phase 8, evaluate feasibility

2. How do we handle custom build scripts?
   - Execute via subprocess, sandboxing TBD

3. How do we handle cross-compilation?
   - Phase 4+, leverage Rust's cross support

4. How do we handle incremental linking?
   - Research needed, may use lld or mold

### 11.3 Community Questions

1. What Lake features do users care about most?
   - Survey Lean community

2. What are the biggest pain points with Lake?
   - Build speed, error messages, caching?

3. Would users adopt lemma build?
   - Depends on compatibility and performance

---

## 12. Next Steps

### 12.1 Immediate Actions (Week 1)

1. **Create crate structure**
   - [ ] Create lemma-build crate
   - [ ] Create lemma-lakefile crate
   - [ ] Create lemma-graph crate
   - [ ] Set up basic module structure
   - [ ] Configure workspace dependencies

2. **Define core data structures**
   - [ ] Package, Target, Module types
   - [ ] Dependency graph types
   - [ ] Build cache types
   - [ ] Write rustdoc comments

3. **Implement lakefile.toml parser**
   - [ ] Define TOML schema
   - [ ] Implement parser
   - [ ] Add validation
   - [ ] Write tests

### 12.2 Short Term (Weeks 2-4)

1. **Implement module resolver**
   - [ ] Import parser
   - [ ] Module discovery
   - [ ] Dependency graph builder
   - [ ] Cycle detection

2. **Implement build cache**
   - [ ] Hash computation
   - [ ] Cache persistence
   - [ ] Invalidation logic

3. **Basic compilation**
   - [ ] Compiler driver
   - [ ] Single module compilation
   - [ ] Error reporting

### 12.3 Medium Term (Weeks 5-12)

1. Complete Phase 1-3
2. Integration testing
3. Performance benchmarking
4. Bug fixes

### 12.4 Long Term (Months 4-7)

1. Complete Phase 4-9
2. Community feedback
3. Documentation
4. Release v1.0

---

## 13. Conclusion

This plan provides a comprehensive roadmap for implementing a native build system for lemma. The phased approach allows for incremental progress and early feedback, while the modular design ensures maintainability.

**Key Success Factors:**
1. Start with lakefile.toml (defer lakefile.lean)
2. Focus on compatibility with existing Lake projects
3. Optimize for incremental build performance
4. Test extensively with real-world projects
5. Maintain clear documentation

**Timeline Summary:**
- Phases 0-3: Months 1-2 (Basic functionality)
- Phases 4-6: Months 3-4 (Feature complete)
- Phases 7-9: Months 5-7 (Polish and advanced features)

**Next Review:** After Phase 3 completion, reassess timeline and priorities based on lessons learned.

---

## Appendix A: Lake Build System Overview

### A.1 Lake's Architecture

Lake uses a sophisticated build system with these key concepts:

**Facets:**
- Each module can have multiple "facets" (outputs)
- Common facets: `.olean` (compiled), `.c` (C code), `.o` (object)
- Facets can depend on other facets

**Build monad:**
- Recursive monad that tracks dependencies
- Memoization prevents redundant builds
- Cycle detection

**Trace system:**
- Hash-based change detection
- Transitive dependency tracking
- Stored in `.lake/build_trace.json`

**Job system:**
- Async parallel execution
- Spawns tasks for each build action
- Manages dependencies between tasks

### A.2 Lake's Lakefile Format

**lakefile.toml example:**
```toml
name = "MyPackage"
version = "0.1.0"

[[lean_lib]]
name = "MyLib"
roots = ["MyLib"]
globs = [{ pattern = "**/*.lean" }]

[[lean_exe]]
name = "my-exe"
root = "Main"
```

**lakefile.lean example:**
```lean
import Lake
open Lake DSL

package myPackage {
  -- Package configuration
}

@[default_target]
lean_lib MyLib {
  roots := #[`MyLib]
}

lean_exe myExe {
  root := `Main
}
```

### A.3 Lake's Build Process

1. Parse lakefile
2. Resolve dependencies (git clone if needed)
3. Discover modules
4. Build dependency graph
5. Topological sort
6. For each module (in parallel):
   - Check trace (hash)
   - If changed: compile
   - Update trace
7. Link executables/libraries

---

## Appendix B: UV Build System Comparison

UV's build system provides useful patterns for lemma:

**Good patterns to adopt:**
1. **Build dispatch:** Central coordinator that routes to appropriate builder
2. **Isolated builds:** Option to build in isolation (virtualenv for Python, separate LEAN_PATH for Lean)
3. **Caching strategy:** Hash-based cache with smart invalidation
4. **Workspace support:** Multi-package projects

**Differences from Lean:**
- Python: source builds (compile Python to bytecode)
- Lean: native compilation (compile Lean to C to binary)
- Python: wheel format (zip of files)
- Lean: executables and .olean files

**Key takeaway:** UV shows that rewriting a build system in Rust can provide significant performance improvements while maintaining compatibility.

---

## Appendix C: Reference Projects

### C.1 Build Systems to Study

1. **Cargo** (Rust)
   - Excellent incremental builds
   - Great error messages
   - Parallel compilation
   - Dependency resolution

2. **Bazel** (Google)
   - Hermetic builds
   - Remote caching
   - Content-addressable storage
   - Very scalable

3. **Buck2** (Meta)
   - Fast incremental builds
   - Virtual file system
   - Build graphs

4. **Turborepo** (Vercel)
   - Monorepo builds
   - Task scheduling
   - Caching strategy

### C.2 Relevant Crates

1. **cargo** (itself)
   - Study `cargo/src/cargo/core/compiler/`
   - Fingerprinting and caching
   - Job queue

2. **rustc** (Rust compiler)
   - Incremental compilation
   - Query system (memoization)
   - Parallel codegen

3. **ninja-rs**
   - Ninja build system in Rust
   - Efficient dependency tracking

---

## Appendix D: Glossary

| Term | Definition |
|------|------------|
| **Facet** | A specific output of a module build (e.g., .olean, .c, .o) |
| **Build trace** | Record of what was built and when, used for incremental builds |
| **Transitive hash** | Hash that includes dependencies' hashes |
| **Build monad** | Monadic structure for composing build actions |
| **Topological sort** | Ordering of nodes in a DAG respecting dependencies |
| **LEAN_PATH** | Environment variable for module search paths |
| **Lake** | Lean's build tool and package manager |
| **Lakefile** | Lake's configuration file (lakefile.toml or lakefile.lean) |
| **.olean** | Compiled Lean module file (binary) |
| **Module** | A single .lean source file and its compiled outputs |
| **Package** | A Lean project with a lakefile and dependencies |
| **Target** | Something to build (library, executable, etc.) |

---

**End of Document**

*This is a living document. It will be updated as implementation progresses and new requirements emerge.*
