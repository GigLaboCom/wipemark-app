#!/usr/bin/env python3
"""The tables of docs/plan/reports/divergence-vs-upstream-2026-10-07.md.

What it is for
--------------
The owner asked on 2026-10-07 why a test rewrite (a 2 285-word article,
Qwen3.8 27B, paraphrase moderate, GPU 2 x 2, most-diverged pick) diverged
so far, compared with upstream `guillaumemeyer/watermarks-remover`. `run.sh`
beside this file makes the bench records — one variant per factor; this
script turns them into the report's measurements and paragraph samples.

What it does
------------
1. Reads `<dir>/runs/<variant>.jsonl` (one JSON line per bench attempt:
   the chunk as the model saw it, the answer after Layer A, the loop's
   verdict and divergence) and, if present, `<dir>/runs/judge.jsonl`
   (Gemma 4 12B's EQUIVALENT / CHANGED per attempt).
2. Simulates the selection policies on the same candidates, with the GPU
   2 x 2 schedule the loop uses (round 1 = candidates k1, k2; round 2 =
   k3, k4 only when nothing in round 1 qualified). A candidate qualifies
   when the loop's verdict is `passed` or `no-op` and its divergence is at
   least the policy's floor (the bench's own rule, `analyse.rs::simulate`):
     max>=0.2   the most diverged, floor 0.2 — the loop today (D111, D95)
     min>=0.2   the least diverged, floor 0.2
     min>=0.05  the least diverged, floor 0.05 — D71, upstream's floor
     k1>=0.05   the first candidate alone, floor 0.05 — upstream's 1 x 1
     max>=0.2, len<=1.3
                today's pick, with the length window's top at 1.3 instead
                of 1.6 (2.0 for a chunk under 20 words) — a tighter window
                simulated, not a run
   A chunk where nothing qualified keeps its source, as in the product.
3. Measures the chosen text of each (variant, policy), over the article's
   prose chunks, words taken as the loop takes them (runs of letters and
   digits, lower-cased) with the placeholders left out:
     rewritten / kept   chunks
     div med [q1-q3]    the loop's divergence of the chosen candidates
     words x            words of the result / words of the source (kept
                        chunks count as themselves)
     pairs left         share of the result's word pairs that are the
                        source's, by words (the bench's measure, E4-5)
     you                second-person words (you, your, yours, yourself,
                        yourselves) in the result; the source's count heads
                        the table
     judged CHANGED     the judge's share among the chosen candidates
4. Two rows have no bench records: the application's run (the report's JSON
   for divergence; its output file for the rest) and `whole` (upstream as it
   runs: the document in one call). Their text is aligned to the chunks: each
   chunk is matched to the block of the answer (a blank-line block or a
   single line) sharing the most word pairs with it; link targets, inline
   code and URLs are dropped from the answer before counting, since the
   chunk carries them as placeholders. An approximation, said so in the
   report.
5. Prints Markdown: the table, and three paragraph samples per row.

How to run
----------
    python3 -I docs/plan/reports/divergence-vs-upstream/analyse.py --dir tmp/divergence \\
        [--app-json <rewrite.json> --app-out <article.qwen38.md>] > tables.md

Needs Python 3.9+, standard library only.

Output
------
Markdown on stdout. Numbers are shares in [0, 1] unless marked x (a ratio)
or a count.
"""

import argparse
import json
import pathlib
import re
import statistics

YOU = {"you", "your", "yours", "yourself", "yourselves"}
SAMPLES = [
    ("intro", "Your agent is smart"),
    ("list item", "You ask Claude Code"),
    ("paragraph", "Every one of those"),
]
POLICIES = [
    ("max>=0.2", True, 0.2, False, None),
    ("min>=0.2", False, 0.2, False, None),
    ("min>=0.05", False, 0.05, False, None),
    ("k1>=0.05", False, 0.05, True, None),
    ("max>=0.2, len<=1.3", True, 0.2, False, 1.3),
]
PH = re.compile(r"⟦\d+⟧")


def words(text: str, placeholders: bool = False) -> list:
    out, word = [], []
    i = 0
    while i < len(text):
        m = PH.match(text, i)
        if m:
            if word:
                out.append("".join(word))
                word = []
            if placeholders:
                out.append(m.group(0))
            i = m.end()
            continue
        c = text[i]
        if c.isalnum():
            word.append(c.lower())
        elif word:
            out.append("".join(word))
            word = []
        i += 1
    if word:
        out.append("".join(word))
    return out


def pairs(ws: list) -> list:
    return list(zip(ws, ws[1:]))


def bigram_set(ws: list) -> set:
    if len(ws) == 1:
        return {(ws[0], "")}
    return set(pairs(ws))


def divergence(a: list, b: list) -> float:
    sa, sb = bigram_set(a), bigram_set(b)
    union = sa | sb
    return 1.0 - len(sa & sb) / len(union) if union else 0.0


def kept_pairs(src: list, ans: list) -> tuple:
    """(pairs of the answer that are the source's, pairs of the answer)."""
    s = set(pairs(src))
    p = pairs(ans)
    return sum(1 for x in p if x in s), len(p)


