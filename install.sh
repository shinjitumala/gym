#!/bin/bash
set -euxo pipefail
dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)

cargo install --path "$dir"
