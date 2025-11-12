//! Build plan - Determines what to build and in what order

use crate::error::Result;
use crate::module::{Module, ModuleResolver};
use lemma_lakefile::Lakefile;

/// A build plan specifies what needs to be built and in what order
#[derive(Debug, Clone)]
pub struct BuildPlan {
    /// Modules to build, in topological order (dependencies first)
    pub modules: Vec<Module>,

    /// Dependency graph for the modules
    /// This is stored to avoid rebuilding it multiple times
    pub dependency_graph: lemma_graph::DependencyGraph<String>,

    /// Executables to link (from lakefile)
    pub executables: Vec<String>,

    /// Libraries to build (from lakefile)
    pub libraries: Vec<String>,
}

impl BuildPlan {
    /// Create a new empty build plan
    pub fn new() -> Self {
        Self {
            modules: Vec::new(),
            dependency_graph: lemma_graph::DependencyGraph::new(),
            executables: Vec::new(),
            libraries: Vec::new(),
        }
    }

    /// Create a build plan from discovered modules and lakefile
    ///
    /// This will:
    /// 1. Build a dependency graph from module imports
    /// 2. Topologically sort the modules
    /// 3. Determine which executables and libraries to build from the lakefile
    pub fn from_modules(
        modules: Vec<Module>,
        resolver: &ModuleResolver,
        lakefile: &Lakefile,
    ) -> Result<Self> {
        // Build dependency graph
        let graph = resolver.build_dependency_graph(&modules)?;

        // Topologically sort the modules (dependencies first)
        let sorted_names = graph.topological_sort()?;

        // Reorder modules according to topological sort
        let mut sorted_modules = Vec::new();
        for name in &sorted_names {
            if let Some(module) = modules.iter().find(|m| &m.name == name) {
                sorted_modules.push(module.clone());
            }
        }

        // Extract executables and libraries from lakefile
        let executables = lakefile
            .executables
            .iter()
            .map(|exe| exe.name.clone())
            .collect();

        let libraries = lakefile
            .libraries
            .iter()
            .map(|lib| lib.name.clone())
            .collect();

        Ok(Self {
            modules: sorted_modules,
            dependency_graph: graph,
            executables,
            libraries,
        })
    }

    /// Get the number of build tasks
    pub fn task_count(&self) -> usize {
        self.modules.len() + self.executables.len() + self.libraries.len()
    }

    /// Check if there's anything to build
    pub fn is_empty(&self) -> bool {
        self.modules.is_empty() && self.executables.is_empty() && self.libraries.is_empty()
    }
}

impl Default for BuildPlan {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::module::Module;
    use std::path::PathBuf;

    #[test]
    fn test_empty_build_plan() {
        let plan = BuildPlan::new();
        assert!(plan.is_empty());
        assert_eq!(plan.task_count(), 0);
    }

    #[test]
    fn test_build_plan_task_count() {
        let mut graph = lemma_graph::DependencyGraph::new();
        graph.add_node_if_missing("A".to_string());
        graph.add_node_if_missing("B".to_string());

        let plan = BuildPlan {
            modules: vec![
                Module::new("A".to_string(), PathBuf::from("A.lean"), vec![]),
                Module::new("B".to_string(), PathBuf::from("B.lean"), vec![]),
            ],
            dependency_graph: graph,
            executables: vec!["main".to_string()],
            libraries: vec!["mylib".to_string()],
        };

        assert_eq!(plan.task_count(), 4); // 2 modules + 1 exe + 1 lib
        assert!(!plan.is_empty());
    }
}
