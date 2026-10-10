#!/usr/bin/env python3
"""R10's two triggers: is FDnCNN run (Gemini), is LaMa run (Grok)?

What it is for
--------------
Step E12-R10 of the E12-R series (`docs/plan/E12-R10-model-evaluation.md`
§2.1 and §3.1), filed 2026-10-08 by the coordinator from the owner's spec
`wipemark-recon-spec-2026-10-08` (`07-model-evaluation.md`; the owner's
S10: FDnCNN conditionally after R*, LaMa for Gemini not evaluated, LaMa for
Grok if holes cover more than 1 %). Written 2026-10-09, ahead of the run,
at the owner's request that all code work be finished first. "Not needed"
is a result that closes the question; this script states it from numbers.

What it does
------------
`fdncnn` (§2.1) reads one or more R1 run folders (`scripts/regress.py
baseline|run`: `index.json` and one `<id>.json` per file — the CLI's
`clean --json`, with the accepted R6/R8 in the CLI) and, per class and
variant of R1's JPEG classes (`recon-jpeg-444`, `recon-jpeg-420` by
default) whose quality is **q85 to q95**:

1. counts the **files** — not the marks — with at least one restoration
   that has `texture_left`, or `outline_left` **by chroma** (see below);
2. divides by every file of that variant (a refused or unfound mark is a
   file that does not count, and is shown apart);
3. compares the largest share with `SHARE` (20 %, `[tunable]`);
4. with `--ab SCORE.json` (from `ab.py score`, the `remainder` question —
   Q-R7, 20 pairs), compares the share of R* items seen with a visible
   remainder with `AB_SHARE` (30 %, `[tunable]`);
5. writes the numbers and one verdict line: **"FDnCNN not evaluated"** when
   neither holds (which closes it for Gemini), **"FDnCNN to be run"**
   otherwise, with the variant or the A/B that tripped it.

"By chroma": the JSON carries `outline_left`, `outline` (the share),
`step` and `chroma`, but not the two spreads `verify.rs` (`Outline::left`)
compares `step` and `chroma` with. A restoration is counted as left by
chroma when `outline_left` holds and `chroma` > `CHROMA_LEVELS` (4.0) —
the condition the chroma term needs, whatever the spread; when the share
or the luma step could have fired too, the file is still counted (an upper
bound: it says *could*, and the table shows how many such files there
are). Files with `smoothed` (D496) or with an outline left by the share or
the step alone are shown apart and not counted: FDnCNN removes noise; it
does not add texture or move a band.

The run is also checked for what it says about the CLI: the share of 4:2:0
restorations carrying `planar` (R6, D471) and of lossy restorations
carrying `interval` (R8). None of either is a warning in the report — the
trigger is to be measured **with the accepted R6/R8 in the CLI** — any
release CLI built at or after the owner's decisions of 2026-10-10 (the
planes, D471; DCT-POCS, D472) — never on R0.

`lama` (§3.1) reads R11 stage 1's hole shares — `invariance.csv` as R11's
  `scripts/grok/invariance.py run` writes it (one row per pair: `source`,
  `size`, `hole_share`, …, `reading`), or a plain file a person writes:

* JSON: a list, or `{"rows": [...]}`, of `{"profile", "source",
  "hole_share"}` — the share of the mark's support with `α̂ ≥ opaque_above`,
  a fraction in [0, 1] (`hole_pct`, a percentage, is read too);
* CSV with a header naming `source` and `hole_share` (or `hole_pct`), and
  `profile` if there is one.

A row of `invariance.csv` whose `reading` is `none` — no support, nothing
departs from the background at that rectangle — has no hole share and is
left out, said in the report; any other row without one is a refusal.

Any row over 1 % (`HOLE_SHARE`, `[tunable]`) → "LaMa mandatory"; any above
0 → "LaMa evaluated as an option"; none → "Grok has no holes: LaMa not
evaluated". Gemini is always "not evaluated" (S10: no Gemini map has a
pixel with `α ≥ 0.95`).

How to run it
-------------
    python3 scripts/model-eval/trigger.py fdncnn --run reports/regress-<commit>-<date>/ [--run …] \
        [--classes recon-jpeg-444,recon-jpeg-420] [--ab ab/remainder/score.json] \
        [--out docs/plan/reports/E12-R10-fdncnn-decision-<date>.md]
    python3 scripts/model-eval/trigger.py lama --holes r11-holes.json \
        [--out docs/plan/reports/E12-R10-lama-decision-<date>.md]
    python3 scripts/model-eval/trigger.py selftest

What it needs
-------------
Python 3.10+ and numpy (through `evalkit.py`, whose helpers it shares;
none of the model's packages). An R1 run made by a release CLI
built at or after the owner's decisions of 2026-10-10, which carries R6
and R8's DCT-POCS by default (`scripts/regress.py run`).

What its output means
---------------------
Markdown (stdout, or `--out`) and the same as JSON beside it
(`<out>.json`): per variant n, the files counted, by texture, by chroma,
the ones apart, the share; the A/B; the CLI's check; the verdict line.
Exit 0 when the verdict is "not evaluated", 1 when the model is to be run
(a finding: there is work), 2 on a usage error or an unreadable input.
"""

