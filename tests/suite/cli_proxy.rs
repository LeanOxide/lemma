//! Tests for proxy mode (when invoked as lean, lake, etc.)

use super::test_helpers::{LemmaTestContext, TestSetup};

#[test]
fn test_which_command_shows_proxy_path() {
    let ctx = LemmaTestContext::new();

    // Create a fake toolchain
    ctx.create_fake_toolchain("leanprover/lean4:v4.24.0");

    // Set as default
    let _setup = ctx.run(&["default", "leanprover/lean4:v4.24.0"]);

    // Use which to find lean binary path
    let result = ctx.run(&["which", "lean"]);

    // Should show the path to lean in the toolchain
    result.assert_success();
    result.assert_stdout_contains("lean");
}

#[test]
fn test_which_without_toolchain_fails() {
    let ctx = LemmaTestContext::new();

    // Try to find lean without any toolchain configured
    let result = ctx.run(&["which", "lean"]);

    // Should fail because no toolchain is available
    result.assert_failed();
}

#[test]
fn test_which_with_environment_override() {
    let ctx = LemmaTestContext::new();

    // Create a fake toolchain
    ctx.create_fake_toolchain("leanprover/lean4:v4.24.0");

    // Use which with LEMMA_TOOLCHAIN environment variable
    let result = ctx.run_with_env(
        &["which", "lean"],
        &[("LEMMA_TOOLCHAIN", "leanprover/lean4:v4.24.0")],
    );

    result.assert_success();
    result.assert_stdout_contains("lean");
}

#[test]
fn test_run_command_with_toolchain() {
    let ctx = LemmaTestContext::new();

    // Create a fake toolchain
    ctx.create_fake_toolchain("leanprover/lean4:v4.24.0");

    // Use lemma run to execute with specific toolchain
    let result = ctx.run(&["run", "leanprover/lean4:v4.24.0", "lean", "--version"]);

    result.assert_success();
}

#[test]
fn test_run_command_without_toolchain_fails() {
    let ctx = LemmaTestContext::new();

    // Try to run with a non-existent toolchain
    let result = ctx.run(&["run", "nonexistent", "lean", "--version"]);

    result.assert_failed();
    result.assert_stderr_contains("not installed");
}
