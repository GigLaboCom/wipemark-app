#!/usr/bin/env python3
# A quick reading of the E4-8 voice run while it is still going — the numbers behind each
# intermediate report the coordinator posts to Watchword.
#
# What it is for
#   The owner, 2026-10-10, asked for intermediate reports as the voice run
#   (scripts/verify/e4-8/host-voice-run.sh → crates/wipemark-pipeline/bench/run-voice.sh) finishes
#   each part. `bench report` reads every run and the judge at the end; this reads only the run
#   records that exist, needs no build and no model, and says what each part did so far. It is not
#   the bench's report and its figures are not the bench's: no judge, no loop selection.
#
# What it does
#   For each runs/<name>.jsonl under the run's directory, over every attempt:
#   1. how many, how many passed, and the refusals by kind (and by guard for a guard);
#   2. the mean divergence and length ratio of the attempts that passed;
#   3. the voice, over attempts that passed whose source had the feature: the share that kept
#      the second person (source > 0 → answer > 0), the share that kept the first person, the
#      share whose formal/informal address switched (`switched` set), and the mean register shift;
#   4. one table, the shipped templates' row beside the +voice row of the same model.
#
# How to run
#   python3 -I scripts/verify/e4-8/progress.py crates/wipemark-pipeline/bench/results/voice-2026-10-10
#
# What it needs
#   Python 3 alone.
#
# What its output means
#   A Markdown table per question. "kept 2nd" is the measure keep-voice exists for: a rewrite that
#   turns "you" into an impersonal sentence loses it. Read it beside "passed" — a voice kept by
#   refusing more answers is no gain.
import collections
import json
import pathlib
import sys

run_dir = pathlib.Path(sys.argv[1])
rows_of = {}
for path in sorted((run_dir / "runs").glob("*.jsonl")):
    rows = []
    for line in path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if line:
            rows.append(json.loads(line))
    rows_of[path.stem] = rows


def share(hits, total):
    return f"{100 * hits / total:.0f}% of {total}" if total else "—"


print("| run | attempts | passed | refused (kind/guard: n) | divergence | length | kept 2nd | kept 1st | address switched | register shift |")
print("|---|---|---|---|---|---|---|---|---|---|")
for name in sorted(rows_of, key=lambda n: (n.replace("+voice", ""), "+voice" in n)):
    rows = rows_of[name]
    passed = [r for r in rows if r["verdict"] == "passed"]
    refused = collections.Counter()
    for r in rows:
        rej = r.get("rejection")
        if rej:
            refused[rej.get("guard") or rej.get("kind")] += 1
    div = sum(r["divergence"] for r in passed) / len(passed) if passed else 0
    length = sum(r["length_ratio"] for r in passed) / len(passed) if passed else 0
    second = [r for r in passed if r["voice"]["second"][0] > 0]
    first = [r for r in passed if r["voice"]["first"][0] > 0]
    kept2 = sum(1 for r in second if r["voice"]["second"][1] > 0)
    kept1 = sum(1 for r in first if r["voice"]["first"][1] > 0)
    addressed = [r for r in passed if r["voice"]["formal"][0] > 0 or r["voice"]["switched"] is not None]
    switched = sum(1 for r in addressed if r["voice"]["switched"])
    shift = sum(r["voice"]["register_shift"] for r in passed) / len(passed) if passed else 0
    refusals = ", ".join(f"{k}: {v}" for k, v in refused.most_common()) or "—"
    print(
        f"| {name} | {len(rows)} | {share(len(passed), len(rows))} | {refusals} | {div:.2f} | {length:.2f} "
        f"| {share(kept2, len(second))} | {share(kept1, len(first))} | {share(switched, len(addressed))} | {shift:.3f} |"
    )