import argparse
import csv
import json
import os
import re
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import evalkit as ek  # noqa: E402

# ── [tunable] — R10 §2.1 and §3.1; each moves only with a line in a report ──
SHARE = 0.20        # files of a variant with a texture or a chroma outline left
AB_SHARE = 0.30     # R* items seen with a visible remainder
HOLE_SHARE = 0.01   # Grok: holes over this share of the mark's support make LaMa mandatory
Q_RANGE = (85, 95)
DEFAULT_CLASSES = ("recon-jpeg-444", "recon-jpeg-420")


# ───────────────────────────────────────────────────────── FDnCNN

def quality_of(variant):
    m = re.search(r"(?:^|-)q(\d+)(?:-|$)", variant or "")
    return int(m.group(1)) if m else None


def restorations(rec):
    """The `restored[]` of a record's `clean --json`, as `regress.py` `facts` reads it."""
    cl = rec.get("clean") or {}
    cj = cl.get("json") if isinstance(cl.get("json"), dict) else {}
    rep = cj.get("report") if isinstance(cj.get("report"), dict) else {}
    vis = rep.get("visible") if isinstance(rep.get("visible"), dict) else {}
    return [r for r in (vis.get("restored") or []) if isinstance(r, dict)]


def why_left(r):
    """What a restoration left, as this trigger reads it: a set of reasons."""
    out = set()
    if r.get("texture_left"):
        out.add("texture")
    if r.get("outline_left"):
        if isinstance(r.get("chroma"), (int, float)) and r["chroma"] > ek.CHROMA_LEVELS:
            out.add("chroma")
            if (r.get("outline") or 0) > 0.20 or abs(r.get("step") or 0) > ek.STEP_LEVELS:
                out.add("chroma-or-other")
        else:
            out.add("outline-other")
    if r.get("smoothed"):
        out.add("smoothed")
    return out


COUNTED = {"texture", "chroma"}


def read_runs(dirs):
    recs = {}
    for d in dirs:
        with open(os.path.join(d, "index.json")) as f:
            index = json.load(f)
        for fid in index["files"]:
            with open(os.path.join(d, fid + ".json")) as f:
                recs[fid] = json.load(f)
    return recs


def fdncnn_rows(recs, classes=DEFAULT_CLASSES):
    """Per (class, variant) in q85–q95: the counts the trigger reads, by **file**."""
    groups = {}
    for fid, rec in sorted(recs.items()):
        cls, variant = rec.get("class"), rec.get("variant")
        q = quality_of(variant)
        if cls not in classes or q is None or not (Q_RANGE[0] <= q <= Q_RANGE[1]):
            continue
        g = groups.setdefault((cls, variant), {"class": cls, "variant": variant, "n": 0, "restored": 0,
                                               "counted": 0, "texture": 0, "chroma": 0, "chroma_or_other": 0,
                                               "outline_other": 0, "smoothed": 0, "files": []})
        g["n"] += 1
        rs = restorations(rec)
        if rs:
            g["restored"] += 1
        reasons = set()
        for r in rs:
            reasons |= why_left(r)
        if reasons & COUNTED:
            g["counted"] += 1
            g["files"].append(fid)
        g["texture"] += "texture" in reasons
        g["chroma"] += "chroma" in reasons
        g["chroma_or_other"] += "chroma-or-other" in reasons
        g["outline_other"] += bool("outline-other" in reasons and not reasons & COUNTED)
        g["smoothed"] += bool("smoothed" in reasons and not reasons & COUNTED)
    for g in groups.values():
        g["share"] = g["counted"] / g["n"] if g["n"] else 0.0
    return [groups[k] for k in sorted(groups)]


