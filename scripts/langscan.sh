#!/usr/bin/env bash
set -euo pipefail

# Thin wrapper for `xtask langscan`.
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"
exec cargo run -p xtask --quiet -- langscan "$@"
