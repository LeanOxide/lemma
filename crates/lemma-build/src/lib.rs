//! Lemma build system
//!
//! This crate implements a native build system for Lean projects, inspired by
//! Lake but implemented from scratch in Rust for better performance and integration
//! with lemma's toolchain management.
//!
//! # Architecture
//!
//! The build system follows a pipeline architecture:
//! 1. **Lakefile Parsing**: Load and parse project configuration
//! 2. **Module Resolution**: Discover .lean files and parse imports
//! 3. **Build Planning**: Create dependency graph and topological sort
//! 4. **Build Cache**: Check hashes to determine what needs rebuilding
//! 5. **Job Scheduling**: Execute compilation tasks in parallel
//! 6. **Compilation**: Invoke lean compiler with correct flags
//! 7. **Linking**: Link compiled artifacts into executables/libraries
//!
//! # Example
//!
//! ```rust,no_run
//! use lemma_build::BuildContext;
//! use std::path::Path;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let context = BuildContext::from_directory(Path::new("."))?;
//! context.build().await?;
//! # Ok(())
//! # }
//! ```

pub mod cache;
pub mod compiler;
pub mod context;
pub mod error;
pub mod facets;
pub mod module;
pub mod plan;
pub mod scheduler;
pub mod target;

pub use cache::BuildCache;
pub use compiler::CompilationDriver;
pub use context::BuildContext;
pub use error::{Error, Result};
pub use facets::FacetBuilder;
pub use module::{Module, ModuleResolver};
pub use plan::BuildPlan;
pub use scheduler::{BuildJob, JobScheduler, JobState, JobStats};
pub use target::{BuildTarget, Facet, TargetSpec};

/// Build a Lean project from the given directory
///
/// This is the main entry point for building a project. It will:
/// 1. Load the lakefile
/// 2. Discover all modules
/// 3. Build the dependency graph
/// 4. Determine what needs to be built
/// 5. Execute the build in parallel
///
/// # Example
///
/// ```rust,no_run
/// # async fn example() -> anyhow::Result<()> {
/// lemma_build::build(std::path::Path::new(".")).await?;
/// # Ok(())
/// # }
/// ```
pub async fn build(project_dir: &std::path::Path) -> Result<()> {
    let context = BuildContext::from_directory(project_dir)?;
    context.build().await
}
