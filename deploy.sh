#!/bin/bash
set -euo pipefail
dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)
cd "$dir"

ndk=/opt/android-ndk
toolchain="$ndk/toolchains/llvm/prebuilt/linux-x86_64/bin"
api=35
clang="$toolchain/aarch64-linux-android${api}-clang"
ar="$toolchain/llvm-ar"

need() {
    command -v "$1" >/dev/null 2>&1 || {
        echo "error: '$1' is required but not installed." >&2
        echo "       $2" >&2
        exit 1
    }
}
need cargo "Install Rust from https://rustup.rs."
need jq "Install it with the package manager; install.sh expects it too."
need cargo-leptos "Install it with 'cargo install cargo-leptos' — the web UI cannot be built without it."

if command -v rustup >/dev/null 2>&1; then
    installed=$(rustup target list --installed)
    for target in wasm32-unknown-unknown aarch64-linux-android; do
        grep -qx -- "$target" <<<"$installed" || {
            echo "error: the Rust target '$target' is not installed." >&2
            echo "       Add it with 'rustup target add $target'." >&2
            exit 1
        }
    done
fi

for file in "$clang" "$ar"; do
    [ -x "$file" ] || {
        echo "error: the Android NDK toolchain is missing: no $file" >&2
        echo "       deploy.sh cross-compiles for aarch64-linux-android with the NDK at $ndk (API $api)." >&2
        exit 1
    }
done

export SQLX_OFFLINE=true

cargo leptos build --release

CC="$clang" AR="$ar" cargo build \
    --target aarch64-linux-android \
    --release --locked --features ssr \
    --config "target.aarch64-linux-android.linker = \"$clang\""

target=$(cargo metadata --format-version 1 --no-deps | jq -r .target_directory)
artifact="$target/aarch64-linux-android/release/gym"
for asset in gym.js gym.css; do
    marker=$(head -n 1 "target/site/pkg/$asset" | cut -c1-40)
    grep -aqF -- "$marker" "$artifact" || {
        echo "error: $artifact carries no $asset, so it would run without the web UI." >&2
        exit 1
    }
done

echo "deployed $artifact, with the web UI embedded"
