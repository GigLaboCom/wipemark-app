#!/usr/bin/env bash
# Retired on 2026-10-07 by the GPUI bump (Watchword
# `wipemark-task-gpui-bump-2026-10-07`, G2; the owner, through the
# coordinator). It rewrote the vendored gpui-component's rev-less zed git
# dependencies to the rev the workspace pinned, so the graph held one
# `gpui`. The workspace and the component (upstream gpui-kit) now both
# take GPUI from the `gpui-pre` snapshots on crates.io at the same exact
# version, so there is nothing left to rewrite; `scripts/check-gpui-pin.sh`
# is the gate that the two agree.
#
# What it does: prints one line saying so and exits 0, so the "first
# command after any clone" in CLAUDE.md and CONTRIBUTING.md, and old
# checkouts' habits, keep working until those lines are removed.
#
# How to run: scripts/pin-gpui-component.sh — it changes nothing.
# What it needs: nothing.
# What its output means: the line below; there is no failure.
echo "pin-gpui-component.sh: retired — GPUI is gpui-pre from crates.io at the component's exact version; scripts/check-gpui-pin.sh checks it (docs/architecture/gpui-pin.md)"
