//! Facet builder - Builds specific artifacts for modules, libraries, and executables
//!
//! This module implements the build logic for different facets (build artifacts).
//! Each facet represents a specific type of output from the build process.

use crate::compiler::CompilationDriver;
use crate::error::{Error, Result};
use crate::module::Module;
use crate::target::{BuildTarget, Facet};
use lemma_lakefile::{ExecutableTarget, LibraryTarget};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Builder for specific build facets
pub struct FacetBuilder {
    driver: Arc<CompilationDriver>,
    build_dir: PathBuf,
    modules: Vec<Module>,
}

impl FacetBuilder {
    /// Create a new facet builder
    pub fn new(driver: Arc<CompilationDriver>, build_dir: PathBuf, modules: Vec<Module>) -> Self {
        Self {
            driver,
            build_dir,
            modules,
        }
    }

    /// Build the specified target
    pub async fn build(&self, target: &BuildTarget) -> Result<Vec<PathBuf>> {
        match target {
            BuildTarget::Module { module, facet } => self.build_module_facet(module, facet).await,
            BuildTarget::Library { library, facet } => {
                self.build_library_facet(library, facet).await
            }
            BuildTarget::Executable { executable } => self.build_executable(executable).await,
            BuildTarget::Package { facet: _ } => {
                // For package target, we build all modules
                self.build_all_modules().await
            }
        }
    }

    /// Build a specific facet for a module
    fn build_module_facet<'a>(
        &'a self,
        module: &'a Module,
        facet: &'a Facet,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<PathBuf>>> + 'a>> {
        Box::pin(async move {
            match facet {
                Facet::Deps => {
                    // Build all dependencies first
                    let mut built = Vec::new();
                    for import in &module.imports {
                        if let Some(dep_module) = self.modules.iter().find(|m| &m.name == import) {
                            // Recursively build dependency with LeanArts facet
                            built.extend(
                                self.build_module_facet(dep_module, &Facet::LeanArts)
                                    .await?,
                            );
                        }
                    }
                    Ok(built)
                }

                Facet::LeanArts => {
                    // Build .olean, .ilean, and .c files
                    self.driver.compile_module(module, &self.build_dir).await?;

                    Ok(vec![
                        self.get_olean_path(module),
                        self.get_ilean_path(module),
                        self.get_c_path(module),
                    ])
                }

                Facet::Olean => {
                    // Build only .olean file
                    self.driver.compile_module(module, &self.build_dir).await?;
                    Ok(vec![self.get_olean_path(module)])
                }

                Facet::Ilean => {
                    // Build only .ilean file
                    self.driver.compile_module(module, &self.build_dir).await?;
                    Ok(vec![self.get_ilean_path(module)])
                }

                Facet::C => {
                    // Build only .c file
                    self.driver.compile_module(module, &self.build_dir).await?;
                    Ok(vec![self.get_c_path(module)])
                }

                Facet::Bc => {
                    // Build LLVM bitcode file (requires LLVM backend)
                    // For now, return error as this requires LLVM support
                    Err(Error::Other(
                        "LLVM bitcode generation not yet implemented".to_string(),
                    ))
                }

                Facet::O | Facet::CO => {
                    // Build object file from C
                    self.driver.compile_module(module, &self.build_dir).await?;
                    let c_file = self.get_c_path(module);
                    let o_file = self.get_object_path(module);

                    // Compile C to object file
                    self.compile_c_to_object(&c_file, &o_file).await?;

                    Ok(vec![o_file])
                }

                Facet::BcO => {
                    // Build object file from LLVM bitcode
                    Err(Error::Other(
                        "LLVM bitcode compilation not yet implemented".to_string(),
                    ))
                }

                Facet::Dynlib => {
                    // Build shared library for dynamic loading
                    Err(Error::Other(
                        "Dynamic library generation not yet implemented".to_string(),
                    ))
                }

                _ => Err(Error::InvalidTarget(format!(
                    "Facet {:?} not supported for modules",
                    facet
                ))),
            }
        })
    }

    /// Build a specific facet for a library
    async fn build_library_facet(
        &self,
        library: &LibraryTarget,
        facet: &Facet,
    ) -> Result<Vec<PathBuf>> {
        match facet {
            Facet::LeanArts => {
                // Build all modules in the library
                let lib_modules = self.get_library_modules(library);
                let mut built = Vec::new();
                for module in &lib_modules {
                    built.extend(self.build_module_facet(module, &Facet::LeanArts).await?);
                }
                Ok(built)
            }

            Facet::Static => {
                // Build static library (.a)
                let lib_modules = self.get_library_modules(library);
                let lib_name = format!("lib{}.a", library.name);
                let output_path = self.build_dir.join("lib").join(&lib_name);

                self.driver
                    .link_library(&library.name, &lib_modules, &output_path)
                    .await?;

                Ok(vec![output_path])
            }

            Facet::Shared => {
                // Build shared library (.so, .dll, .dylib)
                Err(Error::Other(
                    "Shared library generation not yet implemented".to_string(),
                ))
            }

            _ => Err(Error::InvalidTarget(format!(
                "Facet {:?} not supported for libraries",
                facet
            ))),
        }
    }

