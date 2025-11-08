//! Tests for configuration and settings

use super::test_helpers::{LemmaTestContext, TestSetup};

#[test]
fn test_settings_file_content() {
    let ctx = LemmaTestContext::new();

    // Trigger initialization
    ctx.create_fake_toolchain("leanprover/lean4:v4.24.0");
    let _result = ctx.run(&["default", "leanprover/lean4:v4.24.0"]);

    // Read the settings file
    let settings = ctx.read_file("settings.toml");

    // Verify it contains expected fields
    assert!(
        settings.contains("version"),
        "Settings should have version field"
    );
    assert!(
        settings.contains("path_setup_shown"),
        "Settings should have path_setup_shown field"
    );
}

#[test]
fn test_settings_persist_across_runs() {
    let ctx = LemmaTestContext::new();

    // First run
    ctx.create_fake_toolchain("leanprover/lean4:v4.24.0");
    let _result = ctx.run(&["default", "leanprover/lean4:v4.24.0"]);

    // Read settings
    let settings_before = ctx.read_file("settings.toml");
    assert!(settings_before.contains("path_setup_shown = true"));

    // Second run
    let _result2 = ctx.run(&["show"]);

    // Settings should be unchanged
    let settings_after = ctx.read_file("settings.toml");
    assert_eq!(settings_before, settings_after);
}

#[test]
fn test_error_messages_on_stderr() {
    let ctx = LemmaTestContext::new();

    // Try to use a non-existent toolchain
    let result = ctx.run(&["default", "nonexistent-toolchain"]);

    result.assert_failed();
    result.assert_stderr_contains("Toolchain");
    result.assert_stderr_contains("not installed");
}

#[test]
fn test_invalid_command_shows_help() {
    let ctx = LemmaTestContext::new();

    // Run an invalid command
    let result = ctx.run(&["invalid-command"]);

    result.assert_failed();
    // Error should be on stderr
    assert!(
        result.stderr.contains("unrecognized") || result.stderr.contains("Usage"),
        "Should show error or usage on stderr"
    );
}
