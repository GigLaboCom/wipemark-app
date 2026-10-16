#!/usr/bin/env python3
"""The Grok chain's helpers — the catalogue file, the degradation list, the summary — and its self-test.

What it is for
--------------
The E12-R close-out (the coordinator's task `wipemark-task-recon-closeout-grok-ready-2026-10-10`,
2026-10-10; `docs/plan/E12-R-closeout-grok-ready.md`, K2–K3; D526–D528). `scripts/grok/run.sh` is the
one command from a folder of Grok captures to a proposed profile; the parts of it that read and write
JSON and Markdown are here, so the shell script stays a list of steps. `selftest` is K3's guard: it runs
`run.sh` end to end on stand-in captures (`scripts/grok/render.py`, the synthetic wordmark fixture — not
Grok's mark), so the chain cannot rot unnoticed.

What it does
------------
* `catalogue --out FILE ROW.json …` — one catalogue file in the shipped schema (`{"schema": 1,
  "profiles": [...]}`) from the rows `calibrate` drafted, each profile `provisional`; the maps are
  copied beside it by `run.sh`. It is a `--catalogue` file for the tools (D526): nothing here touches
  `manifests/marks.v1.json`.
* `degradations --manifest M --source web --committed bench/degradations/grok.json --out FILE` — the
  bench's degradation list for one source from what stage 0 read off its stills: `png` always (the
  map's self-test, R12 §4.2), `jpeg420-qNN`/`jpeg444-qNN` for every JPEG's subsampling and IJG
  quality, `webp-lossy-q90` for a lossy WebP. With no still to read, the committed list (D527). Says
  whether the two differ, so the committed list can be brought to the facts.
* `summary --out DIR …` — `DIR/summary.md`: every step's outcome (`steps.tsv`), stage 0's gate and
  how many cells are still "to fill", stage 1's line per source and its holes, each drafted profile's
  rows, logo, fit and replay, the bench's table, the clean measures, the regression, and **what a
  person has to look at**, in order.
* `selftest` — renders stand-in captures into a temporary folder and runs `run.sh --no-build` over
  them twice: with the sidecar (every step runs or is refused by name, the catalogue file holds the
  drafted profile, the summary is written, exit 3 because P1–P5 wait for a CLI with the profile) and
  without it (the chain stops at stage 1, naming the template to fill, exit 2, the summary still
  written).

How to run it
-------------
    python3 scripts/grok/chain.py catalogue --out reports/grok-<date>/catalogue/marks.json ROW.json …
    python3 scripts/grok/chain.py degradations --manifest corpus/grok/manifest.json --source web \\
        --committed bench/degradations/grok.json --out degradations-web.json
    python3 scripts/grok/chain.py summary --out reports/grok-<date> --captures DIR --manifest M --stage0 T --date D
    python3 scripts/grok/chain.py selftest

What it needs
-------------
Python 3.11+ (the standard library for the three helpers; numpy and Pillow for `selftest`, through
`render.py` and `run.sh`). `selftest` needs the release CLI and the examples built first
(`cargo build --release -p wipemark-cli -p wipemark-picture --examples --bins` and
`cargo build --release -p wipemark-pixels --example calibrate`) — it fails, saying so, without them.

What its output means
---------------------
The files named above. Exit codes: 0 done (selftest: all passed), 1 a selftest check failed, 2 usage
or a refusal.
"""

import argparse
import csv
import json
import os
import re
import subprocess
import sys
import tempfile

REPO = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
LONG = {"web": "grok.com", "x": "grok-in-x", "api": "xai-api", "video": "grok-imagine-video"}


class Refusal(Exception):
    pass


# ── catalogue ────────────────────────────────────────────────────────────────


def catalogue(rows, out):
    profiles = []
    for path in rows:
        with open(path) as f:
            row = json.load(f)
        if row.get("status") != "provisional":
            raise Refusal(f"{path}: a drafted row is provisional, this one says {row.get('status')!r}")
        profiles.append(row)
    ids = [p["id"] for p in profiles]
    if len(set(ids)) != len(ids):
        raise Refusal(f"two rows share an id: {ids}")
    os.makedirs(os.path.dirname(os.path.abspath(out)), exist_ok=True)
    with open(out, "w") as f:
        json.dump({"schema": 1, "profiles": profiles}, f, indent=2)
        f.write("\n")
    return ids


# ── degradations ─────────────────────────────────────────────────────────────


