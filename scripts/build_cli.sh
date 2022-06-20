#! /bin/bash
echo "[INFO] building cli in release mode"
RUSTFLAGS="-C target-cpu=native" cargo build --release
cp target/release/cli template-cli