#!/usr/bin/env bash
set -exu

cargo bump "$(cat ./VERSION)"
git add Cargo.toml
