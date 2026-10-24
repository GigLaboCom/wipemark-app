#!/usr/bin/env bash
#
# Put the developer documentation at dev/: plans, decisions, reports,
# architecture notes, the working notes CLAUDE.md imports, the host-bound
# verification scripts and the private gitleaks rules the pre-commit hook
# reads. It is the private GigLaboCom/wipemark-dev, its own repository;
# this one ignores dev/ (.gitignore).
#
# In the main working copy it clones the repository once. In a linked
# worktree (an agent's) it makes dev a link to the main working copy's dev/,
# so every worktree reads the same notes and a change to them is made, and
# committed, in one place. Run again, it changes nothing.
#
# Usage:
#   scripts/dev-checkout.sh
#   WIPEMARK_DEV_URL=<url> scripts/dev-checkout.sh   # another remote
#

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
DEV="$REPO_ROOT/dev"
URL="${WIPEMARK_DEV_URL:-git@github.com:GigLaboCom/wipemark-dev.git}"

if [ -e "$DEV" ]; then
  echo "dev/ is already there: $(cd "$DEV" && pwd -P)"
  exit 0
fi

# The first worktree git lists is the main working copy.
MAIN="$(git -C "$REPO_ROOT" worktree list --porcelain | awk '/^worktree /{print substr($0, 10); exit}')"
if [ "$MAIN" != "$REPO_ROOT" ] && [ -d "$MAIN/dev/.git" ]; then
  ln -s "$MAIN/dev" "$DEV"
  echo "dev → $MAIN/dev"
  exit 0
fi

git clone "$URL" "$DEV"
# Commits there carry the same author as commits here, and are scanned for
# secrets by its own hook (dev/.githooks, dev/.gitleaks.toml).
name="$(git -C "$REPO_ROOT" config user.name || true)"
if [ -n "$name" ]; then
  git -C "$DEV" config user.name "$name"
fi
git -C "$DEV" config core.hooksPath .githooks
echo "dev/ cloned from $URL"
