#!/bin/sh

PROJ_ROOT="$(dirname "$0")/.."
VERSION=$(cat "$PROJ_ROOT"/.VERSION)

find process -type f | sed 's@process/@@' | while read file; do
    rm -f "$file"
    sed "s/@VERSION@/$VERSION/" "process/$file" > "$file"
    chmod a-w "$file"
    git add "$file"
done
