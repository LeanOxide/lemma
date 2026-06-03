//! Test utilities and helpers for integration tests

use lemma_static::EnvVars;
use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tempfile::TempDir;

/// Test context for running lemma commands
pub struct LemmaTestContext {
    pub temp_dir: TempDir,
    pub lemma_home: PathBuf,
    pub bin_dir: PathBuf,
    pub toolchains_dir: PathBuf,
    pub lemma_exe: PathBuf,
}

impl LemmaTestContext {
    /// Create a new test context with a temporary directory
    pub fn new() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let lemma_home = temp_dir.path().join(".lemma");
        let bin_dir = lemma_home.join("bin");
        let toolchains_dir = lemma_home.join("toolchains");

        // Get the path to the lemma executable
        let lemma_exe = env::current_exe()
            .expect("Failed to get current exe")
            .parent()
            .expect("Failed to get parent dir")
            .parent()
            .expect("Failed to get grandparent dir")
            .join("lemma");

        Self {
            temp_dir,
            lemma_home,
            bin_dir,
            toolchains_dir,
            lemma_exe,
        }
    }

    /// Run a lemma command with the test environment
    pub fn run(&self, args: &[&str]) -> CommandResult {
        let mut cmd = Command::new(&self.lemma_exe);
        cmd.args(args)
            .env(EnvVars::LEMMA_HOME, &self.lemma_home)
            .env("PATH", env::var("PATH").unwrap_or_default());

        let output = cmd.output().expect("Failed to execute command");
        CommandResult::from_output(output)
    }

    /// Run a lemma command with additional environment variables
    pub fn run_with_env(&self, args: &[&str], env_vars: &[(&str, &str)]) -> CommandResult {
        let mut cmd = Command::new(&self.lemma_exe);
        cmd.args(args)
            .env(EnvVars::LEMMA_HOME, &self.lemma_home)
            .env("PATH", env::var("PATH").unwrap_or_default());

        for (key, value) in env_vars {
            cmd.env(key, value);
        }

        let output = cmd.output().expect("Failed to execute command");
        CommandResult::from_output(output)
    }

    /// Get the path to the lemma home directory
    pub fn lemma_home(&self) -> &Path {
        &self.lemma_home
    }

    /// Get the path to the bin directory
    pub fn bin_dir(&self) -> &Path {
        &self.bin_dir
    }

    /// Get the path to the toolchains directory
    pub fn toolchains_dir(&self) -> &Path {
        &self.toolchains_dir
    }

    /// Check if a file exists in the lemma home directory
    pub fn exists(&self, path: &str) -> bool {
        self.lemma_home.join(path).exists()
    }

    /// Read a file from the lemma home directory
    pub fn read_file(&self, path: &str) -> String {
        std::fs::read_to_string(self.lemma_home.join(path)).expect("Failed to read file")
    }
}

/// Result of running a command
pub struct CommandResult {
    pub status_code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl CommandResult {
    fn from_output(output: Output) -> Self {
        Self {
            status_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        }
    }

    /// Check if the command succeeded
    pub fn success(&self) -> bool {
        self.status_code == 0
    }

    /// Assert that the command succeeded
    pub fn assert_success(&self) {
        if !self.success() {
            panic!(
                "Command failed with status code {}\nstdout: {}\nstderr: {}",
                self.status_code, self.stdout, self.stderr
            );
        }
    }

    /// Assert that the command failed
    pub fn assert_failed(&self) {
        if self.success() {
            panic!(
                "Expected command to fail, but it succeeded\nstdout: {}\nstderr: {}",
                self.stdout, self.stderr
            );
        }
    }

    /// Assert that stdout contains a string
    pub fn assert_stdout_contains(&self, s: &str) {
        if !self.stdout.contains(s) {
            panic!(
                "Expected stdout to contain '{}', but it didn't.\nstdout: {}",
                s, self.stdout
            );
        }
    }

    /// Assert that stderr contains a string
    pub fn assert_stderr_contains(&self, s: &str) {
        if !self.stderr.contains(s) {
            panic!(
                "Expected stderr to contain '{}', but it didn't.\nstderr: {}",
                s, self.stderr
            );
        }
    }

    /// Assert that stdout does not contain a string
    pub fn assert_stdout_not_contains(&self, s: &str) {
        if self.stdout.contains(s) {
            panic!(
                "Expected stdout to not contain '{}', but it did.\nstdout: {}",
                s, self.stdout
            );
        }
    }

    /// Assert that stderr does not contain a string
    pub fn assert_stderr_not_contains(&self, s: &str) {
        if self.stderr.contains(s) {
            panic!(
                "Expected stderr to not contain '{}', but it did.\nstderr: {}",
                s, self.stderr
            );
        }
    }
}

/// Test setup trait for common test setup operations
pub trait TestSetup {
    /// Create a fake toolchain for testing
    fn create_fake_toolchain(&self, name: &str);
}

impl TestSetup for LemmaTestContext {
    fn create_fake_toolchain(&self, name: &str) {
        // Convert toolchain name to directory name using the proper ToolchainDesc format
        // New format: "stable" -> "stable-linux", "v4.24.0" -> "v4.24.0-linux"
        let desc =
            lemma_toolchain::ToolchainDesc::parse(name).expect("Failed to parse toolchain name");
        let dir_name = desc.to_directory_name();
        let toolchain_dir = self.toolchains_dir.join(&dir_name);
        std::fs::create_dir_all(toolchain_dir.join("bin"))
            .expect("Failed to create fake toolchain");

        // Create a fake lean binary
        let lean_bin = if cfg!(windows) {
            toolchain_dir.join("bin").join("lean.exe")
        } else {
            toolchain_dir.join("bin").join("lean")
        };

        // Create a simple shell script or executable that outputs a version
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::write(&lean_bin, "#!/bin/sh\necho 'Lean (version 4.24.0-test)'")
                .expect("Failed to write fake lean binary");
            let mut perms = std::fs::metadata(&lean_bin).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&lean_bin, perms).unwrap();
        }

        #[cfg(windows)]
        {
            std::fs::write(&lean_bin, "@echo off\necho Lean (version 4.24.0-test)")
                .expect("Failed to write fake lean binary");
        }
    }
}
