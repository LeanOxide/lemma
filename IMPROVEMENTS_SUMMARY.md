# Lemma CLI Improvements - Summary

## What We Accomplished

### 1. ✅ Fixed Workspace Structure
- Fixed `crates/lemma-rs/Cargo.toml` to properly inherit workspace metadata and dependencies
- Moved `[profile.release]` settings to workspace root (where they belong)
- Resolved compilation errors

### 2. ✅ Adopted UV's CLI Architecture Patterns
You successfully implemented UV's three-layer argument structure:

```
Cli
├── command: Commands
└── top_level: TopLevelArgs
    ├── global_args: Box<GlobalArgs>  ✅ Implemented
    ├── help: Option<bool>            ✅ Implemented
    └── version: Option<bool>         ✅ Implemented
```

**GlobalArgs** now includes:
- ✅ `verbose: u8` - Count flag for `-v`, `-vv`, `-vvv`
- ✅ `quiet: u8` - Count flag for `-q`, `-qq`
- ✅ `color: Option<ColorChoice>` - Color control (auto/always/never)
- ✅ `no_color: bool` - Compatibility flag
- ✅ Proper `conflicts_with` constraints
- ✅ Environment variable support

### 3. ✅ Created `lemma-static` Crate
Following UV's pattern, we created a dedicated crate for static constants:

**File: `crates/lemma-static/src/env_vars.rs`**
```rust
pub struct EnvVars;

impl EnvVars {
    pub const LEMMA_HOME: &'static str = "LEMMA_HOME";
    pub const LEMMA_TOOLCHAIN: &'static str = "LEMMA_TOOLCHAIN";
    pub const LEMMA_NO_CONFIG: &'static str = "LEMMA_NO_CONFIG";
    pub const LEMMA_VERBOSE: &'static str = "LEMMA_VERBOSE";
    pub const LEMMA_COLOR: &'static str = "LEMMA_COLOR";
    pub const NO_COLOR: &'static str = "NO_COLOR";
    // ... and more
}
```

**Benefits:**
- Single source of truth for environment variable names
- Easy to document and maintain
- Type-safe references throughout the codebase

### 4. ✅ Added Environment Variable Support
Enabled the `env` feature for clap and added environment variable bindings:

```rust
#[arg(global = true, long, short, env = "LEMMA_VERBOSE")]
pub verbose: u8,

#[arg(global = true, long, env = "LEMMA_COLOR")]
pub color: Option<ColorChoice>,
```

**Now users can configure lemma via:**
- Command-line flags: `lemma -v show`
- Environment variables: `LEMMA_VERBOSE=2 lemma show`
- Both (CLI takes precedence)

## Testing

```bash
# Test environment variables work
LEMMA_VERBOSE=2 ./target/debug/lemma --help
# Shows: [env: LEMMA_VERBOSE=2]

LEMMA_COLOR=always ./target/debug/lemma show

# Test count flags
./target/debug/lemma -vv show    # Very verbose
./target/debug/lemma -q show     # Quiet
```

## Current Workspace Structure

```
lemma/
├── Cargo.toml                          # Workspace root with shared config
├── CLI_ARCHITECTURE_GUIDE.md           # Detailed guide on UV patterns
├── IMPROVEMENTS_SUMMARY.md             # This file
└── crates/
    ├── lemma-static/                   # ✅ NEW: Static constants
    │   └── src/
    │       ├── lib.rs
    │       └── env_vars.rs
    └── lemma-rs/                       # Main binary
        ├── Cargo.toml                  # ✅ Fixed: proper dependencies
        └── src/
            ├── main.rs
            ├── cli.rs                  # ✅ Improved: UV-style GlobalArgs
            ├── commands.rs
            └── ...
```

## What's Left to Do (Next Steps)

Following the phases outlined in `CLI_ARCHITECTURE_GUIDE.md`:

### Phase 2: Settings Layer (Next Priority)
Create a Settings layer to separate CLI args from business logic:

**File: `crates/lemma-rs/src/settings.rs`** (create this)
```rust
use crate::cli::{GlobalArgs, ColorChoice};

/// Resolved global settings used throughout the application
#[derive(Debug, Clone)]
pub struct GlobalSettings {
    pub verbose: u8,
    pub quiet: u8,
    pub color: ColorChoice,
    pub lemma_home: PathBuf,
}

impl GlobalSettings {
    /// Resolve settings from CLI args + environment + config + defaults
    pub fn resolve(args: &GlobalArgs) -> Self {
        // Priority order:
        // 1. CLI args (highest)
        // 2. Environment variables
        // 3. Config file
        // 4. Defaults (lowest)

        let color = args.color
            .or_else(|| {
                if args.no_color {
                    Some(ColorChoice::Never)
                } else {
                    None
                }
            })
            .unwrap_or(ColorChoice::Auto);

        let lemma_home = std::env::var("LEMMA_HOME")
            .ok()
            .map(PathBuf::from)
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

    /// Get the appropriate log level based on verbosity
    pub fn log_level(&self) -> &'static str {
        match (self.verbose, self.quiet) {
            (0, 0) => "info",
            (1, 0) => "debug",
            (2.., 0) => "trace",
            (0, 1) => "warn",
            (0, 2..) => "error",
            _ => "info",
        }
    }
}
```