def you(ws: list) -> int:
    return sum(1 for w in ws if w in YOU)


def quart(xs: list) -> str:
    if not xs:
        return "–"
    if len(xs) < 4:
        return f"{statistics.median(xs):.2f}"
    q = statistics.quantiles(xs, n=4)
    return f"{q[1]:.2f} [{q[0]:.2f}–{q[2]:.2f}]"


def load(path: pathlib.Path) -> list:
    return [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines() if line.strip()]


def groups(records: list) -> dict:
    by = {}
    for r in records:
        by.setdefault(r["chunk"], {})[r["k"]] = r
    return dict(sorted(by.items()))


def pick(group: dict, most: bool, floor: float, single: bool, cap=None):
    rounds = [[1]] if single else [[1, 2], [3, 4]]
    for rnd in rounds:
        ok = [
            group[k]
            for k in rnd
            if k in group
            and group[k]["verdict"] in ("passed", "no-op")
            and group[k].get("divergence") is not None
            and group[k]["divergence"] >= floor
            and (cap is None or group[k]["length_ratio"] <= cap)
        ]
        if ok:
            if most:
                best = max(ok, key=lambda r: r["divergence"])  # first max wins, as the loop's tie rule
            else:
                best = min(ok, key=lambda r: r["divergence"])
            return best
    return None


def summarise(rows: list, judge: dict) -> dict:
    """rows: (source chunk text, chosen answer or None, divergence or None, key or None)."""
    rewritten = sum(1 for _, a, _, _ in rows if a is not None)
    divs = [d for _, a, d, _ in rows if a is not None and d is not None]
    sw = aw = kp = tp = sy = ay = 0
    for src, ans, _, _ in rows:
        s = words(src)
        a = words(ans) if ans is not None else s
        sw += len(s)
        aw += len(a)
        k, t = kept_pairs(s, a)
        kp += k
        tp += t
        sy += you(s)
        ay += you(a)
    judged = [judge[key] for _, a, _, key in rows if a is not None and key in judge]
    changed = sum(1 for v in judged if v == "CHANGED")
    return {
        "rewritten": rewritten,
        "kept": len(rows) - rewritten,
        "div": quart(divs),
        "words": aw / sw if sw else 0,
        "pairs_left": kp / tp if tp else 0,
        "you_src": sy,
        "you": ay,
        "judged": f"{changed}/{len(judged)}" if judged else "–",
    }


def strip_markup(text: str) -> str:
    text = re.sub(r"\]\([^)]*\)", "]", text)
    text = re.sub(r"`[^`]*`", " ", text)
    text = re.sub(r"https?://\S+", " ", text)
    return text


def blocks_of(doc: str) -> list:
    out = [b for b in re.split(r"\n\s*\n", doc) if b.strip()]
    out += [line for line in doc.splitlines() if line.strip()]
    return out


def align(chunks: list, doc: str) -> list:
    """Each chunk's best-matching block of `doc`, by shared word pairs."""
    cands = [(b, words(strip_markup(b))) for b in blocks_of(doc)]
    out = []
    for src in chunks:
        s = set(pairs(words(src)))
        sw = words(src)
        best, best_score = None, -1.0
        for b, bw in cands:
            if not bw:
                continue
            shared = len(s & set(pairs(bw)))
            # Prefer the block that shares the most pairs, then the one
            # closest in length (a whole section shares as many as its
            # paragraph does).
            score = shared - abs(len(bw) - len(sw)) * 0.01
            if score > best_score:
                best, best_score = b, score
        out.append(best)
    return out


def sample_rows(chunks: list, chosen: list) -> dict:
    out = {}
    for label, start in SAMPLES:
        for src, ans in zip(chunks, chosen):
            if src.lstrip().startswith(start):
                out[label] = (src, ans)
                break
    return out