def derived_slices(manifest, source):
    long = LONG.get(source, source)
    seen, stills = ["png"], 0
    for r in manifest["files"]:
        f = r.get("facts") or {}
        if r.get("profile") != long or f.get("kind") != "picture":
            continue
        stills += 1
        j = f.get("jpeg")
        if j:
            q = (j.get("quality") or {}).get("luma")
            sub = j.get("subsampling")
            if q and sub in ("4:2:0", "4:4:4"):
                seen.append(f"jpeg{sub.replace(':', '')}-q{q[0]}")
        elif str(f.get("format", "")).startswith("WebP lossy"):
            seen.append("webp-lossy-q90")
    out = []
    for s in seen:
        if s not in out:
            out.append(s)
    return out, stills


def degradations(manifest_path, source, committed_path, out):
    with open(manifest_path) as f:
        manifest = json.load(f)
    committed = None
    if committed_path and os.path.exists(committed_path):
        with open(committed_path) as f:
            committed = json.load(f)
    slices, stills = derived_slices(manifest, source)
    if stills == 0:
        if committed is None:
            raise Refusal(f"no still of {source} and no committed list at {committed_path}")
        doc = dict(committed)
        said = f"no still of {source}: the committed list {committed_path}"
    else:
        doc = {"schema": 1, "profile": None, "slices": slices,
               "comment": f"from stage 0's facts of {LONG.get(source, source)} ({stills} still(s)), by "
                          "scripts/grok/chain.py degradations; png is the map's self-test (E12-R12 §4.2)"}
        same = committed is not None and committed.get("slices") == slices
        said = (f"{source}: {', '.join(slices)} from {stills} still(s)"
                + ("; the same as the committed list" if same else
                   f"; differs from {committed_path} ({', '.join((committed or {}).get('slices') or ['none'])}) — "
                   "bring the committed list to the facts (D527)"))
    os.makedirs(os.path.dirname(os.path.abspath(out)), exist_ok=True)
    with open(out, "w") as f:
        json.dump(doc, f, indent=2)
        f.write("\n")
    print(said)
    return doc, said


# ── summary ──────────────────────────────────────────────────────────────────


def read_steps(out):
    path = os.path.join(out, "steps.tsv")
    if not os.path.exists(path):
        return []
    with open(path, newline="") as f:
        return list(csv.DictReader(f, delimiter="\t"))


def text_of(path):
    try:
        with open(path) as f:
            return f.read()
    except OSError:
        return None


def table_after(text, header_start):
    """The Markdown table whose header line starts with `header_start`, as its lines."""
    if not text:
        return []
    lines = text.splitlines()
    for i, line in enumerate(lines):
        if line.startswith(header_start):
            out = [line]
            for more in lines[i + 1:]:
                if not more.startswith("|"):
                    break
                out.append(more)
            return out
    return []


