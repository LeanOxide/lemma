//! Command-line interface for Lemma
//!
//! Note: Environment variable names in clap attributes (e.g., `env = "LEMMA_VERBOSE"`)
//! must be string literals and match the constants defined in `lemma_static::EnvVars`.

use crate::help;
use clap::builder::styling::{AnsiColor, Effects};
use clap::builder::Styles;
use clap::{Parser, Subcommand};
use clap_complete::Shell;
use lemma_static::EnvVars;

const STYLES: Styles = Styles::styled()
    .header(AnsiColor::Green.on_default().effects(Effects::BOLD))
    .usage(AnsiColor::Green.on_default().effects(Effects::BOLD))
    .literal(AnsiColor::Cyan.on_default().effects(Effects::BOLD))
    .placeholder(AnsiColor::Cyan.on_default());

#[derive(Parser)]
#[command(name = "lemma")]
#[command(about = "A modern Lean4 toolchain manager", long_about = None)]
#[command(version)]
#[command(after_long_help = "")]
#[command(styles=STYLES)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    #[command(flatten)]
    pub top_level: TopLevelArgs,
}

#[derive(Parser)]
#[command(disable_help_flag = true, disable_version_flag = true)]
pub struct TopLevelArgs {
    // #[command(flatten)]
    // pub cache_args: Box<CacheArgs>,
    #[command(flatten)]
    pub global_args: Box<GlobalArgs>,

    /// Display the concise help for this command.
    #[arg(global = true, short, long, action = clap::ArgAction::HelpShort, help_heading = "Global options")]
    help: Option<bool>,

    /// Display the lemma version.
    #[arg(short = 'V', long, action = clap::ArgAction::Version)]
    version: Option<bool>,
}

#[derive(Parser, Debug, Clone)]
#[command(next_help_heading = "Global options", next_display_order = 1000)]
pub struct GlobalArgs {
    /// Use quiet output.
    ///
    /// Repeating this option, e.g., `-qq`, will enable a silent mode in which
    /// lemma will write minimal output.
    #[arg(global = true, action = clap::ArgAction::Count, long, short, conflicts_with = "verbose")]
    pub quiet: u8,

    /// Use verbose output.
    ///
    /// Repeating this option, e.g., `-vv`, will increase verbosity further.
    /// You can configure fine-grained logging using the `RUST_LOG` environment variable.
    /// (<https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html#directives>)
    #[arg(global = true, action = clap::ArgAction::Count, long, short,
          conflicts_with = "quiet", env = "LEMMA_VERBOSE")]
    pub verbose: u8,

    /// Disable colors.
    ///
    /// Provided for compatibility with other tools, use `--color never` instead.
    #[arg(
        global = true,
        long,
        hide = true,
        conflicts_with = "color",
        env = "NO_COLOR"
    )]
    pub no_color: bool,

    /// Control the use of color in output.
    ///
    /// By default, lemma will automatically detect support for colors when writing to a terminal.
    #[arg(
        global = true,
        long,
        value_enum,
        conflicts_with = "no_color",
        value_name = "COLOR_CHOICE",
        env = "LEMMA_COLOR"
    )]
    pub color: Option<lemma_config::ColorChoice>,
}

impl From<&GlobalArgs> for lemma_config::CliArgs {
    fn from(args: &GlobalArgs) -> Self {
        Self {
            quiet: args.quiet,
            verbose: args.verbose,
            no_color: args.no_color,
            color: args.color,
        }
    }
}

#[derive(Subcommand)]
pub enum Commands {
    /// Manage toolchains
    #[command(after_long_help = help::TOOLCHAIN_HELP)]
    Lean {
        #[command(subcommand)]
        command: ToolchainCommands,
    },

    /// Modify directory toolchain overrides
    #[command(after_long_help = help::OVERRIDE_HELP)]
    Override {
        #[command(subcommand)]
        command: OverrideCommands,
    },

