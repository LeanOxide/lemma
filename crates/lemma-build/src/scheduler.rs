//! Job scheduler for parallel build execution
//!
//! This module implements a dependency-aware job scheduler that can execute
//! compilation tasks in parallel while respecting dependency order.

use crate::error::{Error, Result};
use crate::module::Module;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::{Mutex, Semaphore};

/// State of a build job
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobState {
    /// Job is waiting for dependencies
    Pending,
    /// Job is currently executing
    Running,
    /// Job completed successfully
    Completed,
    /// Job failed with an error
    Failed,
}

/// A build job represents a single compilation task
#[derive(Debug, Clone)]
pub struct BuildJob {
    /// Module to compile
    pub module: Module,
    /// Current state of the job
    pub state: JobState,
    /// Names of modules this job depends on
    pub dependencies: Vec<String>,
}

impl BuildJob {
    /// Create a new build job
    pub fn new(module: Module) -> Self {
        let dependencies = module.imports.clone();
        Self {
            module,
            state: JobState::Pending,
            dependencies,
        }
    }

    /// Check if this job is ready to run
    ///
    /// A job is ready if all its dependencies have completed successfully.
    pub fn is_ready(&self, completed: &HashSet<String>) -> bool {
        self.state == JobState::Pending
            && self
                .dependencies
                .iter()
                .all(|dep| completed.contains(dep))
    }
}

/// Parallel job scheduler with dependency tracking
pub struct JobScheduler {
    /// All jobs to execute
    jobs: HashMap<String, BuildJob>,
    /// Dependency graph for efficient dependency queries
    dependency_graph: Option<lemma_graph::DependencyGraph<String>>,
    /// Set of completed job names
    completed: Arc<Mutex<HashSet<String>>>,
    /// Set of failed job names
    failed: Arc<Mutex<HashSet<String>>>,
    /// Semaphore to limit concurrency
    semaphore: Arc<Semaphore>,
    /// Total number of jobs
    total_jobs: usize,
    /// Counter for completed jobs (for progress)
    completed_count: Arc<AtomicUsize>,
}

