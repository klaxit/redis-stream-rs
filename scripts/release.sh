#!/usr/bin/env bash
set -exu

if [ -z "$(git status --untracked-files=no --porcelain)" ]; then
  cargo install cargo-bump
  npx standard-version -a --scripts.postbump "./scripts/bump-cargo.sh"
else
  echo "Uncommitted changes detected, please commit before release."
  exit 1
fi
