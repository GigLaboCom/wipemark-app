#!/usr/bin/env bash
# dmgbuild-venv.sh — make the Python venv that dmg.sh runs dmgbuild from (D816).
#
# What it is for: dmg.sh makes the .dmg with dmgbuild, which writes the
# window's .DS_Store itself, with no Finder and no AppleScript. dmgbuild is
# a Python package; it is installed into a venv of its own, never into a
# system Python, from dmgbuild-requirements.txt — three wheels pinned by
# version and sha256. The release lane's `macos` job runs this before
# bundle.sh; a person runs it once into a scratch directory.
#
# What it does: checks that the Python is 3.10 or later (dmgbuild 1.6.7
# needs it), makes a venv at <dir> (an existing one is replaced), and
# installs the three wheels with --require-hashes --no-deps
# --only-binary :all:, so nothing else is resolved and no sdist's build code
# runs. Prints the line to export.
#
# How to run:   scripts/release/macos/dmgbuild-venv.sh <dir> [python]
#               export WIPEMARK_DMGBUILD_VENV=<dir>
#               The Python is the second argument, else `python3` on PATH.
# Needs:        macOS, a Python 3.10 or later with venv, the network (PyPI).
# Output:       the venv at <dir>; the pinned versions, and the export line.
#               Exit 1 on an old Python or a failed install.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
dir="${1:?usage: dmgbuild-venv.sh <dir> [python]}"
python="${2:-python3}"

"$python" -c 'import sys; sys.exit(sys.version_info < (3, 10))' || {
  echo "ERROR: $python is $("$python" -V 2>&1); dmgbuild 1.6.7 needs 3.10 or later — name one: dmgbuild-venv.sh <dir> /path/to/python3.12" >&2
  exit 1
}
rm -rf "$dir"
"$python" -m venv "$dir"
"$dir/bin/python" -I -m pip install --quiet --disable-pip-version-check --no-input \
  --require-hashes --no-deps --only-binary :all: -r "$here/dmgbuild-requirements.txt"
"$dir/bin/python" -I -c '
from importlib.metadata import version
import sys
print("    " + sys.version.split()[0] + ":", ", ".join(f"{n} {version(n)}" for n in ("dmgbuild", "ds_store", "mac_alias")))'
echo "export WIPEMARK_DMGBUILD_VENV=$dir"
