#!/bin/bash
set -euo pipefail
dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)

(
    echo -e "const exercise = "
    gym prog
    echo -e "const weight = "
    gym get-weight
)> "$dir/s/data.js"
