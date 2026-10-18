#!/bin/sh
# Builds the Barcode plug-in. Needs the wasm32-unknown-unknown target:
#   rustup target add wasm32-unknown-unknown
set -eu
cd "$(dirname "$0")"
cargo build --release --target wasm32-unknown-unknown
out=target/wasm32-unknown-unknown/release/vectorcraft_plugin_barcode.wasm
echo "built: $out ($(wc -c < "$out") bytes)"
echo "install in VectorCraft: File > Open that .wasm file, or plugin.install {\"path\": \"$(cd "$(dirname "$out")" && pwd)/$(basename "$out")\"} on the control port."
