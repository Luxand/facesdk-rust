#!/bin/bash
#
# Build wrapper for facesdk-rust.
#
# Works around NASM not supporting Unicode paths on Windows.
# If the default CARGO_HOME (inside the user profile) contains non-ASCII
# characters, we relocate it to C:/cargo-home so that NASM can assemble
# the mozjpeg SIMD sources without path-encoding errors.
#
# Usage:
#   ./build.sh                     # debug build
#   ./build.sh --release           # release build
#   ./build.sh --bin portrait      # build specific binary
#   ./build.sh <any cargo args>    # pass-through to cargo build

set -e

# Detect if CARGO_HOME contains non-ASCII characters
needs_ascii_cargo_home() {
    local home="${CARGO_HOME:-$HOME/.cargo}"
    # Check if the path contains only ASCII printable characters
    if echo "$home" | LC_ALL=C grep -qP '[^\x20-\x7E]'; then
        return 0  # true: needs relocation
    fi
    return 1  # false: path is ASCII-safe
}

if needs_ascii_cargo_home; then
    export CARGO_HOME="C:/cargo-home"
    echo "[build.sh] CARGO_HOME relocated to $CARGO_HOME (NASM Unicode workaround)"
fi

exec cargo build "$@"
