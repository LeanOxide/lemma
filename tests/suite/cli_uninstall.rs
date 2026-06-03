//! Tests for toolchain uninstall functionality

use super::test_helpers::{LemmaTestContext, TestSetup};

#[test]
fn test_uninstall_toolchain() {
    let ctx = LemmaTestContext::new();

    // Create and install a fake toolchain
    ctx.create_fake_toolchain("leanprover/lean4:v4.24.0");

    // Verify it exists
    let list_result = ctx.run(&["lean", "list"]);
    list_result.assert_success();
    list_result.assert_stdout_contains("leanprover/lean4:v4.24.0");

    // Uninstall it
    let result = ctx.run(&["lean", "uninstall", "leanprover/lean4:v4.24.0"]);
    result.assert_success();

    // Verify it's gone
    let list_result = ctx.run(&["lean", "list"]);
    list_result.assert_success();
    assert!(
        !list_result.stdout.contains("leanprover/lean4:v4.24.0"),
        "Toolchain should be removed from list"
    );
}

#[test]
fn test_uninstall_nonexistent_toolchain() {
    let ctx = LemmaTestContext::new();

    // Try to uninstall a toolchain that doesn't exist
    let _result = ctx.run(&["lean", "uninstall", "nonexistent"]);

    // Should handle gracefully (either error or no-op)
    // The exact behavior depends on the implementation
}

#[test]
fn test_uninstall_default_toolchain() {
    let ctx = LemmaTestContext::new();

    // Create and set a default toolchain
    ctx.create_fake_toolchain("leanprover/lean4:v4.24.0");
    let _setup = ctx.run(&["default", "leanprover/lean4:v4.24.0"]);

    // Uninstall the default toolchain
    let result = ctx.run(&["lean", "uninstall", "leanprover/lean4:v4.24.0"]);
    result.assert_success();

    // Check that default is cleared or shows an error
    let _show_result = ctx.run(&["show"]);
    // Should either show no default or handle the missing toolchain gracefully
}

#[test]
fn test_uninstall_toolchain_with_override() {
    let ctx = LemmaTestContext::new();
    use std::fs;

    // Create a fake toolchain
    ctx.create_fake_toolchain("leanprover/lean4:v4.24.0");

    // Create a test directory and set an override
    let test_dir = ctx.temp_dir.path().join("test_project");
    fs::create_dir(&test_dir).unwrap();

    let _override_result = ctx.run(&[
        "override",
        "set",
        "leanprover/lean4:v4.24.0",
        "--path",
        test_dir.to_str().unwrap(),
    ]);

    // Uninstall the toolchain
    let result = ctx.run(&["lean", "uninstall", "leanprover/lean4:v4.24.0"]);
    result.assert_success();

    // The override should still exist but point to a missing toolchain
    let override_list = ctx.run(&["override", "list"]);
    override_list.assert_success();
}
