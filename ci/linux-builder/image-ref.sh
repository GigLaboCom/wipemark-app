#!/usr/bin/env bash
# image-ref.sh — print the builder image's name; the one place it is computed.
#
# What it is for: the owner asked on 2026-10-12 for the Wipemark AppImage to
# be built as MnemoVi's is. MnemoVi's lesson is that "which image built this
# artifact" must have an answer that survives the next rebuild, so the tag is
# derived from the image's inputs, never `latest` (D600).
#
# What it does: hashes the Dockerfile and tools.sha256 and prints
# `wipemark-linux-builder:<first 12 hex of the sha256>`. Changing a pin or a
# package gives a new name rather than mutating an old image.
#
# How to run:   ci/linux-builder/image-ref.sh
#               (WIPEMARK_BUILDER_IMAGE overrides the repository part)
# Needs:        bash, sha256sum.
# Output:       one line, the image reference.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
name="${WIPEMARK_BUILDER_IMAGE:-wipemark-linux-builder}"
tag="$(cat "$here/Dockerfile" "$here/tools.sha256" | sha256sum | cut -c1-12)"
echo "$name:$tag"
