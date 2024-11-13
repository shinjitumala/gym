#!/bin/bash
set -euox pipefail

# start metadata
# type interactive
dir=$1
# end metadata

git -C "$dir" pull
