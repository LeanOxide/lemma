# Lemma - A Modern Lean4 Toolchain Manager

[English](README.md) | [简体中文](README_CN.md)

**Lemma** is a rewrite of [elan](https://github.com/leanprover/elan) that addresses critical usability issues, particularly around proxy support and custom toolchain sources.

## Why Lemma?

After analyzing the elan codebase, we identified several critical issues that make it difficult to use in enterprise and restricted network environments:

## Key Features

### Full Proxy Support
- **HTTP, HTTPS, and SOCKS5 proxies** with authentication
- Standard environment variables: `HTTP_PROXY`, `HTTPS_PROXY`, `NO_PROXY`

### Custom Sources and Mirrors
- Configure custom registry URLs

```toml
release_url = "https://release.custom.org"
```

## Installation

### Quick Install (Recommended)

**Linux / macOS:**

```bash
curl --proto '=https' --tlsv1.2 -sSf https://lemma.puqing.work/install.sh | sh
```

Or download and inspect the script first:

```bash
curl --proto '=https' --tlsv1.2 -sSfL https://lemma.puqing.work/install.sh -o install.sh
chmod +x install.sh
./install.sh
```

**Windows (PowerShell):**

```powershell
irm https://lemma.puqing.work/install.ps1 | iex
```

Or download and inspect the script first:

```powershell
Invoke-WebRequest -Uri https://lemma.puqing.work/install.ps1 -OutFile install.ps1
.\install.ps1
```

### From Source

```bash
# Build from source
cargo build --release

# Install
cargo install --path .
```

### Self-Update

Once installed, you can update lemma itself:

```bash
lemma self update
```

This will check for the latest version and download it automatically if a newer version is available.

## Usage

### Basic Commands

```bash
# Install a toolchain
lemma toolchain install stable
lemma toolchain install nightly
lemma toolchain install v4.0.0

# List installed toolchains
lemma toolchain list

# Set default toolchain
lemma default stable

# Update toolchains
lemma update

# Show information
lemma info

# Self-management
lemma self update              # Update lemma itself
lemma self uninstall           # Uninstall lemma and all toolchains
```

## Configuration File

Lemma stores its configuration in `~/.lemma/config.toml` (or `$LEMMA_HOME/config.toml`).

Example configuration:

```toml
version = "1"
default_toolchain = "stable"
path_setup_shown = true
release_url = "https://release.lean-lang.org"

[overrides]
```

## Environment Variables

Lemma respects standard proxy environment variables:

- `HTTP_PROXY` / `http_proxy` - HTTP proxy URL
- `HTTPS_PROXY` / `https_proxy` - HTTPS proxy URL
- `ALL_PROXY` / `all_proxy` - Proxy for all protocols
- `NO_PROXY` / `no_proxy` - Comma-separated list of domains to bypass proxy
- `LEMMA_HOME` - Lemma installation directory (default: `~/.lemma`)
- `LEMMA_RELEASE_URL` - Override default release server

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