    /// Set the default toolchain
    #[command(after_long_help = help::DEFAULT_HELP)]
    Default {
        /// Toolchain to set as default
        toolchain: String,
    },

    /// Show the active toolchain and installed toolchains
    #[command(after_long_help = help::SHOW_HELP)]
    Show,

    /// Display the path to a binary in the active toolchain
    #[command(after_long_help = help::WHICH_HELP)]
    Which {
        /// Name of the binary (e.g., lean, lake, leanc)
        binary: String,

        /// Toolchain to use (defaults to active toolchain)
        #[arg(short, long)]
        toolchain: Option<String>,
    },

    /// Run a command with a toolchain
    #[command(after_long_help = help::RUN_HELP)]
    Run {
        /// Toolchain to use (e.g., stable, v4.24.0)
        toolchain: String,

        /// Command and arguments to run
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        command: Vec<String>,
    },

    /// Generate tab-completion scripts for your shell
    #[command(after_long_help = help::COMPLETIONS_HELP)]
    Completions {
        /// Shell type
        shell: Shell,
    },

    /// Fetch dependency caches (e.g., mathlib4)
    Fetch {
        /// Package to fetch (e.g., mathlib4)
        package: String,

        /// Specific modules to fetch (can be specified multiple times)
        #[arg(short, long = "module", value_name = "MODULE")]
        modules: Vec<String>,

        /// Auto-detect modules from project imports
        #[arg(short, long, conflicts_with = "modules")]
        auto: bool,

        /// Show what would be downloaded without actually downloading
        #[arg(long)]
        dry_run: bool,

        /// Project path to analyze (defaults to current directory)
        #[arg(long)]
        path: Option<String>,
    },

    /// Manage the lemma cache
    #[command(after_long_help = help::CACHE_HELP)]
    Cache {
        #[command(subcommand)]
        command: CacheCommands,
    },

    /// Initialize a new Lean project
    #[command(after_long_help = help::INIT_HELP)]
    Init {
        /// Name of the project (defaults to directory name)
        name: Option<String>,

        /// Path to create the project (defaults to current directory)
        #[arg(long)]
        path: Option<String>,

        /// Create a minimal project with only lakefile.toml
        #[arg(long)]
        bare: bool,

        /// Create a standard project with library and executable (default)
        #[arg(long, alias = "app", conflicts_with_all = ["exe", "lib"])]
        std: bool,

        /// Create an executable-only project
        #[arg(long, conflicts_with_all = ["std", "lib"])]
        exe: bool,

        /// Create a library-only project
        #[arg(long, conflicts_with_all = ["std", "exe"])]
        lib: bool,

        /// Do not create a README.md file
        #[arg(long)]
        no_readme: bool,
    },

    /// Build the current Lean project
    #[command(after_long_help = help::BUILD_HELP)]
    Build {
        /// Toolchain to use for building (defaults to active toolchain)
        #[arg(short, long)]
        toolchain: Option<String>,

        /// Project path (defaults to current directory)
        #[arg(long)]
        path: Option<String>,

        /// Build targets and additional arguments to pass to lake
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Modify the lemma installation
    #[command(after_long_help = help::SELF_HELP)]
    #[command(name = "self")]
    Self_ {
        #[command(subcommand)]
        command: SelfCommands,
    },
}

#[derive(Subcommand)]
pub enum SelfCommands {
    /// Update lemma to the latest version
    #[command(after_long_help = help::SELF_UPDATE_HELP)]
    Update,

