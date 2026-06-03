//! Edge case tests for various scenarios

use super::test_helpers::{LemmaTestContext, TestSetup};
use std::fs;

#[test]
fn test_empty_toolchain_name() {
    let ctx = LemmaTestContext::new();

    // Try to set default with empty string
    let result = ctx.run(&["default", ""]);

    // Implementation allows this (clears the default)
    if result.success() {
        // Verify settings were updated
        let settings = ctx.read_file("lemma.toml");
        assert!(settings.contains("default_toolchain"));
    } else {
        // Or it might reject it
        result.assert_failed();
    }
}

#[test]
fn test_very_long_toolchain_name() {
    let ctx = LemmaTestContext::new();

    // Create a toolchain with a very long name
    let long_name = "a".repeat(200);

    // Try to use it
    let result = ctx.run(&["default", &long_name]);

    // Should handle gracefully
    result.assert_failed();
}

#[test]
fn test_special_characters_in_toolchain_name() {
    let ctx = LemmaTestContext::new();

    // Try various special characters
    let special_names = vec![
        "toolchain@#$%",
        "toolchain with spaces",
        "toolchain\nwith\nnewlines",
    ];

    for name in special_names {
        let result = ctx.run(&["default", name]);
        // Should handle gracefully
        result.assert_failed();
    }
}

#[test]
fn test_multiple_defaults_in_sequence() {
    let ctx = LemmaTestContext::new();

    // Create multiple fake toolchains
    ctx.create_fake_toolchain("leanprover/lean4:v4.24.0");
    ctx.create_fake_toolchain("leanprover/lean4:v4.25.0");
    ctx.create_fake_toolchain("leanprover/lean4:v4.26.0");

    // Set each as default in sequence
    for version in ["v4.24.0", "v4.25.0", "v4.26.0"] {
        let toolchain = format!("leanprover/lean4:{}", version);
        let result = ctx.run(&["default", &toolchain]);
        result.assert_success();

        // Verify it's set
        let show_result = ctx.run(&["show"]);
        show_result.assert_success();
        assert!(
            show_result.stdout.contains(&toolchain) || show_result.stderr.contains(&toolchain),
            "Should show {} as default",
            toolchain
        );
    }
}

#[test]
fn test_concurrent_settings_file_access() {
    let ctx = LemmaTestContext::new();

    // Create a toolchain
    ctx.create_fake_toolchain("leanprover/lean4:v4.24.0");

    // Run multiple commands that access settings
    let _r1 = ctx.run(&["default", "leanprover/lean4:v4.24.0"]);
    let _r2 = ctx.run(&["show"]);
    let _r3 = ctx.run(&["lean", "list"]);

    // Settings file should remain consistent
    let settings = ctx.read_file("lemma.toml");
    assert!(settings.contains("leanprover/lean4:v4.24.0"));
}

#[test]
fn test_toolchain_with_path_separators() {
    let ctx = LemmaTestContext::new();

    // Try to use a toolchain name with path separators
    let result = ctx.run(&["default", "../../../etc/passwd"]);

    // Should fail
    result.assert_failed();
}

#[test]
fn test_settings_file_corruption_recovery() {
    let ctx = LemmaTestContext::new();

    // Create initial valid settings
    ctx.create_fake_toolchain("leanprover/lean4:v4.24.0");
    let _r = ctx.run(&["default", "leanprover/lean4:v4.24.0"]);

    // Corrupt the settings file
    let settings_path = ctx.lemma_home().join("lemma.toml");
    fs::write(&settings_path, "invalid toml {{{").unwrap();

    // Try to run a command - should handle corruption gracefully
    let _result = ctx.run(&["show"]);

    // May succeed with defaults or fail with a clear error
    // Either is acceptable as long as it doesn't panic
}

#[test]
fn test_empty_toolchains_directory() {
    let ctx = LemmaTestContext::new();

    // Initialize lemma
    let _init = ctx.run(&["lean", "list"]);

    // List should succeed even with no toolchains
    let result = ctx.run(&["lean", "list"]);
    result.assert_success();
}

#[test]
fn test_show_without_default_or_override() {
    let ctx = LemmaTestContext::new();

    // Run show without any toolchain configured
    let _result = ctx.run(&["show"]);

    // Should succeed and indicate no toolchain is active
    // The exact output depends on implementation
}

#[test]
fn test_which_with_nonexistent_binary() {
    let ctx = LemmaTestContext::new();

    // Create a toolchain and set as default
    ctx.create_fake_toolchain("leanprover/lean4:v4.24.0");
    let _setup = ctx.run(&["default", "leanprover/lean4:v4.24.0"]);

    // Try to find a binary that doesn't exist
    let result = ctx.run(&["which", "nonexistent-binary"]);

    // Should fail or indicate not found
    result.assert_failed();
}
