#!/bin/sh
# Repository checks run by CI. Run from the repository root with stable Rust and
# a SQLite runtime at least 3.51.3. They are the README's build checks plus
# default-feature tests and the offline documentation check.
set -eu

cargo check --workspace --all-targets --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --locked
cargo test --workspace --all-features --locked
python3 tools/check_docs.py
