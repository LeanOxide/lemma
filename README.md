# Lemma - A Modern Lean4 Toolchain Manager

![GitHub Actions Workflow Status](https://img.shields.io/github/actions/workflow/status/AndPuQing/lemma/ci.yml?style=flat-square&logo=github)
![PyPI Version](https://img.shields.io/pypi/v/lemma?style=flat-square&logo=pypi)
![PyPI Downloads](https://img.shields.io/pypi/dm/lemma?style=flat-square&logo=pypi)
[![dependency status](https://deps.rs/repo/github/AndPuQing/lemma-rs/status.svg?style=flat-square)](https://deps.rs/repo/github/AndPuQing/lemma-rs)
![PyPI License](https://img.shields.io/pypi/l/lemma?style=flat-square)
[![codecov](https://codecov.io/github/andpuqing/lemma/graph/badge.svg?token=X0RRVLGQZQ)](https://codecov.io/github/andpuqing/lemma)

[English](README.md) | [简体中文](README_CN.md)

**Lemma** is a rewrite of [elan](https://github.com/leanprover/elan) that addresses critical usability issues, particularly around proxy support and custom toolchain sources.

## Why Lemma?

After analyzing the elan codebase, we identified several critical issues that make it difficult to use in enterprise and restricted network environments.

## Key Features

### Full Proxy Support

- **HTTP, HTTPS, and SOCKS5 proxies** with authentication
- Standard environment variables: `HTTP_PROXY`, `HTTPS_PROXY`, `NO_PROXY`

### Custom Sources and Mirrors

Configure custom Lean release index URLs:

```toml
release_url = "https://release.custom.org"
```

## Installation

### Quick Install (Recommended)

Install Lemma as a Python package. The package name and command are both `lemma`.

```bash
pipx install lemma
```

If you do not use `pipx`, install with Python's user site instead:

```bash
python -m pip install --user lemma
```

On Windows, use the Python launcher if needed:

```powershell
py -m pip install --user lemma
```

After installation, run a setup command such as `lemma toolchain install stable`. Lemma will create proxy commands such as `lean`, `lake`, and `leanc` under `~/.lemma/bin`. Add that directory to your `PATH` if you want to call those proxies directly.

### From Source

```bash
# Build from source
cargo build --release -p lemma

# Install the CLI from this checkout
cargo install --path crates/lemma-rs
```

### Updating Lemma

Use the same package manager that installed Lemma:

```bash
pipx upgrade lemma
# or
python -m pip install --user --upgrade lemma
```

`lemma self update` prints these safe package-manager commands instead of replacing the running binary directly.

## Usage

### Basic Commands

```bash
# Install a Lean toolchain
lemma toolchain install stable
lemma toolchain install nightly
lemma toolchain install v4.0.0

# List toolchains
lemma toolchain list

# Set default toolchain
lemma default stable

# Upgrade installed channel toolchains
lemma toolchain upgrade

# Show active toolchain information
lemma show

# Self-management
lemma self update              # Show package-manager upgrade commands
lemma self uninstall           # Remove Lemma-managed toolchains and ~/.lemma data
```

Use `lemma toolchain ...` for all toolchain-management operations.

## Configuration File

Lemma stores its configuration in `~/.lemma/lemma.toml` (or `$LEMMA_HOME/lemma.toml`).

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
- `LEMMA_HOME` - Lemma home directory (default: `~/.lemma`)
- `LEMMA_RELEASE_URL` - Override the Lean release index URL
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

### Custom Lean Release Sources

Configure a custom Lean release index in `~/.lemma/lemma.toml`:

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

### Command not found errors

If `lemma` is not found, ensure your Python package manager's scripts directory is on `PATH` (`pipx ensurepath` can help for pipx installs).

If `lean`, `lake`, or `leanc` are not found, ensure Lemma's proxy directory is on `PATH`:

```bash
export PATH="$HOME/.lemma/bin:$PATH"
```

## Contributing

Contributions are welcome! Key areas that need work:

1. **Toolchain Installation** - Improve the download and install pipeline
2. **Binary Proxying** - Improve the toolchain binary wrapper system
3. **Testing** - Add comprehensive test coverage
4. **Documentation** - Expand user and developer documentation
5. **Platform Support** - Test on Windows, macOS, Linux

## License

[MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE)