def cli_check(recs, classes=DEFAULT_CLASSES):
    """How many restorations say they were made by R6 (`planar`) and R8 (`interval`)."""
    r420 = planar = lossy = interval = 0
    for rec in recs.values():
        if rec.get("class") not in classes:
            continue
        for r in restorations(rec):
            if r.get("lossy"):
                lossy += 1
                interval += "interval" in r
            if rec.get("class") == "recon-jpeg-420" or "420" in (rec.get("variant") or ""):
                r420 += 1
                planar += isinstance(r.get("planar"), dict)
    return {"restorations_420": r420, "planar": planar, "restorations_lossy": lossy, "interval": interval,
            "looks_like_r6": r420 == 0 or planar > 0, "looks_like_r8": lossy == 0 or interval > 0}


def ab_share(score):
    """The pooled share of R* items seen with a visible remainder, from `ab.py score` (question `remainder`)."""
    if not score:
        return None
    if score.get("question") != "remainder":
        raise ek.Refusal(f"the A/B score answers {score.get('question')!r}, not 'remainder'")
    return {"share": score["pooled"]["share"], "items": score["pooled"]["items"],
            "observers": sorted(score.get("observers", {}))}


def fdncnn_verdict(rows, ab):
    tripped = [r for r in rows if r["n"] and r["share"] >= SHARE]
    ab_tripped = ab is not None and ab["share"] is not None and ab["share"] >= AB_SHARE
    run = bool(tripped) or ab_tripped
    why = []
    if tripped:
        why.append("; ".join(f"{r['class']} {r['variant']}: {r['counted']}/{r['n']} files ({100 * r['share']:.1f} %)" for r in tripped))
    if ab_tripped:
        why.append(f"the blind A/B: {100 * ab['share']:.1f} % of R* items with a visible remainder")
    line = ("FDnCNN to be run (fdncnn_run.py): " + " and ".join(why)) if run else \
        "FDnCNN not evaluated: no variant from q85 to q95 reaches the share, and the A/B does not either"
    return run, line


def fdncnn_report(runs, rows, check, ab, run, line):
    date = time.strftime("%Y-%m-%d")
    md = [f"# E12-R10 — the FDnCNN trigger, {date}", "",
          "Written by `scripts/model-eval/trigger.py fdncnn` (R10 §2.1).", "",
          f"* runs: {', '.join(runs)}",
          f"* the rule: texture_left, or outline_left by chroma (chroma > {ek.CHROMA_LEVELS}), on ≥ {100 * SHARE:.0f} % "
          f"of the **files** of a variant from q{Q_RANGE[0]} to q{Q_RANGE[1]}; or a visible remainder in ≥ "
          f"{100 * AB_SHARE:.0f} % of the blind A/B's R* items", "",
          "## Per variant", "",
          ek.md_table(["class", "variant", "files", "restored", "counted", "by texture", "by chroma",
                       "by chroma, share or step could also", "outline by share/step only (apart)",
                       "smoothed only (apart)", "share"],
                      [[r["class"], r["variant"], r["n"], r["restored"], r["counted"], r["texture"], r["chroma"],
                        r["chroma_or_other"], r["outline_other"], r["smoothed"], f"{100 * r['share']:.1f} %"]
                       for r in rows]) if rows else "No file of the classes in q85–q95: the trigger cannot be read.",
          "", "## The blind A/B", ""]
    if ab is None:
        md.append("Not given (Q-R7: who looks is the owner's question). The trigger is read on the files alone.")
    else:
        md.append(f"{100 * ab['share']:.1f} % of {ab['items']} answers on R* items saw a remainder "
                  f"(observers: {', '.join(ab['observers']) or '–'}).")
    md += ["", "## The CLI that ran", "",
           f"* 4:2:0 restorations carrying `planar` (R6): {check['planar']} of {check['restorations_420']}",
           f"* lossy restorations carrying `interval` (R8): {check['interval']} of {check['restorations_lossy']}"]
    if not (check["looks_like_r6"] and check["looks_like_r8"]):
        md.append("* **warning**: the run does not look like it was made with the accepted R6/R8 in the CLI "
                  "(a release CLI built at or after D471/D472). The trigger is measured after R*, never on R0.")
    md += ["", "## Verdict", "", line, ""]
    return "\n".join(md)


