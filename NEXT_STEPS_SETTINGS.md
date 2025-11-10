# Next Step: Implementing the Settings Pattern

## Why Settings?

Currently, your `main.rs` accesses CLI args directly:

```rust
// Current approach - accessing CLI directly
if cli.top_level.global_args.verbose > 0 {
    std::env::set_var("RUST_LOG", "debug");
}
```

**Problems with this:**
- Business logic mixed with CLI parsing
- Hard to add config file support later
- Difficult to test (need to construct CLI structs)
- Can't compute derived values
- Unclear precedence (CLI vs env vs config)

**Settings pattern solves this:**
```rust
// Settings approach - clean separation
let settings = GlobalSettings::resolve(&cli.top_level.global_args);
setup_logging(&settings);
commands::handle_command(cli.command, settings)
```

## Implementation Guide

### Step 1: Create `src/settings.rs`

```rust
//! Settings resolution for Lemma
//!
//! This module converts CLI arguments into resolved settings by merging:
//! 1. Command-line arguments (highest priority)
//! 2. Environment variables
//! 3. Configuration files (future)
//! 4. Built-in defaults (lowest priority)

use std::path::PathBuf;
use anyhow::{Context, Result};
use crate::cli::{GlobalArgs, ColorChoice};

/// Resolved global settings used throughout the application.
///
/// Unlike `GlobalArgs`, which represents raw CLI input, `GlobalSettings`
/// represents the final, resolved configuration after merging all sources.
#[derive(Debug, Clone)]
pub struct GlobalSettings {
    /// Verbosity level (0 = normal, 1 = debug, 2+ = trace)
    pub verbose: u8,

    /// Quiet level (0 = normal, 1 = warnings only, 2+ = errors only)
    pub quiet: u8,

    /// Color output configuration
    pub color: ColorChoice,

    /// Lemma home directory (where toolchains are stored)
    pub lemma_home: PathBuf,
}

impl GlobalSettings {
    /// Resolve global settings from CLI arguments.
    ///
    /// Priority order:
    /// 1. CLI flags (highest priority)
    /// 2. Environment variables
    /// 3. Config file (future)
    /// 4. Defaults (lowest priority)
    pub fn resolve(args: &GlobalArgs) -> Result<Self> {
        // Resolve color setting
        let color = resolve_color_choice(args)?;

        // Resolve lemma home directory
        let lemma_home = resolve_lemma_home()?;

        Ok(Self {
            verbose: args.verbose,
            quiet: args.quiet,
            color,
            lemma_home,
        })
    }

    /// Get the appropriate log level for tracing.
    ///
    /// Returns a string like "info", "debug", "trace" based on verbosity.
    pub fn log_level(&self) -> &'static str {
        // Quiet takes precedence over verbose
        match (self.verbose, self.quiet) {
            // Normal verbosity
            (0, 0) => "info",
            // Verbose levels
            (1, 0) => "debug",
            (2.., 0) => "trace",
            // Quiet levels
            (0, 1) => "warn",
            (0, 2..) => "error",
            // If both are set (shouldn't happen due to conflicts_with), default to info
            _ => "info",
        }
    }

    /// Check if we should suppress progress bars and interactive output.
    pub fn is_quiet(&self) -> bool {
        self.quiet > 0
    }

    /// Check if we're in verbose mode.
    pub fn is_verbose(&self) -> bool {
        self.verbose > 0
    }

    /// Get verbosity level for detailed operations.
    pub fn verbosity_level(&self) -> u8 {
        self.verbose
    }
}

/// Resolve the color choice from CLI args and environment.
fn resolve_color_choice(args: &GlobalArgs) -> Result<ColorChoice> {
    // Priority:
    // 1. --color flag (if provided)
    // 2. --no-color flag (if provided)
    // 3. Default (Auto)

    if let Some(color) = args.color {
        return Ok(color);
    }

    if args.no_color {
        return Ok(ColorChoice::Never);
    }

    // Default: auto-detect terminal support
    Ok(ColorChoice::Auto)
}

/// Resolve the Lemma home directory.
fn resolve_lemma_home() -> Result<PathBuf> {
    // Priority:
    // 1. LEMMA_HOME environment variable
    // 2. Default: ~/.lemma

    if let Ok(home) = std::env::var("LEMMA_HOME") {
        let path = PathBuf::from(home);
        if !path.is_absolute() {
            anyhow::bail!(
                "LEMMA_HOME must be an absolute path, got: {}",
                path.display()
            );
        }
        return Ok(path);
    }

    // Default location
    let home = dirs::home_dir()
        .context("Could not determine home directory")?;

    Ok(home.join(".lemma"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level() {
        let settings = GlobalSettings {
            verbose: 0,
            quiet: 0,
            color: ColorChoice::Auto,
            lemma_home: PathBuf::from("/tmp/lemma"),
        };
        assert_eq!(settings.log_level(), "info");

        let settings = GlobalSettings {
            verbose: 1,
            quiet: 0,
            color: ColorChoice::Auto,
            lemma_home: PathBuf::from("/tmp/lemma"),
        };
        assert_eq!(settings.log_level(), "debug");

        let settings = GlobalSettings {
            verbose: 0,
            quiet: 1,
            color: ColorChoice::Auto,
            lemma_home: PathBuf::from("/tmp/lemma"),
        };
        assert_eq!(settings.log_level(), "warn");
    }

    #[test]
    fn test_is_quiet() {
        let settings = GlobalSettings {
            verbose: 0,
            quiet: 0,
            color: ColorChoice::Auto,
            lemma_home: PathBuf::from("/tmp/lemma"),
        };
        assert!(!settings.is_quiet());

        let settings = GlobalSettings {
            verbose: 0,
            quiet: 1,
            color: ColorChoice::Auto,
            lemma_home: PathBuf::from("/tmp/lemma"),
        };
        assert!(settings.is_quiet());
    }

    #[test]
    fn test_resolve_color_choice() {
        // Test --color flag
        let args = GlobalArgs {
            verbose: 0,
            quiet: 0,
            color: Some(ColorChoice::Always),
            no_color: false,
        };
        assert!(matches!(
            resolve_color_choice(&args).unwrap(),
            ColorChoice::Always
        ));

        // Test --no-color flag
        let args = GlobalArgs {
            verbose: 0,
            quiet: 0,
            color: None,
            no_color: true,
        };
        assert!(matches!(
            resolve_color_choice(&args).unwrap(),
            ColorChoice::Never
        ));

        // Test default
        let args = GlobalArgs {
            verbose: 0,
            quiet: 0,
            color: None,
            no_color: false,
        };
        assert!(matches!(
            resolve_color_choice(&args).unwrap(),
            ColorChoice::Auto
        ));
    }
}
```