def summary(out, captures, manifest_path, stage0_path, date):
    steps = read_steps(out)
    lines = [f"# Grok chain — {date}", "",
             f"Written by `scripts/grok/run.sh` over `{captures}` (the steps: the script's header; the capture guide: "
             "`docs/plan/E12-R-grok-capture-guide.md`). Every figure below is about these captures alone.", "",
             "## Steps", "", "| step | outcome | what |", "|---|---|---|"]
    for s in steps:
        lines.append(f"| {s['step']} | {s['status']} | {s['what'].replace('|', '/')} |")
    look = []

    # Stage 0
    lines += ["", "## Stage 0 (R2 §4.3)", ""]
    t0 = text_of(stage0_path)
    try:
        with open(manifest_path) as f:
            manifest = json.load(f)
    except (OSError, ValueError):
        manifest = None
    if t0 is None:
        lines.append(f"No stage-0 table at `{stage0_path}`.")
    else:
        gate = next((ln for ln in t0.splitlines() if ln.startswith("**Gate")), "")
        lines += [f"The table: `{stage0_path}`. {gate}", ""]
        lines += table_after(t0, "| source | mark? |")
        every = table_after(t0, "| file | source | mark |")[2:]
        to_fill = sum(row.split(" | ").count("to fill") for row in every)
        lines += ["", f"Cells still \"to fill\" in the every-file table: **{to_fill}**."]
        if to_fill:
            look.append(f"Fill the by-hand fields by eye ({to_fill} cell(s) \"to fill\"): copy "
                        f"`{os.path.join(out, 'grok.toml.template')}` to `{os.path.join(captures, 'grok.toml')}` "
                        "(or add to it), then run the command again.")
        if "not met" in gate:
            look.append("Stage 0's gate is not met: " + gate.split("not met — ", 1)[-1])
    if manifest:
        rows = manifest["files"]
        by = {}
        for r in rows:
            by.setdefault(r["profile"], []).append(r)
        lines += ["", "| source | files | held out | stills with a flat ring that failed |", "|---|---|---|---|"]
        for src, rs in sorted(by.items()):
            failed = sum(1 for r in rs if r.get("ring") is not None and not r["ring"].get("pass"))
            lines.append(f"| {src} | {len(rs)} | {sum(1 for r in rs if r['held_out'])} | {failed} |")
        keys = [s.get("key") for s in manifest.get("sources", {}).values() if s.get("key")]
        if keys:
            look.append("Upload the stored ZIP(s) to Watchword, ttl 0, and read each back for no `expires_at`: "
                        + "; ".join(f"`{os.path.join(out, 'zip', k.split('wipemark-corpus-')[1].rsplit('-', 3)[0] + '.zip')}` as "
                                    f"FILE `{k}`" for k in keys)
                        + f". Then commit `{manifest_path}` and `{stage0_path}`.")

    # Stage 1
    lines += ["", "## Stage 1 (R11 §4.1)", ""]
    for name, sub in (("at one integer offset", "invariance"), ("after alignment", "invariance-aligned")):
        t1 = text_of(os.path.join(out, "r11", sub, "invariance.md"))
        if t1 is None:
            lines.append(f"* {name}: not run.")
            continue
        tab = table_after(t1, "| source | pairs | stage-1 line |")
        lines += [f"* {name}: `{os.path.join(out, 'r11', sub, 'invariance.md')}`", ""] + tab + [""]
        skipped = [ln for ln in t1.splitlines() if ln.startswith("* skipped")]
        lines += skipped
        if sub == "invariance":
            for row in tab[2:]:
                cells = [c.strip() for c in row.strip("|").split("|")]
                if len(cells) >= 3 and cells[2] not in ("a map exists", "a map exists after alignment"):
                    look.append(f"Stage 1 reads `{cells[2]}` for {cells[0]}: read the maps under "
                                f"`{os.path.join(out, 'r11', 'invariance')}` before trusting it (a \"no map\" is Q-R4).")
        csvp = os.path.join(out, "r11", sub, "invariance.csv")
        if os.path.exists(csvp):
            with open(csvp, newline="") as f:
                holes = [(r["source"], r["size"], float(r["hole_share"] or 0)) for r in csv.DictReader(f)]
            worst = max(holes, key=lambda h: h[2], default=None)
            if worst:
                lines.append(f"Largest hole share {worst[2]:.4g} ({worst[0]} {worst[1]}).")
                if worst[2] > 0 and sub == "invariance":
                    look.append(f"Holes on {worst[0]} {worst[1]} (share {worst[2]:.4g}): over 1 % is Q-R6 and R10 §3.")
    # Stages 2–3
    lines += ["", "## Stages 2–3 (R11 §4.2–§4.3): the proposed profiles", ""]
    cat_path = os.path.join(out, "catalogue", "marks.json")
    cat = None
    if os.path.exists(cat_path):
        with open(cat_path) as f:
            cat = json.load(f)
        lines += [f"The proposed `--catalogue` file: `{cat_path}` — provisional, not the shipped catalogue (D526).", ""]
        for p in cat["profiles"]:
            short = p["id"].rsplit("-", 1)[-1]
            rep = text_of(os.path.join(out, "r11", "calibrate", short, "out", f"{p['id']}.report.md")) or ""
            verified = len(re.findall(r"\| true \|", rep))
            refused = len(re.findall(r"\| false \|", rep))
            r2 = re.findall(r"\| R² min / mean \| ([^|]+) \|", rep)
            rows = ", ".join(f"{q['when'].get('width')}×{q['when'].get('height')} at {q.get('rect')}" for q in p["placements"])
            lines += [f"* **{p['id']}** — {p['vendor']} {p['product']}, model `{p['blend']['model']}`, logo "
                      f"{p['blend']['logo']}, maps {', '.join(str(a['size']) for a in p['alpha'])}; rows {rows}; search "
                      f"at {p['search']['corner']}; R² (min / mean) {', '.join(x.strip() for x in r2) or '—'}; replay over "
                      f"its captures: {verified} proved, {refused} not."]
        look.append("Read each calibration report under `r11/calibrate/<source>/out/` (the fit, the replay, the grey "
                    "error that chose the model) before the profile goes further; set `WIPEMARK_FP_CORPUS` and calibrate "
                    "again for its false-positive pass.")
    else:
        lines.append("No profile was drafted (the steps above say why).")
    for s in steps:
        if s["step"].startswith("4 calibrate") and s["status"] == "refused":
            look.append(f"{s['step']}: {s['what']}")

    # R12b
    lines += ["", "## R12b: the bench, the clean measures, the regression", ""]
    if cat:
        for p in cat["profiles"]:
            short = p["id"].rsplit("-", 1)[-1]
            br = text_of(os.path.join(out, "r12", f"bench-{short}", "report.md"))
            if br:
                lines += [f"* `{p['id']}` on the bench: `{os.path.join(out, 'r12', f'bench-{short}', 'report.md')}`", ""]
                tab = table_after(br, "| config | slice | encoder | n | restored |")
                # The first nine columns (to SSIM's median); the rest are in the report.
                lines += ["|" + "|".join(row.strip().strip("|").split("|")[:9]) + "|" for row in tab]
                lines.append("")
            cl = text_of(os.path.join(out, "r12", f"clean-{short}.md"))
            if cl:
                said, head = [], None
                for ln in cl.splitlines():
                    if ln.startswith("### "):
                        head = ln[4:].split(":")[0]
                    elif ln.startswith("Said as an outline left") and head and "every group" in head:
                        said.append(f"{head}: {ln}")
                lines += [f"* `{p['id']}`'s clean measures (`measure_clean synth`, the pixels suite's procedural "
                          "backgrounds):"] + [f"  * {x}" for x in said]
            deg = os.path.join(out, "r12", f"degradations-{short}.json")
            if os.path.exists(deg):
                with open(deg) as f:
                    d = json.load(f)
                lines.append(f"* its degradation list: {', '.join(d['slices'])} ({d.get('comment', '')})")
    reg = next((s for s in steps if s["step"] == "5 regress"), None)
    if reg:
        lines.append(f"* the regression: {reg['status']} — {reg['what']}")
        if reg["status"] == "skipped":
            look.append("R11's P1–P5 (the held-out proof, the negatives, the Gemini corpus) need a CLI built with the "
                        "profile compiled in: add the row to a scratch copy of `manifests/marks.v1.json` with its maps "
                        "under `crates/wipemark-pixels/marks/grok/`, build the release CLI, and run this command again "
                        "with `--cli-with-profile` (R11 §4.3; never committed while provisional, D526).")
    for s in steps:
        if s["step"].startswith("5 ") and s["status"] == "refused":
            look.append(f"{s['step']}: {s['what']}")

    lines += ["", "## What a person has to look at", ""]
    lines += [f"{i}. {item}" for i, item in enumerate(look, 1)] or ["Nothing: every step ran."]
    lines += ["", "Nothing here says Grok is supported: that is R12 §4.5's six conditions, Q-R5 among them."]
    text = "\n".join(lines) + "\n"
    with open(os.path.join(out, "summary.md"), "w") as f:
        f.write(text.replace(REPO + os.sep, ""))  # paths inside the repository, said relative to it
    return look


