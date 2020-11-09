#!/usr/bin/env bash
set -exu

# Run ./scripts/release.sh first and set your cargo credentials with cargo login

git push --follow-tags origin master
cargo publish
