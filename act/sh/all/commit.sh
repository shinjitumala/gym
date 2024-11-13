#!/bin/bash
set -euox pipefail

# start metadata
# type interactive
dir=$1
# end metadata

changes=$(git status --porcelain=v1 2>/dev/null | wc -l)
if ((changes == 0)); then
    echo "There is nothing to commit."
else
    (cd "$dir" && git add .)
    git -C "$dir" commit -m "upd."
    mapfile -t a < <(git -C "$dir" remote -v | awk '{ print $1 }' | sort | uniq)
    for r in "${a[@]}"; do
        echo "Pushing to $r..."
        git -C "$dir" push "$r" --all
        echo "Done."
    done
fi
