# UV CLI Architecture Guide for Lemma

## Overview of UV's Multi-Crate CLI Pattern

UV uses a sophisticated multi-crate architecture to separate concerns and improve maintainability. Here's how it's structured:

```
uv/crates/
├── uv-cli/          # Pure CLI definitions (clap structures)
├── uv-static/       # Static data (environment variables, constants)
├── uv/              # Main binary and command implementations
│   ├── settings.rs  # Converts CLI args → Settings structs
│   ├── commands/    # Command implementations
│   └── lib.rs       # Main entry point and orchestration
└── uv-*             # Other domain-specific crates
```

## Key Patterns from UV

### 1. **Separate CLI Definition Crate (`uv-cli`)**

The `uv-cli` crate contains ONLY the CLI structure - no implementation logic:

```rust
// crates/uv-cli/src/lib.rs
#[derive(Parser)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Box<Commands>,

    #[command(flatten)]
    pub top_level: TopLevelArgs,
}
```

**Benefits:**
- Fast compilation (changes to logic don't require recompiling CLI)
- Clean separation of concerns
- CLI can be used by multiple binaries
- Easy to generate shell completions

### 2. **Three-Layer Argument Structure**

```rust
Cli
├── command: Commands         // Subcommands (install, update, etc.)
└── top_level: TopLevelArgs   // Global flags
    ├── cache_args: CacheArgs     // Cache-related flags
    ├── global_args: GlobalArgs   // Universal flags (verbose, color, etc.)
    ├── config_file: Option<PathBuf>
    ├── no_config: bool
    ├── help: Option<bool>
    └── version: Option<bool>
```

**Why this structure?**
- **TopLevelArgs**: Groups all flags that can appear before the subcommand
- **GlobalArgs**: Flags that apply to ALL commands (using `global = true`)
- **CacheArgs**: Domain-specific but global flags
- Uses `Box<>` to reduce stack size for large structures

### 3. **GlobalArgs Pattern**

```rust
#[derive(Parser, Debug, Clone)]
#[command(next_help_heading = "Global options", next_display_order = 1000)]
pub struct GlobalArgs {
    /// Use verbose output.
    #[arg(global = true, action = clap::ArgAction::Count, long, short,
          conflicts_with = "quiet")]
    pub verbose: u8,

    /// Use quiet output.
    #[arg(global = true, action = clap::ArgAction::Count, long, short,
          conflicts_with = "verbose")]
    pub quiet: u8,

    /// Control the use of color in output.
    #[arg(global = true, long, value_enum, conflicts_with = "no_color")]
    pub color: Option<ColorChoice>,

    /// Disable colors (for pip compatibility).
    #[arg(global = true, long, hide = true, conflicts_with = "color")]
    pub no_color: bool,

    // ... more global flags
}

#[derive(Debug, Copy, Clone, clap::ValueEnum)]
pub enum ColorChoice {
    Auto,    // Auto-detect terminal support
    Always,  // Force colors
    Never,   // Disable colors
}
```

**Key features:**
- `action = Count` allows `-v`, `-vv`, `-vvv` for different verbosity levels
- `conflicts_with` ensures mutually exclusive flags
- `hide = true` for deprecated flags
- All flags marked `global = true`

### 4. **Static Environment Variables Crate (`uv-static`)**

```rust
// crates/uv-static/src/env_vars.rs
pub struct EnvVars;

impl EnvVars {
    pub const UV_OFFLINE: &'static str = "UV_OFFLINE";
    pub const UV_NO_CONFIG: &'static str = "UV_NO_CONFIG";
    pub const UV_CACHE_DIR: &'static str = "UV_CACHE_DIR";
    // ... all environment variables
}
```

Used in CLI definitions:

```rust
#[arg(global = true, long, env = EnvVars::UV_NO_CONFIG)]
pub no_config: bool,
```

**Benefits:**
- Single source of truth for env var names
- Easy to document
- Refactoring-safe (compiler catches changes)

### 5. **Settings Pattern (CLI → Settings)**

UV separates CLI args (what user types) from Settings (what code uses):

```rust
// crates/uv/src/settings.rs

/// The resolved global settings to use for any invocation of the CLI.
#[derive(Debug, Clone)]
pub(crate) struct GlobalSettings {
    pub(crate) quiet: u8,
    pub(crate) verbose: u8,
    pub(crate) color: ColorChoice,
    pub(crate) network_settings: NetworkSettings,
    pub(crate) python_preference: PythonPreference,
    // ... more settings
}

impl GlobalSettings {
    /// Resolve from CLI args + config files + environment
    pub(crate) fn resolve(
        args: &GlobalArgs,
        workspace: Option<&FilesystemOptions>,
        environment: &EnvironmentOptions,
    ) -> Self {
        Self {
            quiet: args.quiet,
            verbose: args.verbose,
            color: if let Some(color) = args.color {
                color
            } else if args.no_color {
                ColorChoice::Never
            } else {
                ColorChoice::Auto
            },
            // ... resolve other settings with priority:
            // 1. CLI args (highest priority)
            // 2. Config file
            // 3. Environment variables
            // 4. Defaults (lowest priority)
        }
    }
}
```

**Why separate Settings from CLI?**
- CLI has redundant options (e.g., `--color` vs `--no-color`)
- Settings merges CLI + config + env + defaults
- Settings has computed/derived values
- Command implementations only work with Settings, never raw CLI args

### 6. **Main Entry Point Pattern**

```rust
// crates/uv/src/lib.rs

#[instrument(skip_all)]
async fn run(mut cli: Cli) -> Result<ExitStatus> {
    // 1. Enable warnings based on quiet flag
    if cli.top_level.global_args.quiet == 0 {
        uv_warnings::enable();
    }

    // 2. Load environment variables not handled by Clap
    let environment = EnvironmentOptions::new()?;

    // 3. Load configuration from filesystem
    let filesystem = if let Some(config_file) = cli.top_level.config_file.as_ref() {
        Some(FilesystemOptions::from_file(config_file)?)
    } else if cli.top_level.no_config {
        None
    } else {
        // Discover config files in project/user/system locations
        FilesystemOptions::discover(&project_dir)?
    };

    // 4. Resolve final settings (CLI + config + env + defaults)
    let settings = GlobalSettings::resolve(
        &cli.top_level.global_args,
        filesystem.as_ref(),
        &environment,
    );

    // 5. Setup logging based on verbosity
    setup_logging(settings.verbose, settings.color);

    // 6. Dispatch to command handlers
    match cli.command {
        Commands::Install(args) => {
            commands::install::execute(args, settings).await
        }
        // ... other commands
    }
}
```

## Applying to Lemma

Here's how to restructure lemma using these patterns:

### Current Structure
```
lemma/
└── crates/
    ├── lemma-rs/       # Everything mixed together
    └── lemma-static/   # Empty
```

### Recommended Structure
```
lemma/
└── crates/
    ├── lemma-cli/      # NEW: Pure CLI definitions
    ├── lemma-static/   # Constants & env vars
    ├── lemma-core/     # NEW: Core types, errors, etc.
    ├── lemma-config/   # NEW: Config file handling
    └── lemma-rs/       # Main binary & commands
```

### Implementation Steps

#### Step 1: Create `lemma-static` crate

```rust
// crates/lemma-static/src/env_vars.rs
pub struct EnvVars;

impl EnvVars {
    /// Override the default Lemma home directory
    pub const LEMMA_HOME: &'static str = "LEMMA_HOME";

    /// Specify which toolchain to use
    pub const LEMMA_TOOLCHAIN: &'static str = "LEMMA_TOOLCHAIN";

    /// Disable configuration file discovery
    pub const LEMMA_NO_CONFIG: &'static str = "LEMMA_NO_CONFIG";

    /// Set verbosity level (0-3)
    pub const LEMMA_VERBOSE: &'static str = "LEMMA_VERBOSE";

    /// Force color output (always/never/auto)
    pub const LEMMA_COLOR: &'static str = "LEMMA_COLOR";

    // ... more env vars
}
```

#### Step 2: Create `lemma-cli` crate

```rust
// crates/lemma-cli/src/lib.rs
use clap::{Parser, Subcommand};
use lemma_static::EnvVars;

#[derive(Parser)]
#[command(name = "lemma", about = "A modern Lean4 toolchain manager")]
#[command(styles = STYLES)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    #[command(flatten)]
    pub top_level: TopLevelArgs,
}

#[derive(Parser)]
pub struct TopLevelArgs {
    #[command(flatten)]
    pub global_args: Box<GlobalArgs>,

    /// Display help
    #[arg(global = true, short, long, action = clap::ArgAction::HelpShort)]
    help: Option<bool>,

    /// Display version
    #[arg(global = true, short = 'V', long, action = clap::ArgAction::Version)]
    version: Option<bool>,
}

#[derive(Parser, Debug, Clone)]
#[command(next_help_heading = "Global options")]
pub struct GlobalArgs {
    /// Increase verbosity (-v, -vv, -vvv)
    #[arg(global = true, action = clap::ArgAction::Count, long, short,
          conflicts_with = "quiet", env = EnvVars::LEMMA_VERBOSE)]
    pub verbose: u8,

    /// Decrease output (-q, -qq)
    #[arg(global = true, action = clap::ArgAction::Count, long, short,
          conflicts_with = "verbose")]
    pub quiet: u8,

    /// Control colored output
    #[arg(global = true, long, value_enum, env = EnvVars::LEMMA_COLOR)]
    pub color: Option<ColorChoice>,

    /// Override Lemma home directory
    #[arg(global = true, long, env = EnvVars::LEMMA_HOME)]
    pub lemma_home: Option<PathBuf>,
}

#[derive(Debug, Copy, Clone, clap::ValueEnum)]
pub enum ColorChoice {
    Auto,
    Always,
    Never,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Manage Lean toolchains
    Toolchain {
        #[command(subcommand)]
        command: ToolchainCommands,
    },
    // ... other commands
}
```

#### Step 3: Create Settings in `lemma-rs`

```rust
// crates/lemma-rs/src/settings.rs
use lemma_cli::{GlobalArgs, ColorChoice};

#[derive(Debug, Clone)]
pub(crate) struct GlobalSettings {
    pub(crate) verbose: u8,
    pub(crate) quiet: u8,
    pub(crate) color: ColorChoice,
    pub(crate) lemma_home: PathBuf,
}

impl GlobalSettings {
    pub(crate) fn resolve(args: &GlobalArgs) -> Self {
        let color = args.color.unwrap_or(ColorChoice::Auto);

        let lemma_home = args.lemma_home
            .clone()
            .or_else(|| std::env::var("LEMMA_HOME").ok().map(PathBuf::from))
            .unwrap_or_else(|| {
                dirs::home_dir()
                    .expect("Could not determine home directory")
                    .join(".lemma")
            });

        Self {
            verbose: args.verbose,
            quiet: args.quiet,
            color,
            lemma_home,
        }
    }
}
```

#### Step 4: Update Main Entry Point

```rust
// crates/lemma-rs/src/main.rs
use lemma_cli::{Cli, Commands};
use clap::Parser;
use anyhow::Result;

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {:#}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    // Parse CLI
    let cli = Cli::parse();

    // Resolve settings
    let settings = GlobalSettings::resolve(&cli.top_level.global_args);

    // Setup logging
    setup_logging(settings.verbose, settings.quiet);

    // Dispatch to commands
    match cli.command {
        Commands::Toolchain { command } => {
            commands::toolchain::execute(command, settings)
        }
        // ... other commands
    }
}

fn setup_logging(verbose: u8, quiet: u8) {
    let level = match (verbose, quiet) {
        (0, 0) => "info",
        (1, 0) => "debug",
        (2.., 0) => "trace",
        (0, 1) => "warn",
        (0, 2..) => "error",
        _ => "info",
    };

    std::env::set_var("RUST_LOG", level);
    tracing_subscriber::fmt::init();
}
```

## Benefits of This Architecture

1. **Separation of Concerns**: CLI definitions separate from implementation
2. **Faster Compilation**: Changes to logic don't recompile CLI parsing
3. **Testability**: Can test commands with Settings directly
4. **Configuration Merging**: Easy to layer CLI + config + env + defaults
5. **Maintainability**: Each crate has a single, clear purpose
6. **Reusability**: CLI crate can be used by multiple binaries

## Migration Path

You don't need to do everything at once. Incremental steps:

1. ✅ **Phase 1**: Keep current structure, add GlobalArgs with proper flags
2. **Phase 2**: Add Settings structs that convert CLI → Settings
3. **Phase 3**: Extract CLI to separate crate
4. **Phase 4**: Add config file support
5. **Phase 5**: Create domain-specific crates as needed

You're currently at Phase 1, which is a great start!
