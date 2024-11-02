#!/bin/bash
set -euo pipefail

# start metadata
# type text
dir=$1
# end metadata

cd "$dir" && (
git -C "$dir" add .
git -C "$dir" commit -m "upd."
mapfile -t a < <(git -C "$dir" remote -v | awk '{ print $1 }' | sort | uniq)
for r in "${a[@]}"; do
    echo "$r..."
    git -C "$dir" push "$r" --all
done
)
