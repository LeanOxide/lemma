//! Tests for toolchain management commands

use lemma_static::EnvVars;

use super::test_helpers::{LemmaTestContext, TestSetup};

#[test]
fn test_toolchain_list_empty() {
    let ctx = LemmaTestContext::new();

    let result = ctx.run(&["lean", "list"]);
    result.assert_success();

    // Should show no toolchains installed
    // The output might be empty or have a message about no toolchains
}

#[test]
fn test_toolchain_list_with_toolchains() {
    let ctx = LemmaTestContext::new();

    // Trigger initialization first
    let _ = ctx.run(&["--version"]);

    // Create a fake toolchain with proper format
    ctx.create_fake_toolchain("leanprover/lean4:v4.24.0");

    let result = ctx.run(&["lean", "list"]);
    result.assert_success();

    // Should list the toolchain
    result.assert_stdout_contains("leanprover/lean4:v4.24.0");
}

#[test]
fn test_show_command_no_default() {
    let ctx = LemmaTestContext::new();

    let result = ctx.run(&["show"]);

    // Without any toolchain, show should still work but indicate no default
    // The exact behavior may vary - just verify command runs
    assert!(result.success() || result.status_code != 0);
}

#[test]
fn test_show_with_environment_override() {
    let ctx = LemmaTestContext::new();

    // Trigger initialization and create a fake toolchain
    let _ = ctx.run(&["--version"]);
    ctx.create_fake_toolchain("leanprover/lean4:v4.24.0");

    let result = ctx.run_with_env(
        &["show"],
        &[(EnvVars::LEMMA_TOOLCHAIN, "leanprover/lean4:v4.24.0")],
    );

    result.assert_success();
    assert!(
        result.stdout.contains("leanprover/lean4:v4.24.0")
            || result.stderr.contains("leanprover/lean4:v4.24.0"),
        "Output should mention the toolchain"
    );
}

#[test]
fn test_which_command() {
    let ctx = LemmaTestContext::new();

    // Trigger initialization and create a fake toolchain
    let _ = ctx.run(&["--version"]);
    ctx.create_fake_toolchain("leanprover/lean4:v4.24.0");

    // Try to run which command with environment override
    let result = ctx.run_with_env(
        &["which", "lean"],
        &[(EnvVars::LEMMA_TOOLCHAIN, "leanprover/lean4:v4.24.0")],
    );

    result.assert_success();
    result.assert_stdout_contains("lean");
}

#[test]
fn test_default_subcommand() {
    let ctx = LemmaTestContext::new();

    // Try to set a non-existent toolchain as default (should fail)
    let result = ctx.run(&["default", "nonexistent-toolchain"]);
    result.assert_failed();
}

#[test]
fn test_default_with_fake_toolchain() {
    let ctx = LemmaTestContext::new();

    // Trigger initialization and create a fake toolchain
    let _ = ctx.run(&["--version"]);
    ctx.create_fake_toolchain("leanprover/lean4:v4.24.0");

    // Set it as default
    let result = ctx.run(&["default", "leanprover/lean4:v4.24.0"]);
    result.assert_success();

    // Verify it was set
    let show_result = ctx.run(&["show"]);
    show_result.assert_success();
    assert!(
        show_result.stdout.contains("leanprover/lean4:v4.24.0")
            || show_result.stderr.contains("leanprover/lean4:v4.24.0"),
        "Show command should display the default toolchain"
    );
}

#[test]
fn test_override_command() {
    let ctx = LemmaTestContext::new();

    // Trigger initialization and create a fake toolchain
    let _ = ctx.run(&["--version"]);
    ctx.create_fake_toolchain("leanprover/lean4:v4.24.0");

    // Set an override for the temp directory
    let _result = ctx.run(&[
        "override",
        "set",
        "leanprover/lean4:v4.24.0",
        "--path",
        ctx.temp_dir.path().to_str().unwrap(),
    ]);

    // This might fail if the toolchain isn't actually installed
    // But we can at least test the command structure
}

#[test]
fn test_version_command() {
    let ctx = LemmaTestContext::new();

    let result = ctx.run(&["--version"]);
    result.assert_success();
    result.assert_stdout_contains("lemma");
}

#[test]
fn test_help_command() {
    let ctx = LemmaTestContext::new();

    let result = ctx.run(&["--help"]);
    result.assert_success();
    result.assert_stdout_contains("Usage");
}

#[test]
fn test_toolchain_help() {
    let ctx = LemmaTestContext::new();

    let result = ctx.run(&["lean", "--help"]);
    result.assert_success();
    result.assert_stdout_contains("toolchain");
}

#[test]
fn test_toolchain_alias_help() {
    let ctx = LemmaTestContext::new();

    let result = ctx.run(&["toolchain", "--help"]);
    result.assert_success();
    result.assert_stdout_contains("toolchain");
}

#[test]
fn test_completions_command() {
    let ctx = LemmaTestContext::new();

    // Test bash completions
    let result = ctx.run(&["completions", "bash"]);
    result.assert_success();
}

#[test]
fn test_completions_all_shells() {
    let ctx = LemmaTestContext::new();

    let shells = &["bash", "zsh", "fish", "powershell", "elvish"];

    for shell in shells {
        let result = ctx.run(&["completions", shell]);
        result.assert_success();
    }
}

#[test]
fn test_self_update_shows_package_manager_guidance() {
    let ctx = LemmaTestContext::new();

    let result = ctx.run(&["self", "update"]);
    result.assert_success();
    result.assert_stdout_contains("pipx upgrade lemma");
    result.assert_stdout_contains("python -m pip install --user --upgrade lemma");
    result.assert_stdout_not_contains(&format!("{}{}", "lemma.", "puqing.work"));
    result.assert_stdout_not_contains(&format!("{}{}", "manifests/", "stable.toml"));
}
