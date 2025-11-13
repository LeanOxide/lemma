//! Run command - Build and execute the project's executable

use anyhow::{Context, Result};
use lemma_config::GlobalSettings;
use lemma_output::Printer;
use std::path::PathBuf;
use std::process::Command;

/// Build and execute the project's executable
pub fn execute(
    path: Option<&str>,
    bin_name: Option<&str>,
    args: &[String],
    settings: &GlobalSettings,
    printer: &Printer,
) -> Result<()> {
    // Determine the project directory
    let project_dir = if let Some(p) = path {
        PathBuf::from(p)
    } else {
        std::env::current_dir().context("Failed to get current directory")?
    };

    // Validate that this is a Lean project
    validate_lean_project(&project_dir)?;

    // Parse the lakefile to find executables
    let lakefile_path = project_dir.join("lakefile.toml");
    let lakefile_content =
        std::fs::read_to_string(&lakefile_path).context("Failed to read lakefile.toml")?;
    let lakefile: lemma_lakefile::Lakefile =
        lemma_lakefile::parse_toml(&lakefile_content).context("Failed to parse lakefile.toml")?;

    // Determine which executable to run
    let executable_name = if let Some(name) = bin_name {
        // User specified a binary name
        if !lakefile.executables.iter().any(|exe| exe.name == name) {
            anyhow::bail!(
                "Binary '{}' not found in lakefile.toml.\n\n\
                 Available executables: {}",
                name,
                lakefile
                    .executables
                    .iter()
                    .map(|e| e.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
        name.to_string()
    } else {
        // Use the first executable
        if lakefile.executables.is_empty() {
            anyhow::bail!(
                "No executables defined in lakefile.toml.\n\n\
                 Add an executable with:\n\
                 [[lean_exe]]\n\
                 name = \"my_exe\"\n\
                 root = \"Main\""
            );
        }
        lakefile.executables[0].name.clone()
    };

    // Build the project first using native build system
    if settings.is_verbose() {
        printer.hint(format!("Building project..."))?;
    }

    let build_result = crate::commands::build::execute(
        path,
        true,  // use native build system
        false, // no clear
        None,  // no custom out-dir
        &[],   // no specific targets
        settings,
        printer,
    );

    if let Err(e) = build_result {
        anyhow::bail!("Build failed: {}", e);
    }

    // Find the executable
    let build_dir = project_dir.join(&lakefile.build_dir);
    let executable_path = build_dir.join("bin").join(&executable_name);

    if !executable_path.exists() {
        anyhow::bail!(
            "Executable not found at: {}\n\n\
             The build may have succeeded but the executable was not created.",
            executable_path.display()
        );
    }

    // Show what we're running in verbose mode
    if settings.is_verbose() {
        printer.hint(format!("Running {} {}", executable_name, args.join(" ")))?;
    }

    // Run the executable
    let mut cmd = Command::new(&executable_path);
    cmd.args(args);

    // Inherit stdin/stdout/stderr for interactive execution
    cmd.stdin(std::process::Stdio::inherit());
    cmd.stdout(std::process::Stdio::inherit());
    cmd.stderr(std::process::Stdio::inherit());

    // Set working directory to project directory
    cmd.current_dir(&project_dir);

    // Set LEAN_PATH so the executable can find compiled .olean files
    let build_dir = project_dir.join(&lakefile.build_dir);
    let paths = lemma_build::BuildPaths::new(project_dir.clone(), build_dir);

    if let Some(lean_path) = lemma_build::LeanPathBuilder::new()
        .add_project_lib(&paths)
        .add_system_path()
        .build()
    {
        cmd.env("LEAN_PATH", lean_path);
    }

    // Run the command and wait for it to complete
    let status = cmd
        .status()
        .with_context(|| format!("Failed to execute: {}", executable_path.display()))?;

    // Exit with the same code as the executable
    if status.success() {
        Ok(())
    } else {
        std::process::exit(status.code().unwrap_or(1));
    }
}

/// Validate that the given directory contains a Lean project
fn validate_lean_project(dir: &std::path::Path) -> Result<()> {
    let lakefile = dir.join("lakefile.toml");
    if !lakefile.exists() {
        anyhow::bail!(
            "No lakefile.toml found in {}\n\n\
             This doesn't appear to be a Lean project.",
            dir.display()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use lemma_config::{ColorChoice, GlobalSettings};
    use lemma_output::Printer;
    use std::path::PathBuf;

    #[test]
    fn test_run_requires_lakefile() {
        let settings = GlobalSettings {
            verbose: 0,
            quiet: 0,
            color: ColorChoice::Auto,
            lemma_home: PathBuf::from("/tmp/lemma-test"),
        };
        let printer = Printer::new(false, false, false);

        let result = execute(Some("/nonexistent"), None, &[], &settings, &printer);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("lakefile.toml"));
    }
}
