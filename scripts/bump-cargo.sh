#!/usr/bin/env bash
set -exu

VERSION="$(cat ./VERSION)"

# Update Cargo.toml
cargo bump "${VERSION}"

# Update README.md
sed -i.bak -E \
  -e "s|redis-stream = \"[0-9.]+\"|redis-stream = \"${VERSION}\"|g" \
  "README.md"
rm "README.md.bak"

# Stage changes
git add Cargo.toml README.md
