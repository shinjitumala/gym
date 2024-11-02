#!/bin/bash
set -euo pipefail

# start metadata
# type text
dir=$1
# end metadata

git -C "$dir" ls-files --others --exclude-standard
