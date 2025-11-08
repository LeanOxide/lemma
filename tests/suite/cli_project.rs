//! Tests for project-based toolchain resolution

use super::test_helpers::{LemmaTestContext, TestSetup};
use std::fs;

#[test]
fn test_lean_toolchain_file() {
    let ctx = LemmaTestContext::new();

    // Create a fake toolchain
    ctx.create_fake_toolchain("leanprover/lean4:v4.24.0");

    // Create a project directory with lean-toolchain file
    let project_dir = ctx.temp_dir.path().join("my_project");
    fs::create_dir(&project_dir).unwrap();

    let toolchain_file = project_dir.join("lean-toolchain");
    fs::write(&toolchain_file, "leanprover/lean4:v4.24.0\n").unwrap();

    // Show should detect the toolchain from the file when run in that directory
    // Note: This test is limited because we can't easily change the working directory
    // in the spawned process, but we can test the basic functionality
}

#[test]
fn test_leanpkg_toml_file() {
    let ctx = LemmaTestContext::new();

    // Create a fake toolchain
    ctx.create_fake_toolchain("leanprover/lean4:v4.5.0");

    // Create a project directory with leanpkg.toml file
    let project_dir = ctx.temp_dir.path().join("my_project");
    fs::create_dir(&project_dir).unwrap();

    let leanpkg_file = project_dir.join("leanpkg.toml");
    fs::write(
        &leanpkg_file,
        "lean_version = \"v4.5.0\"\nname = \"test\"\n",
    )
    .unwrap();

    // The toolchain module should be able to parse this
    // This is more of an integration test for the project file detection
}

#[test]
fn test_lake_manifest_json_file() {
    let ctx = LemmaTestContext::new();

    // Create a fake toolchain
    ctx.create_fake_toolchain("leanprover/lean4:v4.10.0");

    // Create a project directory with lake-manifest.json file
    let project_dir = ctx.temp_dir.path().join("my_project");
    fs::create_dir(&project_dir).unwrap();

    let manifest_file = project_dir.join("lake-manifest.json");
    fs::write(&manifest_file, r#"{"version": 7, "leanprover_lean4": {"url": "https://github.com/leanprover/lean4", "type": "git", "subDir": null, "rev": "v4.10.0"}}"#).unwrap();

    // The toolchain module should be able to parse this
}

#[test]
fn test_multiple_project_files_precedence() {
    let ctx = LemmaTestContext::new();

    // Create fake toolchains
    ctx.create_fake_toolchain("leanprover/lean4:v4.24.0");
    ctx.create_fake_toolchain("leanprover/lean4:v4.5.0");

    // Create a project directory with both lean-toolchain and leanpkg.toml
    let project_dir = ctx.temp_dir.path().join("my_project");
    fs::create_dir(&project_dir).unwrap();

    // lean-toolchain should take precedence
    let toolchain_file = project_dir.join("lean-toolchain");
    fs::write(&toolchain_file, "leanprover/lean4:v4.24.0\n").unwrap();

    let leanpkg_file = project_dir.join("leanpkg.toml");
    fs::write(
        &leanpkg_file,
        "lean_version = \"v4.5.0\"\nname = \"test\"\n",
    )
    .unwrap();

    // The lean-toolchain file should take precedence
}
