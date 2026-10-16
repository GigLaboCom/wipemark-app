#!/usr/bin/env python3
"""The fifth host verification's own mutations, over the round's mutation runner.

What it is for
--------------
Written by the host verifier of the images series' fifth round (2026-10-05),
which verified `images/series-v3` at `4897d0b` (V1-V5: tests and docs only;
the round's report is `docs/plan/reports/images-followups-5-2026-10-05.md`)
for the coordinator of `GigLaboCom/wipemark-app`.
Besides `docs/plan/reports/images-followups-mutate.py` (its 5V1-5V5 all
red), the verifier wrote its own variants of each new protection, to check
the tests guard the behaviour and not only the one edit the implementer
chose: the texture sentence's "around" figure taken from `texture` (own-V1b),
a lossy WebP held as lossless by a different road (own-V2b),
`TEXTURE_LEVELS` 5.3 — inside the 5 % margin over the roughest q98, so
green was expected (own-V3-5.3) — and the de decimal comma dropped by
another edit (own-V5-de). Its results: V1b, V2b, V5-de red; V3-5.3 green,
as expected.

What it does
------------
**It edits the working tree.** It loads
`docs/plan/reports/images-followups-mutate.py` for its `mutate` (apply one
exact text, run the body, restore from memory whatever happens) and `run`
(`cargo test --locked … -- <name>`), applies each of its own mutations
alone, and runs the named tests. A text not there exactly once is `NOT
APPLIED` — the anchors were written against **`4897d0b`** and may have
drifted. Do not run it over unsaved edits to those files; after an
interrupted run, check `git diff`.

Usage
-----
    python3 mymut.py                    # all four
    python3 mymut.py own-V1b own-V2b    # by id

It changes to the repository root (four directories above this file, or
`$WIPEMARK_REPO`) first; the runner's paths are relative to it. Needs cargo
and the standard library.

Output
------
One line per mutation and test: `red` is the protection holding, `GREEN` a
gap, `NOCOMPILE` no evidence, then the cargo command.
"""
import importlib.util, os, sys
os.chdir(os.environ.get("WIPEMARK_REPO") or os.path.abspath(
    os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "..", "..")))
s = importlib.util.spec_from_file_location('m', 'docs/plan/reports/images-followups-mutate.py')
m = importlib.util.module_from_spec(s); s.loader.exec_module(m)
CLI, PIC, VISIBLE = m.CLI, m.PIC, m.VISIBLE
MY = [
  ("own-V1b", "apps/wipemark-cli/src/image.rs",
   '"around" => fixed(restored.texture_around, 1),', '"around" => fixed(restored.texture, 1),',
   [(VISIBLE, "a_texture_left_on_a_jpeg_is_said_and_exits_three")]),
  ("own-V2b", "crates/wipemark-picture/src/lib.rs",
   "    let examine = ExamineOptions {\n        source: decoded.fidelity,",
   "    let examine = ExamineOptions {\n        source: if matches!(decoded.source, Source::WebP { .. }) { Fidelity::Lossless } else { decoded.fidelity },",
   [(VISIBLE, "a_lossy_webp_is_held_as_lossy")]),
  ("own-V3-5.3", "crates/wipemark-pixels/src/verify.rs",
   "pub const TEXTURE_LEVELS: f32 = 5.5;", "pub const TEXTURE_LEVELS: f32 = 5.3;",
   [(PIC + ["--test", "real"], "texture"), (VISIBLE, "texture"), (VISIBLE, "a_lossy_webp_is_held_as_lossy")]),
  ("own-V5-de", "crates/wipemark-i18n/src/lib.rs",
   'const DECIMAL_COMMA: [&str; 2] = ["de", "ru"];', 'const DECIMAL_COMMA: [&str; 2] = ["ru", "ru"];',
   [(CLI + ["--test", "cli"], "models_sizes_are_spelled_in_the_languages_decimals")]),
]
want = set(sys.argv[1:])
for mid, path, old, new, tests in MY:
    if want and mid not in want: continue
    def body():
        return [(n,) + m.run(a, n) for a, n in tests]
    r = m.mutate(path, old, new, body)
    if r is None: print(mid, "NOT APPLIED"); continue
    for name, red, compiled, cmd in r:
        print(mid, "red" if red else ("GREEN" if compiled else "NOCOMPILE"), cmd, flush=True)
