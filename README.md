# Lemma - A Modern Lean4 Toolchain Manager

**Lemma** is a rewrite of [elan](https://github.com/leanprover/elan) that addresses critical usability issues, particularly around proxy support and custom toolchain sources.

## Why Lemma?

After analyzing the elan codebase, we identified several critical issues that make it difficult to use in enterprise and restricted network environments:

## Key Features

### Full Proxy Support
- **HTTP, HTTPS, and SOCKS5 proxies** with authentication
- Standard environment variables: `HTTP_PROXY`, `HTTPS_PROXY`, `NO_PROXY`
- Configuration file support for persistent settings
- Per-request proxy authentication

```bash
# Set proxy via command
lemma proxy set-http http://proxy.company.com:8080
lemma proxy set-auth username

# Or use environment variables
export HTTP_PROXY=http://proxy.company.com:8080
export HTTPS_PROXY=https://proxy.company.com:8080
```

### Custom Sources and Mirrors
- Configure custom registry URLs
- Multiple fallback mirrors
- Custom GitHub Enterprise API support
- Local toolchain linking

```toml
[sources]
release_url = "https://mirror.example.com/lean"
mirrors = [
    "https://mirror1.example.com/lean",
    "https://mirror2.example.com/lean"
]
github_api = "https://github.company.com/api/v3"
github_token = "ghp_xxxxxxxxxxxxx"
```

### Proper GitHub REST API
- Uses official GitHub API v3 instead of HTML scraping
- Personal access token support for rate limits
- Robust platform asset detection
- Cached release metadata

### Network Resilience
- **Automatic retry** with exponential backoff
- **Download resumption** for interrupted transfers
- **Configurable timeouts** and retry policies
- **Bandwidth limiting** for metered connections

```toml
[network]
connect_timeout = 30      # seconds
read_timeout = 30         # seconds
max_retries = 3           # retry attempts
retry_delay = 2           # base delay in seconds
max_download_speed = 0    # bytes/sec, 0 = unlimited
resume_downloads = true   # resume partial downloads
```

### Better Error Messages
- Specific error types for different failure modes
- Network diagnostic information
- Actionable suggestions for users

## Installation

```bash
# Build from source
cargo build --release

# Install
cargo install --path .

# Initialize
lemma init
```

## Usage

### Basic Commands

```bash
# Initialize lemma
lemma init

# Install a toolchain
lemma install stable
lemma install nightly
lemma install v4.0.0
lemma install leanprover/lean4:v4.0.0

# List installed toolchains
lemma list

# Set default toolchain
lemma default stable

# Update toolchains
lemma update

# Show information
lemma info
```

### Proxy Configuration

```bash
# Show current proxy settings
lemma proxy show

# Set HTTP proxy
lemma proxy set-http http://proxy.example.com:8080

# Set HTTPS proxy
lemma proxy set-https https://proxy.example.com:8080

# Set SOCKS5 proxy
lemma proxy set-socks socks5://127.0.0.1:1080

# Set proxy authentication
lemma proxy set-auth myusername
# (will prompt for password)

# Clear proxy settings
lemma proxy clear
```

### Configuration Management

```bash
# Show configuration
lemma config

# Get config file path
lemma config --path

# Edit configuration
lemma config --edit
```

## Configuration File

Lemma stores its configuration in `~/.lemma/config.toml` (or `$LEMMA_HOME/config.toml`).

Example configuration:

```toml
default_toolchain = "stable"
telemetry = false

[network]
http_proxy = "http://proxy.company.com:8080"
https_proxy = "https://proxy.company.com:8080"
proxy_auth = "username:password"  # or set via lemma proxy set-auth
connect_timeout = 30
read_timeout = 30
max_retries = 3
retry_delay = 2
max_download_speed = 0
resume_downloads = true
insecure = false  # DANGEROUS: skip SSL verification

[sources]
release_url = "https://release.lean-lang.org"
mirrors = []
github_api = "https://api.github.com"
github_token = "ghp_xxxxxxxxxxxxx"  # GitHub personal access token

[sources.custom_registries]
# custom-name = "https://custom-registry.example.com"
```

## Environment Variables

Lemma respects standard proxy environment variables:

- `HTTP_PROXY` / `http_proxy` - HTTP proxy URL
- `HTTPS_PROXY` / `https_proxy` - HTTPS proxy URL
- `ALL_PROXY` / `all_proxy` - Proxy for all protocols
- `NO_PROXY` / `no_proxy` - Comma-separated list of domains to bypass proxy
- `LEMMA_HOME` - Lemma installation directory (default: `~/.lemma`)
- `LEMMA_GITHUB_TOKEN` - GitHub personal access token
- `LEMMA_RELEASE_URL` - Override default release server

## Architecture

Lemma is built with a modern, modular architecture:

### Modules

- **`config.rs`** - Configuration management with TOML support
  - Environment variable overrides
  - Proxy settings
  - Custom registry configuration

- **`download.rs`** - Download client with full proxy support
  - Uses `reqwest` with HTTP, HTTPS, and SOCKS5 proxy support
  - Automatic retry with exponential backoff
  - Download resumption
  - Progress reporting

- **`github.rs`** - GitHub API client (not HTML scraping!)
  - Uses GitHub REST API v3
  - Authentication token support
  - Platform asset detection
  - Release caching

- **`errors.rs`** - Structured error types
  - Specific error variants for different failures
  - Diagnostic information
  - Actionable suggestions

- **`cli.rs`** - Command-line interface
  - Built with `clap` for robust argument parsing
  - Subcommands for all operations

## Development Status

**Current Status:** Early Development / Proof of Concept

### 🚧 In Progress
- Toolchain installation logic
- Toolchain management (list, update, uninstall)
- Override system (per-project toolchains)
- Self-update functionality

### 📋 Planned
- Toolchain binary proxying (like elan's symlink system)
- Telemetry (opt-in)
- Migration tool from elan
- Comprehensive test suite

## Contributing

Contributions are welcome! Key areas that need work:

1. **Toolchain Installation** - Implement the full download and install pipeline
2. **Binary Proxying** - Implement the toolchain binary wrapper system
3. **Testing** - Add comprehensive test coverage
4. **Documentation** - Expand user and developer documentation
5. **Platform Support** - Test on Windows, macOS, Linux

## License

[MIT]

## Acknowledgments

- **Elan** - The original Lean toolchain manager that inspired this project
- **Rustup** - Design inspiration for toolchain management
---

**Note:** Lemma is in early development. While the core infrastructure is in place, toolchain installation is not yet fully implemented. Use at your own risk.
