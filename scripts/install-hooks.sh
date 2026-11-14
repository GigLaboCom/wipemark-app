#!/usr/bin/env bash
#
# Configure git to use the project's .githooks/ directory for hooks.
# Run once after cloning the repo: `core.hooksPath` is local git config,
# so a clone does not carry it, and without it there is no hook at all.
#
# What it turns on: .githooks/pre-commit, which scans the staged changes
# for secrets with gitleaks and the repository's .gitleaks.toml. CI's
# `secrets` job (.github/workflows/gate.yml) scans the whole history with
# the same config; a release attaches nothing while that scan fails
# (.github/workflows/release.yml). The setup is Watchword
# `moat-atlas-gitleaks-hooks-2026-08-20`, ported here 2026-10-24.
#
# Usage:
#   scripts/install-hooks.sh
#

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

echo "Configuring git hooks path → .githooks/"
git -C "$REPO_ROOT" config core.hooksPath .githooks

echo "Done. Pre-commit secret scanning is now active."

# Check if gitleaks is available
if ! command -v gitleaks &>/dev/null; then
  echo ""
  echo "NOTE: gitleaks is not installed yet. Install it for secret detection:"
  echo "  brew install gitleaks          # macOS"
  echo "  scoop install gitleaks         # Windows"
  echo "  https://github.com/gitleaks/gitleaks#installing"
fi
