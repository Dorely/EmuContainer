#!/usr/bin/env sh

set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
cd "$script_dir/.."

if ! command -v rustup >/dev/null 2>&1; then
    echo "rustup is required. Install it from https://rustup.rs/." >&2
    exit 1
fi

rustup toolchain install 1.97.1 --profile minimal --component rustfmt --component clippy
cargo fetch --locked

echo "EmuContainer development dependencies are ready."
