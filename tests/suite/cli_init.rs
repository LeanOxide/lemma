//! Tests for auto-initialization and basic setup

use super::test_helpers::{LemmaTestContext, TestSetup};

#[test]
fn test_auto_init_creates_directories() {
    let ctx = LemmaTestContext::new();

    // Creating a fake toolchain and setting it as default triggers auto-initialization
    ctx.create_fake_toolchain("leanprover/lean4:v4.24.0");
    let _result = ctx.run(&["default", "leanprover/lean4:v4.24.0"]);

    // Check that directories were created
    assert!(ctx.bin_dir().exists(), "bin directory should exist");
    assert!(
        ctx.toolchains_dir().exists(),
        "toolchains directory should exist"
    );
    assert!(
        ctx.lemma_home().join("tmp").exists(),
        "tmp directory should exist"
    );
    assert!(
        ctx.lemma_home().join("update-hashes").exists(),
        "update-hashes directory should exist"
    );
}

#[test]
fn test_auto_init_creates_settings_file() {
    let ctx = LemmaTestContext::new();

    // Creating a fake toolchain and setting it as default triggers auto-initialization
    ctx.create_fake_toolchain("leanprover/lean4:v4.24.0");
    let _result = ctx.run(&["default", "leanprover/lean4:v4.24.0"]);

    // Check that settings file was created
    assert!(
        ctx.exists("settings.toml"),
        "settings.toml should be created"
    );
}

#[test]
fn test_auto_init_creates_proxy_binaries() {
    let ctx = LemmaTestContext::new();

    // Creating a fake toolchain and setting it as default triggers auto-initialization
    ctx.create_fake_toolchain("leanprover/lean4:v4.24.0");
    let _result = ctx.run(&["default", "leanprover/lean4:v4.24.0"]);

    // Check that proxy binaries were created
    let proxy_tools = &[
        "lean",
        "lake",
        "leanpkg",
        "leanchecker",
        "leanc",
        "leanmake",
    ];

    for tool in proxy_tools {
        let tool_path = if cfg!(windows) {
            ctx.bin_dir().join(format!("{}.exe", tool))
        } else {
            ctx.bin_dir().join(tool)
        };
        assert!(
            tool_path.exists() || tool_path.symlink_metadata().is_ok(),
            "{} proxy should exist",
            tool
        );
    }
}

#[test]
fn test_first_run_shows_path_setup_message() {
    let ctx = LemmaTestContext::new();

    // Creating a fake toolchain and setting it as default triggers auto-initialization
    ctx.create_fake_toolchain("leanprover/lean4:v4.24.0");
    let result = ctx.run(&["default", "leanprover/lean4:v4.24.0"]);

    // Check that PATH setup message was shown
    #[cfg(windows)]
    {
        assert!(
            result.stdout.contains("PowerShell") || result.stderr.contains("PowerShell"),
            "Should show PowerShell setup instructions"
        );
    }

    #[cfg(not(windows))]
    {
        assert!(
            result.stdout.contains("export PATH") || result.stderr.contains("export PATH"),
            "Should show export PATH instructions"
        );
    }
}

#[test]
fn test_path_message_shown_only_once() {
    let ctx = LemmaTestContext::new();

    // Creating a fake toolchain and setting it as default triggers auto-initialization
    ctx.create_fake_toolchain("leanprover/lean4:v4.24.0");
    let result1 = ctx.run(&["default", "leanprover/lean4:v4.24.0"]);

    // Verify PATH message was shown
    #[cfg(windows)]
    {
        let has_message =
            result1.stdout.contains("PowerShell") || result1.stderr.contains("PowerShell");
        assert!(has_message, "First run should show PATH message");
    }
    #[cfg(not(windows))]
    {
        let has_message =
            result1.stdout.contains("export PATH") || result1.stderr.contains("export PATH");
        assert!(has_message, "First run should show PATH message");
    }

    // Run command second time
    let result2 = ctx.run(&["default", "leanprover/lean4:v4.24.0"]);

    // PATH message should not be shown again
    #[cfg(windows)]
    {
        result2.assert_stdout_not_contains("PowerShell");
        result2.assert_stderr_not_contains("PowerShell");
    }
    #[cfg(not(windows))]
    {
        result2.assert_stdout_not_contains("export PATH");
        result2.assert_stderr_not_contains("export PATH");
    }
}

#[test]
fn test_multiple_runs_preserve_structure() {
    let ctx = LemmaTestContext::new();

    // Trigger initialization first
    ctx.create_fake_toolchain("leanprover/lean4:v4.24.0");
    let _result1 = ctx.run(&["default", "leanprover/lean4:v4.24.0"]);

    // Run multiple commands
    let _result2 = ctx.run(&["--version"]);
    let _result3 = ctx.run(&["--help"]);

    // Verify structure still correct
    assert!(ctx.bin_dir().exists());
    assert!(ctx.toolchains_dir().exists());
    assert!(ctx.exists("settings.toml"));
}
