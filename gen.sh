#!/bin/bash
set -euo pipefail
dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)

temp="$dir/test.sqlite"
echo "" >"$temp"
sqlite3 "$temp" <"$dir/up.sql"