impl JobScheduler {
    /// Create a new job scheduler
    ///
    /// The concurrency parameter limits how many jobs can run in parallel.
    /// The dependency_graph parameter is used for efficient dependency queries.
    pub fn new(
        modules: Vec<Module>,
        concurrency: usize,
        dependency_graph: Option<lemma_graph::DependencyGraph<String>>,
    ) -> Self {
        let total_jobs = modules.len();
        let jobs: HashMap<String, BuildJob> = modules
            .into_iter()
            .map(|module| {
                let name = module.name.clone();
                (name, BuildJob::new(module))
            })
            .collect();

        Self {
            jobs,
            dependency_graph,
            completed: Arc::new(Mutex::new(HashSet::new())),
            failed: Arc::new(Mutex::new(HashSet::new())),
            semaphore: Arc::new(Semaphore::new(concurrency)),
            total_jobs,
            completed_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Execute all jobs in parallel, respecting dependencies
    ///
    /// This is the main entry point. It will spawn tasks for all jobs
    /// and wait for them to complete.
    ///
    /// The `progress_fn` is called after each job completes with:
    /// - module name
    /// - current job number (1-indexed)
    /// - total jobs
    /// - elapsed time in milliseconds
    pub async fn execute_all<F, Fut, P>(&mut self, job_fn: F, progress_fn: P) -> Result<()>
    where
        F: Fn(Module) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<()>> + Send + 'static,
        P: Fn(String, usize, usize, u128) + Send + Sync + 'static,
    {
        let progress_fn = Arc::new(progress_fn);
        let job_fn = Arc::new(job_fn);
        let mut handles = Vec::new();

        // Get all job names that need to be executed
        let job_names: Vec<String> = self.jobs.keys().cloned().collect();

        // Create a set of job names for fast lookup
        // This is used to filter out external dependencies that aren't part of this build
        let job_names_set: HashSet<String> = self.jobs.keys().cloned().collect();

        // Spawn tasks for each job
        for job_name in job_names {
            let job_fn = Arc::clone(&job_fn);
            let progress_fn = Arc::clone(&progress_fn);
            let completed = Arc::clone(&self.completed);
            let failed = Arc::clone(&self.failed);
            let semaphore = Arc::clone(&self.semaphore);
            let completed_count = Arc::clone(&self.completed_count);
            let total_jobs = self.total_jobs;

            // Clone the job data we need
            let module = self.jobs[&job_name].module.clone();

            // Get dependencies - use graph if available, otherwise fall back to job's dependency list
            let all_dependencies = if let Some(ref graph) = self.dependency_graph {
                // Use graph to query dependencies
                graph.dependencies(&job_name).unwrap_or(vec![])
            } else {
                // Fall back to dependencies stored in the job
                self.jobs[&job_name].dependencies.clone()
            };

            // Filter dependencies to only include those that are part of this build
            // External dependencies (e.g., Std, Init, etc.) are assumed to be already built
            let dependencies: Vec<String> = all_dependencies
                .into_iter()
                .filter(|dep| job_names_set.contains(dep))
                .collect();

            let handle = tokio::spawn(async move {
                // Wait for dependencies to complete
                loop {
                    let completed_set = completed.lock().await;
                    let failed_set = failed.lock().await;

                    // Check if any dependency failed
                    for dep in &dependencies {
                        if failed_set.contains(dep) {
                            eprintln!("[SCHEDULER] Skipping module '{}' because dependency '{}' failed", job_name, dep);
                            return Err(Error::Other(format!(
                                "Dependency '{}' failed, skipping '{}'",
                                dep, job_name
                            )));
                        }
                    }

                    // Check if all dependencies completed
                    let all_deps_complete = dependencies.iter().all(|dep| completed_set.contains(dep));

                    // Debug: Print waiting status (disabled for less noise)
                    // if !all_deps_complete {
                    //     eprintln!("[SCHEDULER] Module '{}' waiting for: {:?} (completed: {:?})",
                    //         job_name,
                    //         dependencies.iter().filter(|d| !completed_set.contains(*d)).collect::<Vec<_>>(),
                    //         completed_set.len()
                    //     );
                    // }

                    drop(completed_set);
                    drop(failed_set);

                    if all_deps_complete {
                        break;
                    }

                    // Wait a bit before checking again
                    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                }

                // Acquire semaphore permit (limits concurrency)
                let _permit = semaphore.acquire().await.unwrap();

                // Track start time
                let start_time = std::time::Instant::now();

                // Execute the job
                let result = job_fn(module.clone()).await;

                // Calculate elapsed time
                let elapsed = start_time.elapsed().as_millis();

                // Update state based on result
                match result {
                    Ok(()) => {
                        completed.lock().await.insert(job_name.clone());
                        let current = completed_count.fetch_add(1, Ordering::SeqCst) + 1;
                        progress_fn(job_name.clone(), current, total_jobs, elapsed);
                        Ok(())
                    }
                    Err(e) => {
                        eprintln!("[SCHEDULER] Module '{}' FAILED: {}", job_name, e);
                        failed.lock().await.insert(job_name.clone());
                        Err(e)
                    }
                }
            });

            handles.push(handle);
        }

        // Wait for all jobs to complete
        let results = futures::future::join_all(handles).await;

        // Check for errors
        let mut errors = Vec::new();
        for result in results {
            match result {
                Ok(Ok(())) => {}
                Ok(Err(e)) => errors.push(e),
                Err(e) => errors.push(Error::Other(format!("Task panicked: {}", e))),
            }
        }

        if !errors.is_empty() {
            return Err(Error::Other(format!(
                "Build failed with {} error(s): {}",
                errors.len(),
                errors
                    .iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            )));
        }

        Ok(())
    }

    /// Get the current state of all jobs
    pub fn get_states(&self) -> HashMap<String, JobState> {
        self.jobs
            .iter()
            .map(|(name, job)| (name.clone(), job.state))
            .collect()
    }

    /// Get statistics about job execution
    pub async fn get_stats(&self) -> JobStats {
        let completed = self.completed.lock().await;
        let failed = self.failed.lock().await;

        JobStats {
            total: self.jobs.len(),
            completed: completed.len(),
            failed: failed.len(),
            pending: self.jobs.len() - completed.len() - failed.len(),
        }
    }
}

/// Statistics about job execution
#[derive(Debug, Clone, Copy)]
pub struct JobStats {
    pub total: usize,
    pub completed: usize,
    pub failed: usize,
    pub pending: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn test_job_state() {
        let module = Module::new("Test".to_string(), PathBuf::from("Test.lean"), vec![]);
        let job = BuildJob::new(module);
        assert_eq!(job.state, JobState::Pending);
    }

    #[test]
    fn test_job_is_ready() {
        let module = Module::new(
            "B".to_string(),
            PathBuf::from("B.lean"),
            vec!["A".to_string()],
        );
        let job = BuildJob::new(module);

        // Not ready without dependencies
        let completed = HashSet::new();
        assert!(!job.is_ready(&completed));

        // Ready with dependencies
        let mut completed = HashSet::new();
        completed.insert("A".to_string());
        assert!(job.is_ready(&completed));
    }

    #[tokio::test]
    async fn test_scheduler_basic() {
        let modules = vec![
            Module::new("A".to_string(), PathBuf::from("A.lean"), vec![]),
            Module::new("B".to_string(), PathBuf::from("B.lean"), vec![]),
        ];

        let mut scheduler = JobScheduler::new(modules, 2, None);
        let counter = Arc::new(AtomicUsize::new(0));

        let counter_clone = Arc::clone(&counter);
        let job_fn = move |_module: Module| {
            let counter = Arc::clone(&counter_clone);
            async move {
                counter.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
        };

        let progress_fn = |_name: String, _current: usize, _total: usize, _elapsed: u128| {};

        scheduler.execute_all(job_fn, progress_fn).await.unwrap();

        assert_eq!(counter.load(Ordering::SeqCst), 2);
        let stats = scheduler.get_stats().await;
        assert_eq!(stats.completed, 2);
        assert_eq!(stats.failed, 0);
    }

    #[tokio::test]
    async fn test_scheduler_with_dependencies() {
        let modules = vec![
            Module::new("A".to_string(), PathBuf::from("A.lean"), vec![]),
            Module::new("B".to_string(), PathBuf::from("B.lean"), vec!["A".to_string()]),
            Module::new(
                "C".to_string(),
                PathBuf::from("C.lean"),
                vec!["A".to_string(), "B".to_string()],
            ),
        ];

        let mut scheduler = JobScheduler::new(modules, 2, None);
        let execution_order = Arc::new(Mutex::new(Vec::new()));

        let order_clone = Arc::clone(&execution_order);
        let job_fn = move |module: Module| {
            let order = Arc::clone(&order_clone);
            async move {
                order.lock().await.push(module.name.clone());
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                Ok(())
            }
        };

        let progress_fn = |_name: String, _current: usize, _total: usize, _elapsed: u128| {};

        scheduler.execute_all(job_fn, progress_fn).await.unwrap();

        let order = execution_order.lock().await;
        assert_eq!(order.len(), 3);

        // A must come before B
        let a_pos = order.iter().position(|x| x == "A").unwrap();
        let b_pos = order.iter().position(|x| x == "B").unwrap();
        assert!(a_pos < b_pos);

        // B must come before C
        let c_pos = order.iter().position(|x| x == "C").unwrap();
        assert!(b_pos < c_pos);

        // A must come before C
        assert!(a_pos < c_pos);
    }

    #[tokio::test]
    async fn test_scheduler_error_handling() {
        let modules = vec![
            Module::new("A".to_string(), PathBuf::from("A.lean"), vec![]),
            Module::new("B".to_string(), PathBuf::from("B.lean"), vec!["A".to_string()]),
        ];

        let mut scheduler = JobScheduler::new(modules, 2, None);

        let job_fn = move |module: Module| async move {
            if module.name == "A" {
                Err(Error::Other("Simulated failure".to_string()))
            } else {
                Ok(())
            }
        };

        let progress_fn = |_name: String, _current: usize, _total: usize, _elapsed: u128| {};

        let result = scheduler.execute_all(job_fn, progress_fn).await;
        assert!(result.is_err());

        let stats = scheduler.get_stats().await;
        assert!(stats.failed > 0);
    }
}
