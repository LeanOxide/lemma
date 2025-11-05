#!/bin/sh
# Lemma installer script

set -e

LEMMA_HOME="${LEMMA_HOME:-$HOME/.lemma}"
LEMMA_BIN_DIR="$LEMMA_HOME/bin"

# Determine OS and architecture
get_platform() {
    local os
    local arch

    os="$(uname -s)"
    arch="$(uname -m)"

    case "$os" in
        Linux)
            case "$arch" in
                x86_64)
                    # Check if we're using musl or glibc
                    if ldd --version 2>&1 | grep -q musl; then
                        echo "x86_64-unknown-linux-musl"
                    else
                        echo "x86_64-unknown-linux-gnu"
                    fi
                    ;;
                *)
                    echo "Unsupported architecture: $arch" >&2
                    exit 1
                    ;;
            esac
            ;;
        Darwin)
            case "$arch" in
                x86_64)
                    echo "x86_64-apple-darwin"
                    ;;
                arm64)
                    echo "aarch64-apple-darwin"
                    ;;
                *)
                    echo "Unsupported architecture: $arch" >&2
                    exit 1
                    ;;
            esac
            ;;
        *)
            echo "Unsupported operating system: $os" >&2
            exit 1
            ;;
    esac
}

# Fetch latest version from manifest
get_latest_version() {
    local manifest_url="$1"
    local temp_manifest

    temp_manifest="$(mktemp)"

    if command -v curl >/dev/null 2>&1; then
        curl -sSfL "$manifest_url" -o "$temp_manifest"
    elif command -v wget >/dev/null 2>&1; then
        wget -q "$manifest_url" -O "$temp_manifest"
    else
        echo "Error: curl or wget is required" >&2
        rm -f "$temp_manifest"
        exit 1
    fi

    # Parse version from TOML
    version=$(grep '^version = ' "$temp_manifest" | cut -d'"' -f2)
    rm -f "$temp_manifest"

    if [ -z "$version" ]; then
        echo "Error: Failed to parse version from manifest" >&2
        exit 1
    fi

    echo "$version"
}

main() {
    local platform
    local base_url="https://lemma.puqing.work"
    local manifest_url="$base_url/manifests/stable.toml"
    local version
    local download_url
    local temp_dir
    local archive_name

    echo "=> Installing lemma..."
    echo

    # Detect platform
    platform="$(get_platform)"
    echo "   Platform: $platform"

    # Fetch latest version
    echo "   Checking latest version..."
    version="$(get_latest_version "$manifest_url")"
    echo "   Version: $version"

    # Construct download URL
    archive_name="lemma-${platform}.tar.gz"
    download_url="$base_url/releases/v${version}/${archive_name}"
    echo "   Download URL: $download_url"
    echo

    # Create temp directory
    temp_dir="$(mktemp -d)"
    trap "rm -rf '$temp_dir'" EXIT

    # Download archive
    echo "=> Downloading lemma..."
    if command -v curl >/dev/null 2>&1; then
        curl -sSfL "$download_url" -o "$temp_dir/$archive_name"
    elif command -v wget >/dev/null 2>&1; then
        wget -q "$download_url" -O "$temp_dir/$archive_name"
    else
        echo "Error: curl or wget is required" >&2
        exit 1
    fi

    # Extract archive
    echo "=> Extracting..."
    mkdir -p "$LEMMA_BIN_DIR"
    tar -xzf "$temp_dir/$archive_name" -C "$temp_dir"

    # Install binary
    echo "=> Installing to $LEMMA_BIN_DIR..."
    mv "$temp_dir/lemma" "$LEMMA_BIN_DIR/lemma"
    chmod +x "$LEMMA_BIN_DIR/lemma"

    # Create proxy binaries
    echo "=> Creating proxy binaries..."
    for binary in lean lake leanc; do
        ln -sf lemma "$LEMMA_BIN_DIR/$binary"
    done

    echo
    echo "✓ lemma installed successfully!"
    echo
    echo "To get started, add lemma to your PATH:"
    echo
    echo "  export PATH=\"\$HOME/.lemma/bin:\$PATH\""
    echo
    echo "Add this to your shell profile (~/.bashrc, ~/.zshrc, etc.) to make it permanent."
    echo
    echo "Then install a Lean toolchain:"
    echo
    echo "  lemma toolchain install stable"
    echo
}

main "$@"