    /// Uninstall lemma and all installed toolchains
    #[command(after_long_help = help::SELF_UNINSTALL_HELP)]
    Uninstall {
        /// Skip confirmation prompt
        #[arg(short = 'y', long)]
        yes: bool,
    },
}

#[derive(Subcommand)]
pub enum ToolchainCommands {
    /// Install a toolchain
    #[command(after_long_help = help::TOOLCHAIN_INSTALL_HELP)]
    Install {
        /// Toolchain to install (e.g., stable, v4.24.0)
        toolchain: String,

        /// Force reinstall if already installed
        #[arg(short, long)]
        force: bool,

        /// Custom URL pointing to a JSON file containing Lean installation metadata.
        ///
        /// This allows using alternative Lean distribution sources instead of the
        /// official release server at https://release.lean-lang.org.
        ///
        /// The JSON file should follow the same format as the official release index.
        #[arg(long, value_name = "URL", env = EnvVars::LEMMA_RELEASE_URL)]
        lean_release_json_url: Option<String>,
    },

    /// Uninstall a toolchain
    #[command(after_long_help = help::TOOLCHAIN_UNINSTALL_HELP)]
    Uninstall {
        /// Toolchain to uninstall
        toolchain: String,
    },

    /// List installed toolchains
    #[command(after_long_help = help::TOOLCHAIN_LIST_HELP)]
    List {
        /// Show only installed toolchains
        #[arg(long, conflicts_with = "only_available")]
        only_installed: bool,

        /// Show only available toolchains for download
        #[arg(long, conflicts_with = "only_installed")]
        only_available: bool,

        /// Custom URL pointing to a JSON file containing Lean installation metadata.
        ///
        /// This allows using alternative Lean distribution sources instead of the
        /// official release server at https://release.lean-lang.org.
        #[arg(long, value_name = "URL", env = EnvVars::LEMMA_RELEASE_URL)]
        lean_release_json_url: Option<String>,
    },

    /// Show the installation directory for toolchains
    #[command(after_long_help = help::TOOLCHAIN_DIR_HELP)]
    Dir {
        /// Show the directory for a specific toolchain
        toolchain: Option<String>,
    },

    /// Find an installed toolchain matching a request
    #[command(after_long_help = help::TOOLCHAIN_FIND_HELP)]
    Find {
        /// Toolchain request (e.g., stable, v4, v4.24, v4.24.0)
        request: Option<String>,
    },

    /// Link a custom toolchain
    #[command(after_long_help = help::TOOLCHAIN_LINK_HELP)]
    Link {
        /// Name for the toolchain
        name: String,

        /// Path to the toolchain directory
        path: String,
    },

    /// Upgrade installed toolchains
    #[command(after_long_help = help::TOOLCHAIN_UPGRADE_HELP)]
    Upgrade {
        /// Specific toolchain to upgrade (upgrades all if not specified)
        toolchain: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum OverrideCommands {
    /// Set directory override for a toolchain
    #[command(after_long_help = help::OVERRIDE_SET_HELP)]
    Set {
        /// Toolchain to use in this directory
        toolchain: String,

        /// Directory to override (defaults to current directory)
        #[arg(long)]
        path: Option<String>,
    },

    /// Remove directory override
    #[command(after_long_help = help::OVERRIDE_UNSET_HELP)]
    Unset {
        /// Directory to remove override from (defaults to current directory)
        #[arg(long)]
        path: Option<String>,
    },

    /// List all directory overrides
    #[command(after_long_help = help::OVERRIDE_LIST_HELP)]
    List,
}

#[derive(Subcommand)]
pub enum CacheCommands {
    /// Show cache directory path
    #[command(after_long_help = help::CACHE_DIR_HELP)]
    Dir,

    /// Display cache statistics
    #[command(after_long_help = help::CACHE_STATS_HELP)]
    Stats,

    /// Remove cached downloads
    #[command(after_long_help = help::CACHE_CLEAN_HELP)]
    Clean {
        /// Don't ask for confirmation
        #[arg(short = 'y', long)]
        yes: bool,
    },

    /// Remove all cached data (downloads and toolchains)
    #[command(after_long_help = help::CACHE_PRUNE_HELP)]
    Prune {
        /// Don't ask for confirmation
        #[arg(short = 'y', long)]
        yes: bool,
    },
}
