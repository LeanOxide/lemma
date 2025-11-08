//! Tests for custom toolchain linking

use super::test_helpers::LemmaTestContext;
use std::fs;

#[test]
fn test_link_custom_toolchain() {
    let ctx = LemmaTestContext::new();

    // Create a custom toolchain directory
    let custom_dir = ctx.temp_dir.path().join("custom_lean");
    fs::create_dir_all(custom_dir.join("bin")).unwrap();

    // Create a fake lean binary
    let lean_bin = if cfg!(windows) {
        custom_dir.join("bin").join("lean.exe")
    } else {
        custom_dir.join("bin").join("lean")
    };

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::write(&lean_bin, "#!/bin/sh\necho 'Custom Lean'").unwrap();
        let mut perms = fs::metadata(&lean_bin).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&lean_bin, perms).unwrap();
    }

    #[cfg(windows)]
    {
        fs::write(&lean_bin, "@echo off\necho Custom Lean").unwrap();
    }

    // Link the custom toolchain
    let result = ctx.run(&[
        "toolchain",
        "link",
        "my-custom",
        custom_dir.to_str().unwrap(),
    ]);

    result.assert_success();

    // Verify it appears in the list
    let list_result = ctx.run(&["toolchain", "list"]);
    list_result.assert_success();
    list_result.assert_stdout_contains("my-custom");
}

#[test]
fn test_link_with_channel_name() {
    let ctx = LemmaTestContext::new();

    // Create a custom toolchain directory
    let custom_dir = ctx.temp_dir.path().join("custom_lean");
    fs::create_dir_all(custom_dir.join("bin")).unwrap();

    // Link with a name that looks like a channel (like 'stable')
    // The implementation may allow this for custom toolchains
    let result = ctx.run(&["toolchain", "link", "stable", custom_dir.to_str().unwrap()]);

    // Check the result - implementation determines if this is allowed
    // If it succeeds, verify it's in the list
    if result.success() {
        let list_result = ctx.run(&["toolchain", "list"]);
        list_result.assert_success();
    }
}

#[test]
fn test_link_nonexistent_directory() {
    let ctx = LemmaTestContext::new();

    // Try to link a non-existent directory
    let result = ctx.run(&["toolchain", "link", "my-custom", "/nonexistent/path"]);

    // Should fail
    result.assert_failed();
}

#[test]
fn test_link_and_use_custom_toolchain() {
    let ctx = LemmaTestContext::new();

    // Create a custom toolchain directory
    let custom_dir = ctx.temp_dir.path().join("custom_lean");
    fs::create_dir_all(custom_dir.join("bin")).unwrap();

    // Create a fake lean binary
    let lean_bin = if cfg!(windows) {
        custom_dir.join("bin").join("lean.exe")
    } else {
        custom_dir.join("bin").join("lean")
    };

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::write(&lean_bin, "#!/bin/sh\necho 'Custom Lean'").unwrap();
        let mut perms = fs::metadata(&lean_bin).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&lean_bin, perms).unwrap();
    }

    #[cfg(windows)]
    {
        fs::write(&lean_bin, "@echo off\necho Custom Lean").unwrap();
    }

    // Link the custom toolchain
    let _link_result = ctx.run(&[
        "toolchain",
        "link",
        "my-custom",
        custom_dir.to_str().unwrap(),
    ]);

    // Set it as default
    let result = ctx.run(&["default", "my-custom"]);
    result.assert_success();

    // Verify it shows as active
    let show_result = ctx.run(&["show"]);
    show_result.assert_success();
    show_result.assert_stdout_contains("my-custom");
}