def main() -> None:
    p = argparse.ArgumentParser()
    p.add_argument("--dir", required=True, type=pathlib.Path)
    p.add_argument("--app-json")
    p.add_argument("--app-out")
    a = p.parse_args()
    runs = a.dir / "runs"
    judge = {}
    calib = {}
    if (runs / "judge.jsonl").exists():
        for j in load(runs / "judge.jsonl"):
            if "of" in j:
                judge[j["of"]] = j["verdict"]
            elif "calib" in j:
                c = calib.setdefault(j["calib"], [0, 0])
                c[0] += j["verdict"] == ("EQUIVALENT" if j["calib"] == "same" else "CHANGED")
                c[1] += 1
    table = []
    samples = []
    base_chunks = None
    passed_info = []
    for variant in ["default", "light", "upstream", "voice", "voicelight", "t07"]:
        path = runs / f"{variant}.jsonl"
        if not path.exists():
            continue
        g = groups(load(path))
        chunks = [next(iter(x.values()))["chunk_text"] for x in g.values()]
        if base_chunks is None:
            base_chunks = chunks
        att = [r for x in g.values() for r in x.values()]
        passed = [r["divergence"] for r in att if r["verdict"] == "passed"]
        lr = [r["length_ratio"] for r in att if r["verdict"] == "passed"]
        rej = {}
        for r in att:
            if r["verdict"] != "passed":
                rej[r["verdict"]] = rej.get(r["verdict"], 0) + 1
        jp = [judge[r["key"]] for r in att if r["verdict"] == "passed" and r["key"] in judge]
        rej["judged CHANGED (passed)"] = f"{sum(v == 'CHANGED' for v in jp)}/{len(jp)}"
        passed_info.append((variant, len(att), len(passed), quart(passed), quart(lr), rej))
        for name, most, floor, single, cap in POLICIES:
            if variant not in ("default", "voice") and name not in ("max>=0.2", "min>=0.2"):
                continue
            if variant == "voice" and name not in ("max>=0.2", "min>=0.2", "max>=0.2, len<=1.3"):
                continue
            rows = []
            for x in g.values():
                w = pick(x, most, floor, single, cap)
                src = next(iter(x.values()))["chunk_text"]
                rows.append((src, w["answer"] if w else None, w["divergence"] if w else None, w["key"] if w else None))
            table.append((variant, name, summarise(rows, judge)))
            samples.append((f"{variant} / {name}", sample_rows([r[0] for r in rows], [r[1] for r in rows])))

    if base_chunks and a.app_json and a.app_out:
        rep = json.load(open(a.app_json, encoding="utf-8"))["report"]["best_effort"]
        divs = []
        for c in rep["chunks"]:
            w = c["outcome"].get("rewritten")
            for at in c["attempts"]:
                v = at["verdict"]
                if w and "passed" in v and at["round"] == w["round"] and at["candidate"] == w["candidate"]:
                    divs.append(v["passed"]["divergence"])
        doc = pathlib.Path(a.app_out).read_text(encoding="utf-8")
        chosen = align(base_chunks, doc)
        rows = [(s, strip_markup(c) if c else None, None, None) for s, c in zip(base_chunks, chosen)]
        r = summarise(rows, {})
        r["div"] = quart(divs) + " (report)"
        r["rewritten"] = sum(1 for c in rep["chunks"] if c["outcome"].get("rewritten"))
        r["kept"] = len(rep["chunks"]) - r["rewritten"]
        table.insert(0, ("app run (seed 4071544710)", "max>=0.2", r))
        samples.insert(0, ("app run / max>=0.2", sample_rows(base_chunks, chosen)))

    if base_chunks and (runs / "whole.jsonl").exists():
        for rec in load(runs / "whole.jsonl"):
            chosen = align(base_chunks, rec["answer"])
            rows = []
            divs = []
            for s, c in zip(base_chunks, chosen):
                cw = strip_markup(c) if c else None
                rows.append((s, cw, None, None))
                if cw is not None:
                    divs.append(divergence(words(s), words(cw)))
            r = summarise(rows, {})
            r["div"] = quart(divs) + " (aligned)"
            r["rewritten"] = "1 call"
            r["kept"] = "–"
            sw, aw = words(rec["source"]), words(rec["answer"])
            r["doc"] = f"whole doc: words x{len(aw) / len(sw):.2f}, you {you(sw)}→{you(aw)}, {rec['tokens_out']} tok, {rec['finish']}"
            table.append((f"whole (seed {rec['seed']})", "1 x 1, no pick", r))
            samples.append((f"whole seed {rec['seed']}", sample_rows(base_chunks, chosen)))

    you_src = table[0][2]["you_src"] if table else 0
    print(f"Prose chunks: {len(base_chunks or [])}; second-person words in them: {you_src}.\n")
    print("| run | policy | rewritten / kept | divergence med [q1–q3] | words × | pairs left | you | judged CHANGED |")
    print("|---|---|---|---|---|---|---|---|")
    for variant, name, r in table:
        print(f"| {variant} | {name} | {r['rewritten']} / {r['kept']} | {r['div']} | {r['words']:.2f} | {r['pairs_left']:.2f} | {r['you']} | {r['judged']} |")
    for variant, name, r in table:
        if "doc" in r:
            print(f"\n{variant}: {r['doc']}")
    if calib:
        print("\nJudge calibration (right / asked): " + ", ".join(f"{k} {v[0]}/{v[1]}" for k, v in sorted(calib.items())))
    print("\nAttempts per run (all four candidates of every chunk):\n")
    print("| run | attempts | passed | divergence of passed | length ratio of passed | rejected |")
    print("|---|---|---|---|---|---|")
    for v, n, np_, dq, lq, rej in passed_info:
        print(f"| {v} | {n} | {np_} | {dq} | {lq} | {', '.join(f'{k} {c}' for k, c in sorted(rej.items()))} |")
    print("\n## Samples\n")
    for label, _ in SAMPLES:
        print(f"### {label}\n")
        src = None
        for _, s in samples:
            if label in s:
                src = s[label][0]
                break
        if src:
            print(f"**source** — {src.strip()}\n")
        for name, s in samples:
            if label in s:
                ans = s[label][1]
                print(f"**{name}** — {ans.strip() if ans else '(kept as it was)'}\n")


if __name__ == "__main__":
    main()