    /// Build an executable
    async fn build_executable(&self, executable: &ExecutableTarget) -> Result<Vec<PathBuf>> {
        let output_path = self.build_dir.join("bin").join(&executable.name);

        // Find all modules needed for this executable
        // For now, link all modules (TODO: filter by executable.root)
        self.driver
            .link_executable(&executable.name, &self.modules, &output_path)
            .await?;

        Ok(vec![output_path])
    }

    /// Build all modules with default facets
    async fn build_all_modules(&self) -> Result<Vec<PathBuf>> {
        let mut built = Vec::new();
        for module in &self.modules {
            built.extend(self.build_module_facet(module, &Facet::LeanArts).await?);
        }
        Ok(built)
    }

    /// Get the path for a module's .olean artifact
    fn get_olean_path(&self, module: &Module) -> PathBuf {
        // Lake structure: `.lake/build/lib/<package>/Module.olean`
        let parts: Vec<&str> = module.name.split('.').collect();
        let mut path = self.build_dir.join("lib");
        // Note: We'd need package name here, but for now just use hierarchical structure
        for part in parts {
            path = path.join(part);
        }
        path.set_extension("olean");
        path
    }

    /// Get the path for a module's .ilean artifact
    fn get_ilean_path(&self, module: &Module) -> PathBuf {
        let parts: Vec<&str> = module.name.split('.').collect();
        let mut path = self.build_dir.join("lib");
        for part in parts {
            path = path.join(part);
        }
        path.set_extension("ilean");
        path
    }

    /// Get the C file path for a module
    fn get_c_path(&self, module: &Module) -> PathBuf {
        // Lake structure: `.lake/build/ir/Module/Nested.c` (hierarchical)
        let parts: Vec<&str> = module.name.split('.').collect();
        let mut path = self.build_dir.join("ir");
        for part in parts {
            path = path.join(part);
        }
        path.set_extension("c");
        path
    }

    /// Get the object file path for a module
    fn get_object_path(&self, module: &Module) -> PathBuf {
        // Lake structure: `.lake/build/ir/Module/Nested.o` (hierarchical)
        let parts: Vec<&str> = module.name.split('.').collect();
        let mut path = self.build_dir.join("ir");
        for part in parts {
            path = path.join(part);
        }
        path.set_extension("o");
        path
    }

    /// Get all modules that belong to a library
    fn get_library_modules(&self, _library: &LibraryTarget) -> Vec<Module> {
        // For now, return all modules
        // TODO: Filter by library.root and library.globs
        self.modules.clone()
    }

    /// Compile a C file to an object file
    async fn compile_c_to_object(&self, c_file: &Path, o_file: &Path) -> Result<()> {
        use tokio::process::Command;

        // Ensure parent directory exists
        if let Some(parent) = o_file.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                Error::Compilation(format!(
                    "Failed to create directory {}: {}",
                    parent.display(),
                    e
                ))
            })?;
        }

        // Find leanc or use system compiler
        let compiler = which::which("leanc")
            .or_else(|_| which::which("gcc"))
            .or_else(|_| which::which("clang"))
            .map_err(|_| {
                Error::Compilation("No C compiler found (tried leanc, gcc, clang)".to_string())
            })?;

        let output = Command::new(&compiler)
            .arg("-c")
            .arg(c_file)
            .arg("-o")
            .arg(o_file)
            .output()
            .await
            .map_err(|e| {
                Error::Compilation(format!(
                    "Failed to run C compiler {}: {}",
                    compiler.display(),
                    e
                ))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(Error::Compilation(format!(
                "C compilation failed for {}: {}",
                c_file.display(),
                stderr
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_olean_path() {
        let driver = Arc::new(CompilationDriver::new(
            PathBuf::from("lean"),
            PathBuf::from("src"),
            PathBuf::from(".lake/build"),
            "test".to_string(),
        ));
        let builder = FacetBuilder::new(driver, PathBuf::from(".lake/build"), vec![]);

        let module = Module::new(
            "Foo.Bar.Baz".to_string(),
            PathBuf::from("Foo/Bar/Baz.lean"),
            vec![],
        );

        let olean_path = builder.get_olean_path(&module);
        assert_eq!(
            olean_path,
            PathBuf::from(".lake/build/lib/Foo/Bar/Baz.olean")
        );
    }

    #[test]
    fn test_get_c_path() {
        let driver = Arc::new(CompilationDriver::new(
            PathBuf::from("lean"),
            PathBuf::from("src"),
            PathBuf::from(".lake/build"),
            "test".to_string(),
        ));
        let builder = FacetBuilder::new(driver, PathBuf::from(".lake/build"), vec![]);

        let module = Module::new("Foo.Bar".to_string(), PathBuf::from("Foo/Bar.lean"), vec![]);

        let c_path = builder.get_c_path(&module);
        assert_eq!(c_path, PathBuf::from(".lake/build/ir/Foo/Bar.c"));
    }
}
