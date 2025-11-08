# Lemma - A Modern Lean4 Toolchain Manager

![GitHub Actions Workflow Status](https://img.shields.io/github/actions/workflow/status/AndPuQing/lemma/ci.yml?style=flat-square&logo=github)
![Crates.io Version](https://img.shields.io/crates/v/lemma-rs?style=flat-square&logo=rust)
![Crates.io Downloads (recent)](https://img.shields.io/crates/dr/lemma-rs?style=flat-square)
[![dependency status](https://deps.rs/repo/github/AndPuQing/lemma-rs/status.svg?style=flat-square)](https://deps.rs/repo/github/AndPuQing/lemma-rs)
![Crates.io License](https://img.shields.io/crates/l/lemma-rs?style=flat-square)
![Crates.io Size](https://img.shields.io/crates/size/lemma-rs?style=flat-square)
[![codecov](https://codecov.io/github/andpuqing/lemma/graph/badge.svg?token=X0RRVLGQZQ)](https://codecov.io/github/andpuqing/lemma)

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
default_toolchain = "leanprover/lean4:stable"
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
- `LEMMA_TOOLCHAIN` - Override active toolchain for current session

## Advanced Usage

### Project-specific Toolchains

Lemma automatically detects project-specific toolchains from:

1. **lean-toolchain file**: Create a `lean-toolchain` file in your project root:
   ```
   stable
   ```
   or with full specification:
   ```
   leanprover/lean4:v4.25.0
   ```

2. **leanpkg.toml**: Specify `lean_version` in your package configuration:
   ```toml
   lean_version = "v4.25.0"
   ```

### Directory Overrides

Set a toolchain for a specific directory and all subdirectories:

```bash
cd my-project
lemma override set stable
```

Remove the override:

```bash
lemma override unset
```

List all directory overrides:

```bash
lemma override list
```

### Custom Release Sources

Configure a custom release server in `~/.lemma/config.toml`:

```toml
release_url = "https://mirror.example.com/lean-releases"
```

Or use environment variable:

```bash
export LEMMA_RELEASE_URL=https://mirror.example.com/lean-releases
```

## Toolchain Resolution

Lemma resolves which toolchain to use in the following priority order:

1. **Explicit override**: `+toolchain` syntax (e.g., `lean +nightly test.lean`)
2. **Environment variable**: `LEMMA_TOOLCHAIN`
3. **Directory override**: Set via `lemma override set`
4. **Project file**: `lean-toolchain` or `leanpkg.toml` in current directory or parent directories
5. **Default toolchain**: Configured via `lemma default <toolchain>`

## Troubleshooting

### Toolchain not found

If you see "Toolchain not installed" errors:

```bash
# List installed toolchains
lemma toolchain list

# Install the required toolchain
lemma toolchain install stable
```

### Proxy connection issues

If downloads fail behind a proxy:

```bash
# Verify proxy settings
echo $HTTPS_PROXY

# Test with curl
curl -v https://release.lean-lang.org

# Set proxy for lemma
export HTTPS_PROXY=http://your-proxy:port
```

### Permission denied errors

On Linux/macOS, ensure lemma's bin directory is in your PATH and has execute permissions:

```bash
chmod +x ~/.lemma/bin/lemma
export PATH="$HOME/.lemma/bin:$PATH"
```

## Contributing

Contributions are welcome! Key areas that need work:

1. **Toolchain Installation** - Implement the full download and install pipeline
2. **Binary Proxying** - Implement the toolchain binary wrapper system
3. **Testing** - Add comprehensive test coverage
4. **Documentation** - Expand user and developer documentation
5. **Platform Support** - Test on Windows, macOS, Linux

## License

[MIT](LICENSE)

## Acknowledgments

- **Elan** - The original Lean toolchain manager that inspired this project

---

**Note:** Lemma is in early development. While the core infrastructure is in place, toolchain installation is not yet fully implemented. Use at your own risk.
