#!/bin/sh
# Designer fork: build the web editor into frontend/dist.
#
# What `cargo run build` does, by hand, with two differences: the branded-asset
# download is never run (branding/ is committed), and the wrapper is built
# without the `shader-nodes` feature, which does not compile at this commit.
#
# Needs: Rust 1.98 with the wasm32-unknown-unknown target, wasm-bindgen-cli
# 0.2.121, wasm-opt (Binaryen 130), cargo-about 0.9.0, Node 22, and `npm ci`
# in frontend/. WASM_BINDGEN and WASM_OPT name the two tools if they are not
# on PATH.
set -eu
cd "$(dirname "$0")"
wasm_bindgen="${WASM_BINDGEN:-wasm-bindgen}"
wasm_opt="${WASM_OPT:-wasm-opt}"

cargo build --lib -p graphite-wasm-wrapper --target wasm32-unknown-unknown --release \
  --no-default-features --features gpu
"$wasm_bindgen" --target web --out-name graphite_wasm_wrapper --out-dir frontend/wrapper/pkg \
  target/wasm32-unknown-unknown/release/graphite_wasm_wrapper.wasm --no-demangle
"$wasm_opt" -O3 -g frontend/wrapper/pkg/graphite_wasm_wrapper_bg.wasm \
  -o frontend/wrapper/pkg/graphite_wasm_wrapper_bg.wasm
(cd frontend && node node_modules/vite/bin/vite.js build)
echo "built frontend/dist"
