#!/bin/bash

rustup target add x86_64-apple-darwin
rustup target add aarch64-apple-darwin
rustup target add x86_64-unknown-linux-gnu
rustup target add aarch64-unknown-linux-gnu
rustup target add x86_64-unknown-linux-musl
rustup target add aarch64-unknown-linux-musl
rustup target add aarch64-pc-windows-msvc
rustup target add x86_64-pc-windows-msvc

output_dir="dist"
rm -rf "$output_dir"
py_versions=("3.9" "3.10" "3.11" "3.12" "3.13" "3.14")
targets_mac=("x86_64-apple-darwin" "aarch64-apple-darwin")
targets_linux=("x86_64-unknown-linux-gnu" "aarch64-unknown-linux-gnu" "x86_64-unknown-linux-musl" "aarch64-unknown-linux-musl")
targets_windows=("x86_64-pc-windows-msvc" "aarch64-pc-windows-msvc")
source .venv/bin/activate
for ver in "${py_versions[@]}"; do
    uv python pin "$ver"
    uv sync
    for target in "${targets_mac[@]}"; do
        maturin build --release --target "$target" --strip --zig --out "$output_dir" -i "$ver"
    done
    for target in "${targets_linux[@]}"; do
        maturin build --release --target "$target" --strip --zig --out "$output_dir" -i "$ver"
    done
    for target in "${targets_windows[@]}"; do
        maturin build --release --target "$target" --strip --zig --out "$output_dir" -i "$ver"
    done
done