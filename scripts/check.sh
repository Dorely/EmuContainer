#!/usr/bin/env sh

set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
cd "$script_dir/.."

cargo fmt --all --check
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo build --locked

echo "EmuContainer formatting, linting, and local build checks passed."
