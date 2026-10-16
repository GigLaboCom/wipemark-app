#!/usr/bin/env bash
# Check that a source build of llama.cpp for an MSVC target handed cmake
# CMake's own Release flags (E2-5 I7).
#
# Usage: scripts/check-msvc-flags.sh <wipemark-llama-sys build-script stderr>
#        (target/debug/build/wipemark-llama-sys-*/stderr of a source build)
#
# The `cmake` crate prints every command it runs ("running: ...") to the
# build script's stderr. Under
# the Visual Studio generator it sets CMAKE_<LANG>_FLAGS and
# CMAKE_<LANG>_FLAGS_RELEASE to the cc crate's `-nologo -MD -Brepro`
# unless they are defined — which drops /O2, /Ob2, /DNDEBUG and /EHsc: an
# unoptimised ggml with assertions on and no C++ exception model.
# wipemark-llama-sys's build.rs defines them (`msvc_release_flags`). This
# reads both configure lines (ggml, then llama) back and fails unless each
# one carries
#   -DCMAKE_C_FLAGS_RELEASE=/O2 /Ob2 /DNDEBUG
#   -DCMAKE_CXX_FLAGS_RELEASE=/O2 /Ob2 /DNDEBUG
#   -DCMAKE_CXX_FLAGS=... /EHsc ...
# and no other value for the two Release variables.
set -euo pipefail

out="${1:?usage: $0 <build-script stderr file>}"
[ -f "$out" ] || { echo "FAIL: $out is not a file" >&2; exit 1; }

# Configure lines: `running: "cmake" "<source dir>" ...`, not `--build`.
mapfile -t configures < <(grep '^running: ' "$out" | grep -i 'cmake' | grep -v '"--build"' || true)
if [ "${#configures[@]}" -ne 2 ]; then
  echo "FAIL: expected 2 cmake configure lines (ggml, llama), found ${#configures[@]} in $out" >&2
  exit 1
fi

# Every quoted argument of one line, one per line.
args_of() { grep -o '"[^"]*"' <<<"$1" | sed 's/^"//; s/"$//'; }

status=0
for i in 0 1; do
  stage=$([ "$i" -eq 0 ] && echo ggml || echo llama)
  args=$(args_of "${configures[$i]}")
  for lang in C CXX; do
    mapfile -t release < <(grep "^-DCMAKE_${lang}_FLAGS_RELEASE=" <<<"$args" || true)
    if [ "${#release[@]}" -eq 0 ]; then
      echo "FAIL ($stage): CMAKE_${lang}_FLAGS_RELEASE is not defined" >&2; status=1
    fi
    for r in "${release[@]}"; do
      if [ "$r" != "-DCMAKE_${lang}_FLAGS_RELEASE=/O2 /Ob2 /DNDEBUG" ]; then
        echo "FAIL ($stage): $r" >&2; status=1
      else
        echo "ok   ($stage): $r"
      fi
    done
  done
  cxx=$(grep '^-DCMAKE_CXX_FLAGS=' <<<"$args" || true)
  if grep -q '/EHsc' <<<"$cxx"; then
    echo "ok   ($stage): $cxx"
  else
    echo "FAIL ($stage): no /EHsc in ${cxx:-CMAKE_CXX_FLAGS (not defined)}" >&2; status=1
  fi
done
exit "$status"