def cmd_fdncnn(args):
    recs = read_runs(args.run)
    classes = tuple(args.classes.split(",")) if args.classes else DEFAULT_CLASSES
    rows = fdncnn_rows(recs, classes)
    check = cli_check(recs, classes)
    ab = None
    if args.ab:
        with open(args.ab) as f:
            ab = ab_share(json.load(f))
    run, line = fdncnn_verdict(rows, ab)
    text = fdncnn_report(args.run, rows, check, ab, run, line)
    emit(args.out, text, {"rows": rows, "cli": check, "ab": ab, "run": run, "verdict": line,
                          "share": SHARE, "ab_share": AB_SHARE, "q_range": Q_RANGE})
    return 1 if run else 0


# ───────────────────────────────────────────────────────── LaMa

def read_holes(path, skipped=None):
    """R11 stage 1's hole shares, JSON or CSV (see the header): [{profile, source, hole_share}]."""
    with open(path, newline="") as f:
        text = f.read()
    try:
        data = json.loads(text)
        rows = data["rows"] if isinstance(data, dict) else data
    except ValueError:
        rows = list(csv.DictReader(text.splitlines()))
    out = []
    for r in rows:
        if r.get("hole_share") in (None, "") and r.get("hole_pct") in (None, "") \
                and str(r.get("reading", "")).strip() == "none":
            if skipped is not None:  # invariance.py: no support at this pair's rectangle, so no share to read
                skipped.append(f"{r.get('source') or '-'} {r.get('size') or ''}".strip())
            continue
        if r.get("hole_share") not in (None, ""):
            share = float(r["hole_share"])
        elif r.get("hole_pct") not in (None, ""):
            share = float(r["hole_pct"]) / 100.0
        else:
            raise ek.Refusal(f"{path}: a row with neither hole_share nor hole_pct: {r}")
        if not 0.0 <= share <= 1.0:
            raise ek.Refusal(f"{path}: a hole share outside [0, 1]: {r}")
        out.append({"profile": r.get("profile") or "-", "source": r.get("source") or "-", "hole_share": share,
                    "size": r.get("size") or "-"})
    return out


def lama_verdict(rows):
    over = [r for r in rows if r["hole_share"] > HOLE_SHARE]
    some = [r for r in rows if r["hole_share"] > 0.0]
    if over:
        return "mandatory", ("LaMa mandatory for Grok: holes over 1 % of the mark's support in "
                             + ", ".join(f"{r['profile']}/{r['source']} ({100 * r['hole_share']:.2f} %)" for r in over)
                             + " — without inpainting every such file is a mark left (exit 3), and E12-7 is a blocker")
    if some:
        return "option", "LaMa evaluated as an option for Grok: holes under 1 % (exit 3 → 0 on a small share of pixels), same gates"
    return "none", "Grok has no holes: LaMa not evaluated"


def cmd_lama(args):
    skipped = []
    rows = read_holes(args.holes, skipped)
    kind, line = lama_verdict(rows)
    md = [f"# E12-R10 — the LaMa trigger, {time.strftime('%Y-%m-%d')}", "",
          "Written by `scripts/model-eval/trigger.py lama` (R10 §3.1).", "",
          "* **Gemini: not evaluated.** No Gemini map has a pixel with α ≥ 0.95; the peaks are 0.33–0.51 "
          "(`E12-R2-corpora.md` §3). Recorded by S10.", f"* input: `{args.holes}`", "",
          ek.md_table(["profile", "source", "size", "hole share"],
                      [[r["profile"], r["source"], r["size"], f"{100 * r['hole_share']:.3f} %"] for r in rows]),
          ""] + ([f"* left out, no support (`reading` none): {', '.join(skipped)}", ""] if skipped else []) + [
          "## Verdict", "", line, ""]
    emit(args.out, "\n".join(md), {"rows": rows, "verdict": line, "kind": kind, "hole_share": HOLE_SHARE})
    return 0 if kind == "none" else 1


def emit(out, text, data):
    if out:
        os.makedirs(os.path.dirname(os.path.abspath(out)), exist_ok=True)
        with open(out, "w") as f:
            f.write(text)
        ek.write_json(out + ".json", data)
        print(f"written: {out}")
    else:
        print(text)


