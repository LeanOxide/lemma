//! Tests for directory override functionality

use super::test_helpers::{LemmaTestContext, TestSetup};
use std::fs;

#[test]
fn test_override_set_and_unset() {
    let ctx = LemmaTestContext::new();

    // Create a fake toolchain
    ctx.create_fake_toolchain("leanprover/lean4:v4.24.0");

    // Create a test directory
    let test_dir = ctx.temp_dir.path().join("test_project");
    fs::create_dir(&test_dir).unwrap();

    // Set an override
    let result = ctx.run(&[
        "override",
        "set",
        "leanprover/lean4:v4.24.0",
        "--path",
        test_dir.to_str().unwrap(),
    ]);

    result.assert_success();

    // Unset the override
    let result = ctx.run(&["override", "unset", "--path", test_dir.to_str().unwrap()]);

    result.assert_success();
}

#[test]
fn test_override_list() {
    let ctx = LemmaTestContext::new();

    // Create a fake toolchain
    ctx.create_fake_toolchain("leanprover/lean4:v4.24.0");

    // Create a test directory
    let test_dir = ctx.temp_dir.path().join("test_project");
    fs::create_dir(&test_dir).unwrap();

    // Set an override
    let _result = ctx.run(&[
        "override",
        "set",
        "leanprover/lean4:v4.24.0",
        "--path",
        test_dir.to_str().unwrap(),
    ]);

    // List overrides
    let result = ctx.run(&["override", "list"]);

    result.assert_success();
    result.assert_stdout_contains("leanprover/lean4:v4.24.0");
}

#[test]
fn test_override_nonexistent_toolchain_fails() {
    let ctx = LemmaTestContext::new();

    // Create a test directory
    let test_dir = ctx.temp_dir.path().join("test_project");
    fs::create_dir(&test_dir).unwrap();

    // Try to set override with non-existent toolchain
    let result = ctx.run(&[
        "override",
        "set",
        "nonexistent",
        "--path",
        test_dir.to_str().unwrap(),
    ]);

    result.assert_failed();
}

#[test]
fn test_override_unset_without_override() {
    let ctx = LemmaTestContext::new();

    // Create a test directory
    let test_dir = ctx.temp_dir.path().join("test_project");
    fs::create_dir(&test_dir).unwrap();

    // Try to unset when no override exists
    let result = ctx.run(&["override", "unset", "--path", test_dir.to_str().unwrap()]);

    // Should fail or indicate no override was present
    assert!(
        !result.success()
            || result.stdout.contains("No override")
            || result.stderr.contains("No override")
    );
}
