#!/usr/bin/env bash
set -euo pipefail
dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)
cd "$dir"

for cmd in cargo jq rsync; do
    command -v "$cmd" >/dev/null 2>&1 || {
        echo "error: '$cmd' is required but not installed." >&2
        exit 1
    }
done

cargo fmt

# Every query is a `sqlx` macro, checked against `migrations/` when the committed
# `.sqlx` cache was made. `SQLX_OFFLINE` makes the build read that cache, so
# installing needs no database.
export SQLX_OFFLINE=true
export LEPTOS_TAILWIND_VERSION=v4.3.3

# Two builds, in this order, because the web UI is embedded in the binary.
# `cargo leptos` builds the WASM half and writes it to `target/site/pkg`, but it
# starts the server build before the site is complete, so that pass can embed
# nothing. The second pass compiles only the binary, and its `build.rs` now sees
# a finished site and embeds it. `gym web` says so plainly if this is skipped.
if command -v cargo-leptos >/dev/null 2>&1; then
    cargo leptos build --release
else
    echo "note: cargo-leptos is not installed, so the web UI is not built" >&2
    echo "      (install it with 'cargo install cargo-leptos', or gym web will refuse to start)" >&2
fi
cargo build --release --locked --features ssr

target=$(cargo metadata --format-version 1 --no-deps | jq -r .target_directory)
dest="${CARGO_HOME:-$HOME/.cargo}/bin"
mkdir -p "$dest"

install_bin() {
    local name="$1" tmp="$dest/$1.tmp"
    rm -f "$tmp" # a stale temp would defeat rsync -u
    rsync -ruP "$target/release/$name" "$tmp"
    chmod 755 "$tmp"
    mv -f "$tmp" "$dest/$name" # atomic; avoids ETXTBSY
}

install_bin gym
