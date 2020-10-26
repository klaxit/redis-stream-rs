#!/usr/bin/env bash
set -exu

# Run ./scripts/release.sh first and set your ~/.cargo/credentials

git push --follow-tags origin master
cargo login
cargo publish
