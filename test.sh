#!/bin/bash

set -e

cd "$(dirname "$0")"

echo "=== Running cargo test ==="
cargo test

echo ""
echo "=== Running cargo clippy ==="
cargo clippy -- -D warnings

echo ""
echo "=== Running cargo fmt check ==="
cargo fmt --check