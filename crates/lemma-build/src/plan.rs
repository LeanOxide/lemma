//! Build plan - Determines what to build and in what order

use crate::error::Result;
use crate::module::Module;

/// A build plan specifies what needs to be built and in what order
#[derive(Debug, Clone)]
pub struct BuildPlan {
    /// Modules to build, in topological order (dependencies first)
    pub modules: Vec<Module>,

    /// Executables to link
    pub executables: Vec<String>,

    /// Libraries to build
    pub libraries: Vec<String>,
}

impl BuildPlan {
    /// Create a new empty build plan
    pub fn new() -> Self {
        Self {
            modules: Vec::new(),
            executables: Vec::new(),
            libraries: Vec::new(),
        }
    }

    /// Create a build plan from discovered modules
    ///
    /// This will:
    /// 1. Build a dependency graph from module imports
    /// 2. Topologically sort the modules
    /// 3. Determine which executables and libraries to build
    ///
    /// TODO: Implement in Phase 2
    pub fn from_modules(_modules: Vec<Module>) -> Result<Self> {
        // TODO: Phase 2 - Build dependency graph
        // TODO: Phase 2 - Topological sort
        // TODO: Phase 2 - Identify executables and libraries
        Ok(Self::new())
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
