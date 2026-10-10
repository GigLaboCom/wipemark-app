#!/usr/bin/env python3
"""The third host verification's own mutations: protections the round's script did not cover.

What it is for
--------------
Written by the host verifier of the images series' third round (2026-10-05,
`images/series-v3` at **`1a22a54`**; findings: Watchword FILE
`wipemark-task-images-followups-4-2026-10-05`). Besides running
`docs/plan/reports/images-followups-mutate.py` (63 of 63 red), the verifier
broke what it read in the code itself and asked whether a test noticed:
`CHROMA_LEVELS` 6.0 and 7.5, the luma weights R<->B, Cb's coefficients
R<->B, chroma as |dCr| alone, "searched" never said, the German residual
sentence back to "hoechstens", the de/ru plurals, the de decimal point, and
`farthest` as a signed max. Two stayed green and became the fourth round's
L1 (G, the outline sentence's figure) and L2 (B3, the Cb half of the
colour criterion). Its last results are in the task's "What holds".

What it does
------------
**It edits the working tree.** Each mutation replaces one exact text in one
source file, runs `cargo test --locked -p wipemark-pixels -p
wipemark-picture -p wipemark-cli -p wipemark-i18n --no-fail-fast`, prints
RED / GREEN / DID NOT COMPILE with the failing test names, and writes the
file back from memory whatever happens (`finally`). A mutation whose text
is not there exactly once is printed `NOT APPLIED` and skipped — the line
anchors were written against `1a22a54` and may have drifted since. Do not
run it on a tree with unsaved edits to those files; an interrupted run
(`kill -9`) can leave a mutation in place — check `git diff` afterwards.

Usage
-----
    python3 mymut.py            # every mutation
    python3 mymut.py A1 E       # those whose name starts with A1 or E

It changes to the repository root (four directories above this file, or
`$WIPEMARK_REPO`) before anything else, so it can be run from anywhere.
Needs cargo and the pinned toolchain; each mutation is a full test build of
four crates, minutes each.

Output
------
One line per mutation. RED is the protection holding; GREEN is a gap (a
test should exist that goes red); DID NOT COMPILE is no evidence either way.
"""
import os, subprocess, sys, re
REPO = os.environ.get("WIPEMARK_REPO") or os.path.abspath(
    os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "..", ".."))
os.chdir(REPO)
V="crates/wipemark-pixels/src/verify.rs"
M=[
 ("A1 CHROMA 6.0", V, "pub const CHROMA_LEVELS: f32 = 4.0;", "pub const CHROMA_LEVELS: f32 = 6.0;"),
 ("A2 CHROMA 7.5", V, "pub const CHROMA_LEVELS: f32 = 4.0;", "pub const CHROMA_LEVELS: f32 = 7.5;"),
 ("B1 luma weights R<->B", V, "let luma = f64::from(LUMA[0]) * r + f64::from(LUMA[1]) * g + f64::from(LUMA[2]) * b;", "let luma = f64::from(LUMA[2]) * r + f64::from(LUMA[1]) * g + f64::from(LUMA[0]) * b;"),
 ("B2 Cb with R/B coefficients swapped", V, "-0.168_736 * r - 0.331_264 * g + 0.5 * b,", "0.5 * r - 0.331_264 * g - 0.168_736 * b,"),
 ("B3 chroma = |dCr| only", V, "outline.chroma = step[4].hypot(step[5]) as f32;", "outline.chroma = step[5].abs() as f32;"),
 ("C L1 CLI never says searched", "apps/wipemark-cli/src/image.rs", "                if restored.searched {\n", "                if false {\n"),
 ("D de residual back to hoechstens", "crates/wipemark-i18n/i18n/de/wipemark.ftl", "liegt die wiederhergestellte Markierung im Mittel { $levels } Stufen vom Bild um sie herum, im am stärksten abweichenden Farbkanal.", "liegt die wiederhergestellte Markierung höchstens { $levels } Stufen vom Bild um sie herum."),
 ("E1 de plural one -> plural", "crates/wipemark-i18n/i18n/de/wipemark.ftl", "        [one] Beim Umkehren der Überblendung fiel { $clamped } Wert aus dem Wertebereich und wurde begrenzt, daher ist die Wiederherstellung nicht exakt.\n", "        [one] Beim Umkehren der Überblendung fielen { $clamped } Werte aus dem Wertebereich und wurden begrenzt, daher ist die Wiederherstellung nicht exakt.\n"),
 ("E2 de decimal point", "crates/wipemark-i18n/src/lib.rs", 'const DECIMAL_COMMA: [&str; 2] = ["de", "ru"];', 'const DECIMAL_COMMA: [&str; 2] = ["xx", "ru"];'),
 ("E3 ru [few] dropped", "crates/wipemark-i18n/i18n/ru/wipemark.ftl", "        [few] При обращении смешивания { $clamped } значения вышли за пределы диапазона и были обрезаны, поэтому восстановление не точное.\n", ""),
 ("F farthest signed max", "apps/wipemark-cli/src/image.rs", "fold(0f32, |m, s| m.max(s.abs()))", "fold(0f32, |m, s| m.max(*s))"),
 ("G outline sentence uses luma", "apps/wipemark-cli/src/image.rs", '                            "levels" => fixed(farthest(restored), 1),', '                            "levels" => fixed(restored.step.abs(), 1),'),
]
PK=["-p","wipemark-pixels","-p","wipemark-picture","-p","wipemark-cli","-p","wipemark-i18n"]
want=sys.argv[1:]
for name,path,old,new in M:
    if want and not any(name.startswith(w) for w in want): continue
    src=open(path,encoding="utf-8").read()
    if src.count(old)!=1: print(name,"NOT APPLIED"); continue
    try:
        open(path,"w",encoding="utf-8").write(src.replace(old,new))
        r=subprocess.run(["cargo","test","--locked"]+PK+["--no-fail-fast"],capture_output=True,text=True)
        failed=sorted(set(re.findall(r"^    (\S+)$", r.stdout, re.M)))
        comp="could not compile" in r.stderr
        print(f"{name}: {'DID NOT COMPILE' if comp else ('RED' if r.returncode else 'GREEN')} failed={failed}", flush=True)
    finally:
        open(path,"w",encoding="utf-8").write(src)