**Update main.rs to use Settings:**
```rust
fn run() -> Result<()> {
    let cli = Cli::parse();

    // Resolve settings (this is where you'd merge config files, env, etc.)
    let settings = GlobalSettings::resolve(&cli.top_level.global_args);

    // Setup logging using settings
    setup_logging(&settings);

    // Pass settings to commands, not raw CLI args
    commands::handle_command(cli.command, settings)
}

fn setup_logging(settings: &GlobalSettings) {
    std::env::set_var("RUST_LOG", settings.log_level());
    tracing_subscriber::fmt::init();
}
```

**Why this matters:**
- Commands work with Settings, never raw CLI args
- Easy to add config file support later
- Settings resolves conflicts (e.g., `--no-color` vs `--color never`)
- Settings computes derived values (like log level)

### Phase 3: Configuration Files (Future)
Add support for `lemma.toml` configuration:

```toml
# ~/.lemma/lemma.toml or project ./lemma.toml
[global]
verbose = 1
color = "auto"

[mirrors]
github = "https://custom-mirror.example.com"

[cache]
max_size = "10GB"
```

**What you need:**
1. Define config structs (using serde)
2. Load from multiple locations (project, user, system)
3. Merge config + CLI with proper precedence
4. Update `GlobalSettings::resolve()` to include config

### Phase 4: Separate CLI Crate (Future)
Extract CLI to `crates/lemma-cli/`:

```
crates/
├── lemma-cli/       # Pure CLI definitions
├── lemma-static/    # Constants & env vars
├── lemma-rs/        # Main binary & commands
└── lemma-config/    # Config file handling
```

**Benefits:**
- Faster compilation (logic changes don't recompile CLI)
- CLI can be used by multiple binaries
- Clean separation of concerns

### Phase 5: Domain-Specific Crates (Optional)
As the project grows, consider extracting modules:

```
crates/
├── lemma-cli/           # CLI definitions
├── lemma-static/        # Constants
├── lemma-config/        # Config handling
├── lemma-toolchain/     # Toolchain management logic
├── lemma-download/      # Download & caching
├── lemma-install/       # Installation logic
└── lemma-rs/            # Main binary (orchestration)
```

## Key Learnings from UV

1. **Separation of Concerns**: CLI definitions separate from implementation
2. **Box<> for Large Structs**: Reduces stack size (`Box<GlobalArgs>`)
3. **Count Actions**: For verbosity levels (`-v`, `-vv`, `-vvv`)
4. **Conflicts**: Use `conflicts_with` for mutually exclusive flags
5. **Hidden Flags**: Use `hide = true` for deprecated/compatibility flags
6. **Environment Variables**: Document via static constants
7. **Settings Pattern**: Convert CLI → Settings for business logic
8. **Configuration Layers**: CLI > Env > Config > Defaults

## Documentation References

- **Detailed Architecture Guide**: See `CLI_ARCHITECTURE_GUIDE.md`
- **UV Repository**: `/home/happy/work/uv/crates/`
  - `uv-cli/src/lib.rs` - CLI structure
  - `uv-static/src/env_vars.rs` - Environment variables
  - `uv/src/settings.rs` - Settings pattern
  - `uv/src/lib.rs` - Main orchestration

## Quick Reference: Common Tasks

### Adding a New Environment Variable
1. Add to `lemma-static/src/env_vars.rs`:
   ```rust
   pub const LEMMA_NEW_VAR: &'static str = "LEMMA_NEW_VAR";
   ```

2. Add to CLI argument:
   ```rust
   #[arg(long, env = "LEMMA_NEW_VAR")]
   pub new_option: Option<String>,
   ```

### Adding a New Global Flag
1. Add to `GlobalArgs` in `cli.rs`:
   ```rust
   #[arg(global = true, long)]
   pub new_flag: bool,
   ```

2. Add to `GlobalSettings` in `settings.rs`:
   ```rust
   pub struct GlobalSettings {
       pub new_flag: bool,
       // ...
   }
   ```

3. Update `GlobalSettings::resolve()`:
   ```rust
   Self {
       new_flag: args.new_flag,
       // ...
   }
   ```

### Testing CLI Changes
```bash
# Build
cargo build

# Test help shows environment variable
LEMMA_VERBOSE=2 ./target/debug/lemma --help

# Test flag works
./target/debug/lemma -vv show

# Test env var works
LEMMA_VERBOSE=3 ./target/debug/lemma show
```

## Conclusion

You've successfully migrated to UV's proven CLI architecture patterns! The codebase now has:

✅ Clean separation between CLI and implementation
✅ Environment variable support
✅ Professional flag handling (count, conflicts, etc.)
✅ Static constants for maintainability
✅ Proper workspace structure

Next step: Implement the Settings layer to complete Phase 2!
