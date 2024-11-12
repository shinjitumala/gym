#!/bin/bash
set -euox pipefail

# start metadata
# type text
dir=$1
# end metadata

git -C "$dir" pull