### Step 2: Register the Module in `main.rs`

```rust
// Add this near the top with other module declarations
mod settings;

use settings::GlobalSettings;
```

### Step 3: Update `main.rs` to Use Settings

**Before:**
```rust
fn run() -> Result<()> {
    // ...existing proxy mode detection...

    let cli = Cli::parse();

    // Setup logging based on verbosity
    if cli.top_level.global_args.verbose > 0 {
        std::env::set_var("RUST_LOG", "debug");
    }
    tracing_subscriber::fmt::init();

    // Dispatch to appropriate command handler
    commands::handle_command(cli.command)
}
```

**After:**
```rust
fn run() -> Result<()> {
    // ...existing proxy mode detection...

    let cli = Cli::parse();

    // Resolve settings from CLI args + environment + config
    let settings = GlobalSettings::resolve(&cli.top_level.global_args)?;

    // Setup logging using resolved settings
    setup_logging(&settings);

    // Dispatch to appropriate command handler
    commands::handle_command(cli.command, settings)
}

/// Setup logging based on resolved settings
fn setup_logging(settings: &GlobalSettings) {
    use tracing_subscriber::EnvFilter;

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(settings.log_level()));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_ansi(matches!(settings.color, ColorChoice::Always)
            || (matches!(settings.color, ColorChoice::Auto) && atty::is(atty::Stream::Stderr)))
        .init();
}
```

### Step 4: Update Command Handlers

Update `commands.rs` to accept settings:

**Before:**
```rust
pub fn handle_command(command: Commands) -> Result<()> {
    match command {
        Commands::Lean { command } => match command {
            ToolchainCommands::Install { toolchain } => {
                install::execute(&toolchain)
            }
            // ...
        },
        // ...
    }
}
```

**After:**
```rust
pub fn handle_command(command: Commands, settings: GlobalSettings) -> Result<()> {
    match command {
        Commands::Lean { command } => match command {
            ToolchainCommands::Install { toolchain } => {
                install::execute(&toolchain, &settings)
            }
            // ...
        },
        // ...
    }
}
```

