//! Configuration management for Lemma
//!
//! This crate handles all configuration-related functionality:
//! - Loading and merging configuration from multiple sources
//! - Settings resolution (CLI args + env vars + config files)
//! - Configuration file format and validation
//! - Toolchain resolution logic
//!
//! # Architecture Overview
//!
//! ## Three Key Types
//!
//! The configuration system uses three distinct types that represent different
//! stages of configuration processing:
//!
//! ### 1. [`settings::CliArgs`]
//!
//! Raw command-line arguments as parsed by Clap. This is a simplified representation
//! of global CLI flags, containing only fields needed for settings resolution.
//!
//! - Contains `Option<T>` for all values (may be unspecified)
//! - Maps 1:1 with CLI flags from `lemma-cli`
//! - Created by converting from `lemma_cli::GlobalArgs`
//! - Used only as input to settings resolution
//!
//! ### 2. [`Config`]
//!
//! Unified configuration loaded from files and environment variables. Contains both
//! **state** (managed by lemma) and **preferences** (user-configurable).
//!
//! - Loaded from multiple sources with proper precedence
//! - Contains `Option<T>` for preferences (may be unspecified)
//! - Sources (in order): system config → user config → project config → env vars
//! - File format: TOML (typically `~/.lemma/lemma.toml`)
//!
//! **State fields** (managed internally):
//! - `default_toolchain`: The default toolchain to use
//! - `overrides`: Directory-specific toolchain overrides
//! - `path_setup_shown`: Internal flags for user experience
//!
//! **Preference fields** (user-configurable):
//! - `global`: Verbosity, color settings
//! - `paths`: Custom lemma home directory
//! - `network`: Timeouts, proxies
//! - `lean_release`: Mirror URL for releases
//!
//! ### 3. [`GlobalSettings`]
//!
//! Resolved, final configuration used throughout lemma. This is what commands
//! actually use - all ambiguity has been resolved, all sources have been merged.
//!
//! - All fields are concrete types (no `Option<T>`)
//! - Created by merging CLI args, Config, and defaults
//! - Immutable and passed by reference to all commands
//! - Represents the single source of truth for current execution
//!
//! ## Resolution Flow
//!
//! ```text
//! ┌─────────────────┐
//! │  CLI Parsing    │  Clap parses command line
//! │  (lemma-cli)    │  → GlobalArgs
//! └────────┬────────┘
//!          │ Convert
//!          ↓
//! ┌─────────────────┐
//! │   CliArgs       │  Simplified representation
//! │  (Option<T>)    │  of CLI arguments
//! └────────┬────────┘
//!          │
//!          │ ┌──────────────────┐
//!          │ │  Config Loading  │  Load TOML files
//!          │ │  (this crate)    │  + environment vars
//!          │ └────────┬─────────┘
//!          │          │
//!          │          ↓
//!          │  ┌──────────────┐
//!          │  │   Config     │  Merged configuration
//!          │  │ (Option<T>)  │  from all file sources
//!          │  └──────┬───────┘
//!          │         │
//!          └─────────┼─────────┐
//!                    │ Merge   │
//!                    ↓         ↓
//!           ┌─────────────────────┐
//!           │  GlobalSettings     │  Final resolved settings
//!           │  (concrete values)  │  with all defaults applied
//!           └──────────┬──────────┘
//!                      │
//!                      ↓
//!              ┌───────────────┐
//!              │   Commands    │  All commands receive
//!              │  (execution)  │  &GlobalSettings
//!              └───────────────┘
//! ```
//!
//! ## Precedence Order
//!
//! When resolving each setting, sources are checked in this order (higher priority first):
//!
//! 1. **Command-line arguments** (highest priority)
//!    - Explicit user intent for this invocation
//!    - Example: `lemma --verbose install stable`
//!
//! 2. **Environment variables**
//!    - Prefix: `LEMMA_*`
//!    - Separator: `__` (double underscore for nesting)
//!    - Example: `LEMMA_GLOBAL__VERBOSE=2`
//!
//! 3. **Project configuration** (read-only)
//!    - `./lemma.toml` or `./.lemma/lemma.toml`
//!    - Only contains preferences, not state
//!    - Shared across project team
//!
//! 4. **User configuration** (read/write)
//!    - `~/.lemma/lemma.toml`
//!    - Contains both state and preferences
//!    - Modified by lemma commands (default, override)
//!
//! 5. **System configuration** (read-only, Unix only)
//!    - `/etc/lemma/lemma.toml`
//!    - Only contains preferences
//!    - System-wide defaults
//!
//! 6. **Built-in defaults** (lowest priority)
//!    - Hard-coded fallbacks
//!    - Ensure lemma always has valid settings
//!
//! ## Examples
//!
//! ### Basic usage from a command
//!
//! ```rust,no_run
//! use lemma_config::{CliArgs, GlobalSettings};
//!
//! fn my_command(settings: &GlobalSettings) -> anyhow::Result<()> {
//!     // All settings are already resolved - just use them!
//!     if settings.is_verbose() {
//!         println!("Doing something verbose...");
//!     }
//!
//!     let home = &settings.lemma_home;
//!     println!("Lemma home: {}", home.display());
//!
//!     Ok(())
//! }
//! ```
//!
//! ### Settings resolution (typically in main.rs)
//!
//! ```rust,no_run
//! use lemma_config::{CliArgs, GlobalSettings};
//! # struct GlobalArgs { verbose: u8, quiet: u8, color: Option<lemma_config::ColorChoice>, no_color: bool }
//! # let global_args = GlobalArgs { verbose: 0, quiet: 0, color: None, no_color: false };
//!
//! // Convert parsed CLI args to CliArgs
//! let cli_args = CliArgs {
//!     verbose: global_args.verbose,
//!     quiet: global_args.quiet,
//!     color: global_args.color,
//!     no_color: global_args.no_color,
//! };
//!
//! // Resolve to final settings (loads config files, merges everything)
//! let settings = GlobalSettings::resolve(&cli_args)?;
//!
//! // Now pass settings to commands
//! # Ok::<(), anyhow::Error>(())
//! ```
//!
//! ## Design Principles
//!
//! 1. **Clear separation of concerns**
//!    - Parsing (Clap) → Conversion (CliArgs) → Loading (Config) → Resolution (Settings)
//!    - Each type has a single, clear purpose
//!
//! 2. **Immutable after resolution**
//!    - Once `GlobalSettings` is created, it never changes
//!    - Commands receive `&GlobalSettings`, ensuring consistency
//!
//! 3. **Explicit precedence**
//!    - Resolution logic clearly shows which source wins
//!    - No hidden priority rules
//!
//! 4. **Fail fast**
//!    - Invalid configuration fails during loading/resolution
//!    - Commands can assume settings are valid
//!
//! 5. **Configuration = State + Preferences**
//!    - State: managed by lemma (default toolchain, overrides)
//!    - Preferences: managed by user (verbosity, colors, paths)
//!    - Both live in the same file but have different semantics

pub mod config;
pub mod resolution;
pub mod settings;

// Re-export commonly used types
pub use config::{ColorChoice, Config, GlobalConfig, NetworkConfig, PathsConfig};
pub use resolution::{
    find_tool_binary, resolve_toolchain, resolve_toolchain_or_fail, resolve_toolchain_with_source,
};
pub use settings::{CliArgs, GlobalSettings};