# ── selftest ─────────────────────────────────────────────────────────────────


def selftest():
    failures = []

    def check(ok, what):
        print(("ok   " if ok else "FAIL ") + what)
        if not ok:
            failures.append(what)

    ex = os.path.join(REPO, "target", "release", "examples")
    need = [os.path.join(REPO, "target", "release", "wipemark-cli")] + [os.path.join(ex, n) for n in
                                                                        ("recon_bench", "measure_clean", "calibrate")]
    missing = [n for n in need if not os.access(n, os.X_OK)]
    if missing:
        print("FAIL the release CLI and the examples are built — missing: " + ", ".join(missing)
              + " (cargo build --release -p wipemark-cli -p wipemark-picture --examples --bins; "
                "cargo build --release -p wipemark-pixels --example calibrate)")
        return 1
    sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
    import render  # noqa: E402 — the stand-ins

    run_sh = os.path.join(REPO, "scripts", "grok", "run.sh")
    with tempfile.TemporaryDirectory(prefix="chain-selftest-") as tmp:
        caps = os.path.join(tmp, "grok-captures")
        render.render(caps, sources=("web",), sizes=((1100, 1100),), per_group=2, content=0, seed=5)
        n = sum(len(fs) for _, _, fs in os.walk(os.path.join(caps, "web")))
        check(n == 6, f"six stand-in captures: two each on black, white and grey ({n})")

        def chain(out, *extra):
            argv = ["bash", run_sh, caps, "--out", out, "--no-build", "--date", "2026-10-10",
                    "--manifest", os.path.join(out, "manifest.json"), "--stage0", os.path.join(out, "stage0.md"),
                    "--id-prefix", "standin", "--bench-sample", "1", "--bench-groups", "flat", "--configs", "R0",
                    "--clean-count", "3", "--jobs", "2", *extra]
            p = subprocess.run(argv, capture_output=True, text=True, cwd=REPO, timeout=1800)
            return p.returncode, read_steps(out), p.stdout + p.stderr

        out = os.path.join(tmp, "with-sidecar")
        code, steps, said = chain(out)
        names = [s["step"] for s in steps]
        check(code == 3, f"the chain over a sidecar'd folder exits 3, P1–P5 waiting (exit {code})" + ("" if code == 3 else "\n" + said[-3000:]))
        for want in ("0 tools", "1 stage 0", "2 ZIP", "3 stage 1", "4 calibrate web", "4 catalogue", "5 bench web",
                     "5 clean web", "5 regress"):
            check(want in names, f"step {want!r} ran or was refused by name")
        check(all(s["status"] in ("ran", "attention", "refused", "skipped") and s["what"] for s in steps),
              "every step says what happened")
        check(next((s["status"] for s in steps if s["step"] == "4 calibrate web"), None) == "ran",
              "the stand-ins calibrate to a profile")
        check(next((s["status"] for s in steps if s["step"] == "5 regress"), None) == "skipped",
              "the regression is skipped by name without a CLI that carries the profile")
        try:
            with open(os.path.join(out, "catalogue", "marks.json")) as f:
                cat = json.load(f)
            ok = [p["id"] for p in cat["profiles"]] == ["standin-web"] and cat["profiles"][0]["status"] == "provisional" \
                and cat["profiles"][0]["search"]["corner"] == "bottom-left" \
                and all(os.path.exists(os.path.join(out, "catalogue", a["asset"])) for a in cat["profiles"][0]["alpha"])
        except (OSError, ValueError, KeyError, IndexError):
            ok = False
        check(ok, "the proposed catalogue file holds the drafted profile, provisional, its search at the mark's corner, its maps beside it")
        s = text_of(os.path.join(out, "summary.md")) or ""
        check("## What a person has to look at" in s and "P1–P5" in s, "the summary is written and names what is missing")
        check(os.path.exists(os.path.join(out, "zip", "grok.zip")), "the stored ZIP is written for the owner to upload")
        check(os.path.exists(os.path.join(out, "r12", "bench-web", "report.md")), "the bench ran on the drafted profile")

        os.remove(os.path.join(caps, "grok.toml"))
        out2 = os.path.join(tmp, "no-sidecar")
        code, steps, said = chain(out2)
        last = steps[-1] if steps else {}
        check(code == 2 and last.get("step") == "3 stage 1" and last.get("status") == "refused"
              and "grok.toml.template" in last.get("what", ""),
              f"with no sidecar the chain stops at stage 1, naming the template to fill (exit {code}, {last})")
        s2 = text_of(os.path.join(out2, "summary.md")) or ""
        check("to fill" in s2 and os.path.exists(os.path.join(out2, "grok.toml.template")),
              "the stopped chain's summary says what is \"to fill\", and the template is written")
    print(f"selftest: {'all passed' if not failures else str(len(failures)) + ' failed'}")
    return 1 if failures else 0


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    sub = ap.add_subparsers(dest="cmd", required=True)
    p = sub.add_parser("catalogue")
    p.add_argument("--out", required=True)
    p.add_argument("rows", nargs="+")
    p = sub.add_parser("degradations")
    p.add_argument("--manifest", required=True)
    p.add_argument("--source", required=True)
    p.add_argument("--committed")
    p.add_argument("--out", required=True)
    p = sub.add_parser("summary")
    p.add_argument("--out", required=True)
    p.add_argument("--captures", required=True)
    p.add_argument("--manifest", required=True)
    p.add_argument("--stage0", required=True)
    p.add_argument("--date", required=True)
    sub.add_parser("selftest")
    args = ap.parse_args(argv)
    try:
        if args.cmd == "selftest":
            return selftest()
        if args.cmd == "catalogue":
            ids = catalogue(args.rows, args.out)
            print(f"{args.out}: {', '.join(ids)}")
        elif args.cmd == "degradations":
            degradations(args.manifest, args.source, args.committed, args.out)
        else:
            look = summary(args.out, args.captures, args.manifest, args.stage0, args.date)
            print(f"{os.path.join(args.out, 'summary.md')}: {len(look)} thing(s) to look at")
        return 0
    except Refusal as e:
        print(f"chain.py: {e}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