Then update individual command functions to use settings:

```rust
// Before
pub fn execute(toolchain: &str) -> Result<()> {
    // Hard-coded paths
    let toolchains_dir = dirs::home_dir()
        .unwrap()
        .join(".lemma")
        .join("toolchains");
    // ...
}

// After
pub fn execute(toolchain: &str, settings: &GlobalSettings) -> Result<()> {
    // Use settings
    let toolchains_dir = settings.lemma_home.join("toolchains");

    if settings.is_verbose() {
        println!("Installing {} to {}", toolchain, toolchains_dir.display());
    }
    // ...
}
```

### Step 5: Add Optional Dependency

If you want colored output support, add `atty` to check if we're in a TTY:

```toml
# In Cargo.toml [workspace.dependencies]
atty = "0.2"
```

```toml
# In crates/lemma-rs/Cargo.toml [dependencies]
atty.workspace = true
```

## Benefits You'll See Immediately

### 1. Easier Testing
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_install_command() {
        let settings = GlobalSettings {
            verbose: 0,
            quiet: 0,
            color: ColorChoice::Never,
            lemma_home: PathBuf::from("/tmp/test-lemma"),
        };

        // Test with custom settings
        install::execute("lean-4.0.0", &settings).unwrap();
    }
}
```

### 2. Consistent Paths
```rust
// Before: Hard-coded everywhere
dirs::home_dir().unwrap().join(".lemma")

// After: One place to change
settings.lemma_home
```

### 3. Better Progress Reporting
```rust
fn download_with_progress(url: &str, settings: &GlobalSettings) -> Result<()> {
    if settings.is_quiet() {
        // Silent download
        download_silent(url)
    } else {
        // Show progress bar
        download_with_progress_bar(url)
    }
}
```

### 4. Future-Proof for Config Files

When you add config file support later, you only change one place:

```rust
impl GlobalSettings {
    pub fn resolve(args: &GlobalArgs) -> Result<Self> {
        // Load config file
        let config = Config::load()?; // NEW

        // Merge: CLI > Env > Config > Defaults
        let color = args.color
            .or(config.color)  // NEW: Config file fallback
            .unwrap_or(ColorChoice::Auto);

        // ... rest stays the same
    }
}
```

## Gradual Migration Strategy

You don't need to update everything at once:

1. **Day 1**: Create `settings.rs`, update `main.rs`
2. **Day 2**: Update one command module (e.g., `install.rs`)
3. **Day 3**: Update another command module
4. **Continue**: Gradually migrate all commands

Commands can coexist:
```rust
pub fn handle_command(command: Commands, settings: GlobalSettings) -> Result<()> {
    match command {
        // New style: uses settings
        Commands::Install { toolchain } => {
            install::execute(&toolchain, &settings)
        }

        // Old style: doesn't use settings (yet)
        Commands::Update => {
            update::execute()  // No settings parameter
        }
    }
}
```

## Common Patterns

### Using Settings in Commands

```rust
pub fn execute(toolchain: &str, settings: &GlobalSettings) -> Result<()> {
    // Use lemma home
    let toolchains_dir = settings.lemma_home.join("toolchains");

    // Conditional logging
    if settings.is_verbose() {
        println!("Downloading {}", toolchain);
    }

    // Show progress based on quiet mode
    let progress = if settings.is_quiet() {
        None
    } else {
        Some(create_progress_bar())
    };

    // ... rest of implementation
}
```

### Passing Settings Through Layers

```rust
// Top-level command handler
pub fn execute(args: InstallArgs, settings: &GlobalSettings) -> Result<()> {
    let toolchain = resolve_toolchain(&args.toolchain)?;
    download_and_install(&toolchain, settings)
}

// Helper function
fn download_and_install(toolchain: &Toolchain, settings: &GlobalSettings) -> Result<()> {
    let archive = download_toolchain(toolchain, settings)?;
    extract_toolchain(&archive, settings)?;
    verify_installation(toolchain, settings)
}
```

## Ready to Implement?

1. Create `src/settings.rs` with the code above
2. Update `src/main.rs` to use `GlobalSettings::resolve()`
3. Update one command to use settings (start with a simple one)
4. Test it works: `cargo build && ./target/debug/lemma -v show`
5. Gradually migrate other commands

Once you have the Settings layer, you'll be ready for Phase 3: Configuration Files!