# ───────────────────────────────────────────────────────── selftest

def fake(fid, variant, restored, cls="recon-jpeg-444"):
    return fid, {"id": fid, "class": cls, "variant": variant,
                 "clean": {"exit": 3 if restored else 0,
                           "json": {"report": {"visible": {"found": [], "restored": restored}}}}}


def rest(texture_left=False, outline_left=False, chroma=0.5, outline=0.05, step=0.1, smoothed=False, lossy=True,
         planar=None, interval=None):
    r = {"texture_left": texture_left, "outline_left": outline_left, "chroma": chroma, "outline": outline,
         "step": step, "lossy": lossy}
    if smoothed:
        r["smoothed"] = True
    if planar:
        r["planar"] = planar
    if interval:
        r["interval"] = interval
    return r


def _t_the_trigger_counts_left_marks_by_file_not_by_mark():
    # Ten files at q95. One of them carries five restorations, every one with a
    # texture left; the other nine one clean restoration each. By file: 1/10 —
    # under the bar. By mark it would be 5/14 = 36 % — over it.
    recs = dict([fake("many", "q95", [rest(texture_left=True) for _ in range(5)])]
                + [fake(f"f{i}", "q95", [rest()]) for i in range(9)])
    rows = fdncnn_rows(recs)
    assert len(rows) == 1 and rows[0]["n"] == 10, rows
    assert rows[0]["counted"] == 1 and abs(rows[0]["share"] - 0.1) < 1e-12, rows
    run, line = fdncnn_verdict(rows, None)
    assert not run and line.startswith("FDnCNN not evaluated"), line
    # …and two such files in ten is the bar itself.
    recs.update([fake("f0", "q95", [rest(texture_left=True)])])
    run, _ = fdncnn_verdict(fdncnn_rows(recs), None)
    assert run


def _t_an_outline_counts_only_by_chroma():
    recs = dict([fake("step", "q90", [rest(outline_left=True, chroma=2.0, step=3.0)]),
                 fake("chroma", "q90", [rest(outline_left=True, chroma=5.0)]),
                 fake("both", "q90", [rest(outline_left=True, chroma=5.0, outline=0.3)]),
                 fake("soap", "q90", [rest(smoothed=True)]),
                 fake("none", "q90", [])])
    (row,) = fdncnn_rows(recs)
    assert (row["n"], row["restored"], row["counted"], row["chroma"], row["chroma_or_other"]) == (5, 4, 2, 2, 1), row
    assert (row["outline_other"], row["smoothed"]) == (1, 1), row
    assert sorted(row["files"]) == ["both", "chroma"]


def _t_only_q85_to_q95_of_the_jpeg_classes_is_read():
    recs = dict([fake("a", "q75", [rest(texture_left=True)]), fake("b", "fixture-q98", [rest(texture_left=True)]),
                 fake("c", "fixture-q95", [rest()]), fake("d", "q85", [rest()], cls="recon-jpeg-420"),
                 fake("e", "png", [rest(texture_left=True)], cls="recon-png"),
                 fake("f", "x0.9-q90-420", [rest(texture_left=True)], cls="recon-resized")])
    got = [(r["class"], r["variant"]) for r in fdncnn_rows(recs)]
    assert got == [("recon-jpeg-420", "q85"), ("recon-jpeg-444", "fixture-q95")], got


def _t_the_blind_ab_trips_the_trigger_on_its_own():
    recs = dict([fake("a", "q95", [rest()])])
    rows = fdncnn_rows(recs)
    ab = ab_share({"question": "remainder", "pooled": {"share": 0.35, "items": 20}, "observers": {"o1": {}}})
    run, line = fdncnn_verdict(rows, ab)
    assert run and "A/B" in line, line
    run, _ = fdncnn_verdict(rows, dict(ab, share=0.25))
    assert not run


def _t_a_run_without_r6_or_r8_is_said():
    recs = dict([fake("a", "q95", [rest()], cls="recon-jpeg-420")])
    c = cli_check(recs)
    assert not c["looks_like_r6"] and not c["looks_like_r8"], c
    recs = dict([fake("a", "q95", [rest(planar={"sampling": "4:2:0"}, interval={"method": "dct"})], cls="recon-jpeg-420")])
    c = cli_check(recs)
    assert c["looks_like_r6"] and c["looks_like_r8"], c
    assert "warning" not in fdncnn_report(["x"], fdncnn_rows(recs), c, None, False, "v")


