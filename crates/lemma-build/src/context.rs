//! Build context - The main orchestrator for builds

use crate::cache::BuildCache;
use crate::compiler::CompilationDriver;
use crate::error::{Error, Result};
use crate::module::ModuleResolver;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use lemma_lakefile::Lakefile;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// The main build context that orchestrates the entire build process
///
/// This struct holds all the state needed for a build, including:
/// - The parsed lakefile
/// - The module resolver for discovering dependencies
/// - The build cache for incremental builds
/// - The build plan for execution
pub struct BuildContext {
    /// Project root directory
    pub project_dir: PathBuf,

    /// Parsed lakefile configuration
    pub lakefile: Lakefile,

    /// Module resolver for discovering imports
    pub module_resolver: ModuleResolver,

    /// Build cache for incremental builds
    pub cache: BuildCache,
}

impl BuildContext {
    /// Create a build context from a project directory
    ///
    /// This will:
    /// 1. Load and parse the lakefile
    /// 2. Initialize the module resolver
    /// 3. Load the build cache
    pub fn from_directory(project_dir: &Path) -> Result<Self> {
        let project_dir = project_dir.canonicalize()?;
        let lakefile = lemma_lakefile::load(&project_dir)?;

        let module_resolver = ModuleResolver::new(&project_dir, &lakefile)?;
        let cache = BuildCache::load(&project_dir)?;

        Ok(Self {
            project_dir,
            lakefile,
            module_resolver,
            cache,
        })
    }

    /// Execute the build
    ///
    /// This is the main entry point that orchestrates the entire build process:
    /// 1. Discover all modules and their dependencies
    /// 2. Create a build plan (topologically sorted)
    /// 3. Check the cache to determine what needs rebuilding
    /// 4. Execute compilation tasks in parallel
    /// 5. Link executables and libraries
    pub async fn build(&self) -> Result<()> {
        // Phase 1: Discover all modules
        let modules = self.module_resolver.discover_modules()?;

        if modules.is_empty() {
            return Err(Error::ModuleResolution(
                "No Lean modules found in project".to_string(),
            ));
        }

        // Phase 2: Create build plan (topologically sorted)
        let plan =
            crate::plan::BuildPlan::from_modules(modules, &self.module_resolver, &self.lakefile)?;

        // Phase 3: Check cache to determine what needs rebuilding
        let build_dir = self.project_dir.join(&self.lakefile.build_dir);
        let modules_to_build = self
            .cache
            .modules_needing_rebuild(&plan.modules, &build_dir)?;

        if modules_to_build.is_empty() {
            // Nothing to build
            return Ok(());
        }

        // Phase 4: Execute jobs in parallel using the scheduler
        let concurrency = num_cpus::get();

        // Save modules before they're moved into the scheduler
        let all_modules = plan.modules.clone();

        let mut scheduler = crate::scheduler::JobScheduler::new(
            plan.modules
                .into_iter()
                .filter(|m| modules_to_build.contains(&m.name))
                .collect(),
            concurrency,
        );

        // Phase 5: Set up compilation driver
        // Find the lean binary (assume it's in PATH for now; TODO: use lemma-toolchain)
        let lean_binary = which::which("lean").map_err(|e| {
            Error::Other(format!(
                "Could not find 'lean' binary in PATH. \
                 Please ensure Lean is installed and available. Error: {}",
                e
            ))
        })?;

        let driver = CompilationDriver::new(
            lean_binary,
            self.project_dir.join(&self.lakefile.src_dir),
            build_dir.clone(),
            self.lakefile.name.clone(),
        );
        let driver = std::sync::Arc::new(driver);

        // Clone for use in closure
        let driver_for_compile = std::sync::Arc::clone(&driver);
        let build_dir_for_compile = build_dir.clone();

        // Define the compilation job function
        let job_fn = move |module: crate::module::Module| {
            let driver = std::sync::Arc::clone(&driver_for_compile);
            let build_dir = build_dir_for_compile.clone();
            async move {
                // Compile the module
                driver.compile_module(&module, &build_dir).await?;
                Ok(())
            }
        };

        // Calculate total jobs including linking
        let total_jobs_including_linking = modules_to_build.len()
            + self.lakefile.executables.len()
            + self.lakefile.libraries.len();

        // Create multi-progress for managing multiple progress bars
        let multi_progress = Arc::new(MultiProgress::new());

        // Create main progress bar
        let main_pb = multi_progress.add(ProgressBar::new(total_jobs_including_linking as u64));
        main_pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner} [{pos}/{len}] {msg}")
                .unwrap(),
        );

        let main_pb_clone = main_pb.clone();

        // Define progress callback
        let progress_fn =
            move |module_name: String, current: usize, _total: usize, elapsed_ms: u128| {
                main_pb_clone.set_message(format!("Running {} ({}ms)", module_name, elapsed_ms));
                main_pb_clone.set_position(current as u64);
            };

        // Execute all compilation jobs
        scheduler.execute_all(job_fn, progress_fn).await?;

        // TODO: Phase 5 - Update build cache with new hashes

        // Phase 6: Link executables and libraries
        // Link all executables defined in the lakefile
        for executable in &self.lakefile.executables {
            let output_path = build_dir.join("bin").join(&executable.name);

            let start = std::time::Instant::now();
            // For now, link all modules (TODO: filter by executable.root)
            driver
                .link_executable(&executable.name, &all_modules, &output_path)
                .await?;
            let elapsed = start.elapsed().as_millis();

            main_pb.set_message(format!("Running {} ({}ms)", executable.name, elapsed));
            main_pb.inc(1);
        }

        // Link all libraries defined in the lakefile
        for library in &self.lakefile.libraries {
            let lib_name = format!("lib{}.a", library.name);
            let output_path = build_dir.join("lib").join(&lib_name);

            let start = std::time::Instant::now();
            // For now, link all modules (TODO: filter by library.root)
            driver
                .link_library(&library.name, &all_modules, &output_path)
                .await?;
            let elapsed = start.elapsed().as_millis();

            main_pb.set_message(format!("Running {} ({}ms)", library.name, elapsed));
            main_pb.inc(1);
        }

        // Finish progress bar
        main_pb.finish_with_message(format!(
            "Build completed successfully ({} jobs)",
            total_jobs_including_linking
        ));

        Ok(())
    }

    /// Clean the build directory
    pub fn clean(&self) -> Result<()> {
        let build_dir = self.project_dir.join(&self.lakefile.build_dir);
        if build_dir.exists() {
            std::fs::remove_dir_all(&build_dir)?;
        }
        Ok(())
    }
}
