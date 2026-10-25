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
# Every query is a `sqlx` macro, checked against `migrations/` when the
# committed `.sqlx` cache was made. `SQLX_OFFLINE` makes the build read that
# cache, so installing needs no database.
SQLX_OFFLINE=true cargo build --release --locked

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