def _t_r11s_invariance_csv_is_read_as_it_is():
    """`scripts/grok/invariance.py run` (R11) writes the file this reads."""
    import importlib.util
    import tempfile

    here = os.path.dirname(os.path.abspath(__file__))
    spec = importlib.util.spec_from_file_location("grok_invariance", os.path.join(here, "..", "grok", "invariance.py"))
    inv = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(inv)
    with tempfile.TemporaryDirectory() as d:
        lists = [inv.synth_pair(d, "holed", 12, 5, hole=True)[0], inv.synth_pair(d, "bare", 12, 6, peak=0.0)[0]]
        rows = [r for p in lists for r in inv.read_rows(p)]
        out = os.path.join(d, "inv")
        results, _ = inv.run(rows, inv.defaults(), out)
        assert sorted(st["reading"] == "none" for st, _ in results) == [False, True], [st["reading"] for st, _ in results]
        skipped = []
        got = read_holes(os.path.join(out, "invariance.csv"), skipped)
        assert [r["source"] for r in got] == ["holed"] and skipped == ["bare 320x200"], (got, skipped)
        assert got[0]["hole_share"] > HOLE_SHARE and got[0]["size"] == "320x200", got
        assert lama_verdict(got)[0] == "mandatory"


def _t_holes_over_one_percent_make_lama_mandatory():
    import tempfile

    with tempfile.TemporaryDirectory() as d:
        j = os.path.join(d, "h.json")
        with open(j, "w") as f:
            json.dump({"rows": [{"profile": "grok", "source": "web", "hole_share": 0.012},
                                {"profile": "grok", "source": "app", "hole_share": 0.0}]}, f)
        assert lama_verdict(read_holes(j))[0] == "mandatory"
        c = os.path.join(d, "h.csv")
        with open(c, "w") as f:
            f.write("profile,source,hole_pct\ngrok,web,0.5\ngrok,app,0\n")
        assert lama_verdict(read_holes(c))[0] == "option"
        with open(c, "w") as f:
            f.write("profile,source,hole_share\ngrok,web,0\n")
        assert lama_verdict(read_holes(c))[0] == "none"
        with open(c, "w") as f:
            f.write("profile,source,hole_share\ngrok,web,1.5\n")
        try:
            read_holes(c)
        except ek.Refusal:
            pass
        else:
            raise AssertionError("a share over 1 was read")


def selftest():
    return ek.run_cases([
        ("the_trigger_counts_left_marks_by_file_not_by_mark", _t_the_trigger_counts_left_marks_by_file_not_by_mark),
        ("an_outline_counts_only_by_chroma", _t_an_outline_counts_only_by_chroma),
        ("only_q85_to_q95_of_the_jpeg_classes_is_read", _t_only_q85_to_q95_of_the_jpeg_classes_is_read),
        ("the_blind_ab_trips_the_trigger_on_its_own", _t_the_blind_ab_trips_the_trigger_on_its_own),
        ("a_run_without_r6_or_r8_is_said", _t_a_run_without_r6_or_r8_is_said),
        ("holes_over_one_percent_make_lama_mandatory", _t_holes_over_one_percent_make_lama_mandatory),
        ("r11s_invariance_csv_is_read_as_it_is", _t_r11s_invariance_csv_is_read_as_it_is),
    ])


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    sub = p.add_subparsers(dest="cmd", required=True)
    s = sub.add_parser("fdncnn")
    s.add_argument("--run", action="append", required=True, help="an R1 run folder (regress.py baseline|run --out)")
    s.add_argument("--classes", help=f"comma-separated (default {','.join(DEFAULT_CLASSES)})")
    s.add_argument("--ab", help="ab.py score's JSON for the `remainder` question")
    s.add_argument("--out")
    s = sub.add_parser("lama")
    s.add_argument("--holes", required=True, help="R11 stage 1's hole shares, JSON or CSV")
    s.add_argument("--out")
    sub.add_parser("selftest")
    args = p.parse_args(argv)
    try:
        if args.cmd == "selftest":
            return selftest()
        return {"fdncnn": cmd_fdncnn, "lama": cmd_lama}[args.cmd](args)
    except (ek.Refusal, OSError, KeyError, ValueError) as e:
        print(f"trigger.py {args.cmd}: {e}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
