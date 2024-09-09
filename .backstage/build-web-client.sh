#!/usr/bin/env bash
set -eu
script_path=$( cd "$(dirname "${BASH_SOURCE[0]}")" ; pwd -P )
cd "$script_path/.."

CRATE_NAME="truncate_client"
FEATURES=""

# Clear output from old stuff:
rm -f "web_client/src/static/${CRATE_NAME}_bg.wasm"

echo "Building rust…"
BUILD=release

(cd $CRATE_NAME &&
  cargo build \
    --release \
    --lib \
    --target wasm32-unknown-unknown \
    --no-default-features #\
    # --features ${FEATURES}
)

# Get the output directory (in the workspace it is in another location)
# TARGET=`cargo metadata --format-version=1 | jq --raw-output .target_directory`
TARGET="target"

echo "Generating JS bindings for wasm…"
TARGET_NAME="${CRATE_NAME}.wasm"
WASM_PATH="${TARGET}/wasm32-unknown-unknown/$BUILD/$TARGET_NAME"
wasm-bindgen "${WASM_PATH}" --out-dir web_client/src/static --no-modules --no-typescript

# if this fails with "error: cannot import from modules (`env`) with `--no-modules`", you can use:
# wasm2wat target/wasm32-unknown-unknown/release/egui_demo_app.wasm | rg env
# wasm2wat target/wasm32-unknown-unknown/release/egui_demo_app.wasm | rg "call .now\b" -B 20 # What calls `$now` (often a culprit)

# to get wasm-strip:  apt/brew/dnf install wabt
# wasm-strip docs/${CRATE_NAME}_bg.wasm



if [ -n "${TRUNC_OPT+x}" ]; then
  if [[ "${TRUNC_OPT}" = true ]]; then
    echo "Optimizing wasm…"
    # to get wasm-opt:  apt/brew/dnf install binaryen
    wasm-opt "web_client/src/static/${CRATE_NAME}_bg.wasm" -O4 --fast-math -o "web_client/src/static/${CRATE_NAME}_bg.wasm" # add -g to get debug symbols
  fi
fi

echo "Finished web_client/src/static/${CRATE_NAME}_bg.wasm"

(cd web_client/src && npm i && npm run build)

echo "Finished building Eleventy site"

ls -lh web_client/src/static/${CRATE_NAME}_bg.wasm
