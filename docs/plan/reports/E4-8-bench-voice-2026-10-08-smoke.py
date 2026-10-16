#!/usr/bin/env python3
"""The voice bench end to end against a fake endpoint — run, judge, report.

What it is for
    The coordinator's task docs/plan/E4-8-bench-voice.md (Watchword
    `wipemark-task-bench-voice-2026-10-08`, written 2026-10-08 for the owner)
    was built in a container with no model and no GPU, and the four-model run
    (`crates/wipemark-pipeline/bench/run-voice.sh`) is the owner's. The unit
    tests hold each piece; this script holds the road between them: that a
    `bench run` record carries its `voice`, that `--variant keep-voice` puts
    the rule in front of the model in every language and only there, that the
    judge asks both questions and resumes, that `bench plan` counts what the
    parts then make, and that `bench report` prints the voice table — for new
    records and for records with the new fields taken out. Kept because a
    claim no script can reproduce is a claim nobody can check (CLAUDE.md).

What it does
    1. Starts an OpenAI-compatible server on 127.0.0.1 (a thread here). A
       rewrite request is answered by rule from its seed: an even seed
       switches the address (you -> one, вы -> ты, Sie -> du) and an odd one
       keeps it, each with a filler word after every third word so the no-op
       floor passes; a meaning question is answered EQUIVALENT, a voice
       question NO when the rewrite switched and YES when it did not.
    2. `bench plan --of run`, then `bench run`, for the shipped templates
       (`--name fake`) and keep-voice (`--name fake+voice --variant …`) over
       en-mx-01, ru-mx-01 and de-mx-01, grid paraphrase/humanize moderate.
    3. `bench plan --of judge`, `bench judge` (twice: the second makes
       nothing), `bench report` over the records, and again over the records
       with `voice` removed and the judge's voice lines dropped.
    Every check prints PASS or FAIL; the exit status is the number failed.

How to run
    From the repository root, on `e4/bench-voice`:
        python3 -I docs/plan/reports/E4-8-bench-voice-2026-10-08-smoke.py [--show]
    `--show` also prints the voice table the report made. `CARGO_TARGET_DIR`
    is honoured. Writes only under a temporary directory.

What it needs
    Python 3 (standard library), cargo; the bench builds with
    `--features local-llama` (the shim: no llama.cpp, no model).

What its output means
    One line per check. All PASS: the plumbing between run, judge and report
    holds on this commit. It says nothing about any model's voice — that is
    the host's run.
"""

import http.server
import json
import pathlib
import re
import subprocess
import sys
import tempfile
import threading

ROOT = pathlib.Path(__file__).resolve().parents[3]
BENCH = ["cargo", "run", "-q", "--locked", "-p", "wipemark-pipeline", "--features", "local-llama",
         "--example", "bench", "--"]
ITEMS = "en-mx-01,ru-mx-01,de-mx-01"
GRID = "paraphrase:moderate:4;humanize:moderate:2"
RULES = ["Keep the author's voice", "Сохраняй голос автора", "Bewahre die Stimme des Autors"]
SWITCH = [(r"\bYour\b", "One's"), (r"\byour\b", "one's"), (r"\bYou\b", "One"), (r"\byou\b", "one"),
          (r"\bвас\b", "тебя"), (r"\bвам\b", "тебе"), (r"\bвы\b", "ты"), (r"\bВы\b", "Ты"),
          (r"\bваш\b", "твой"), (r"\bIhre\b", "deine"),
          (r"\bIhren\b", "deinen"), (r"\bIhnen\b", "dir"), (r"(?<=\w )Sie\b", "du")]
REQUESTS = []
FAILED = []


def check(name, ok, detail=""):
    print(f"{'PASS' if ok else 'FAIL'} {name}{': ' + detail if detail and not ok else ''}", flush=True)
    if not ok:
        FAILED.append(name)


def rewrite(text, seed):
    filler = "правда" if re.search("[а-яё]", text, re.I) else (
        "wirklich" if re.search(r"\b(und|der|die|das|wir|Sie)\b", text) else "really")
    if seed % 2 == 0:
        for pattern, to in SWITCH:
            text = re.sub(pattern, to, text)
    out = []
    for n, word in enumerate(text.split(" "), 1):
        out.append(word)
        if n % 3 == 0 and re.fullmatch(r"\w+", word):
            out.append(filler)
    return " ".join(out)


class Fake(http.server.BaseHTTPRequestHandler):
    def log_message(self, *_):
        pass

    def do_POST(self):
        body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        system = next((m["content"] for m in body["messages"] if m["role"] == "system"), "")
        user = next(m["content"] for m in body["messages"] if m["role"] == "user")
        REQUESTS.append({"model": body["model"], "system": system, "user": user})
        if system.startswith("You compare two texts for meaning"):
            answer = "EQUIVALENT"
        elif system.startswith("You compare the voice"):
            b = user.split("B:\n<<<\n", 1)[1]
            answer = "NO" if re.search(r"\b(One|one|ты|тебя|du|dir)\b", b) else "YES"
        else:
            text = user.rsplit("[[[BEGIN TEXT]]]\n", 1)[1].split("\n[[[END TEXT]]]", 1)[0]
            answer = rewrite(text, body.get("seed", 0))
        events = [{"choices": [{"index": 0, "delta": {"content": answer}}]},
                  {"choices": [{"index": 0, "delta": {}, "finish_reason": "stop"}],
                   "usage": {"completion_tokens": len(answer.split())}}]
        payload = "".join(f"data: {json.dumps(e)}\n\n" for e in events) + "data: [DONE]\n\n"
        data = payload.encode()
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)


def bench(*args, stdout=None):
    done = subprocess.run(BENCH + list(args), cwd=ROOT, capture_output=True, text=True)
    if done.returncode != 0:
        print(done.stderr[-3000:], file=sys.stderr)
        raise SystemExit(f"bench {args[0]} failed")
    return done.stdout


def lines(path):
    return [json.loads(line) for line in pathlib.Path(path).read_text().splitlines() if line.strip()]


def plan(args):
    out = bench("plan", *args)
    return {k: int(v) for k, v in re.findall(r"(\w+)=(\d+)", out)}


def main():
    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Fake)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    endpoint = f"http://127.0.0.1:{server.server_address[1]}"
    bench("plan", "--of", "run", "--endpoint", endpoint, "--name", "warm", "--out", "/dev/null")  # build once
    with tempfile.TemporaryDirectory(prefix="wipemark-e4-8-smoke-") as tmp:
        tmp = pathlib.Path(tmp)
        runs = []
        for name, variant in [("fake", []), ("fake+voice", ["--variant", "crates/wipemark-pipeline/bench/variants/keep-voice"])]:
            out = tmp / f"{name}.jsonl"
            args = ["--endpoint", endpoint, "--name", name, "--out", str(out), "--items", ITEMS, "--grid", GRID] + variant
            planned = plan(["--of", "run"] + args)
            bench("run", *args)
            records = lines(out)
            check(f"{name}: run makes what plan counted", len(records) == planned["attempts"],
                  f"{len(records)} vs {planned}")
            answered = [r for r in records if isinstance(r.get("answer"), str)]
            check(f"{name}: every answered record carries its voice",
                  answered and all(isinstance(r.get("voice"), dict) and "second" in r["voice"] for r in answered))
            check(f"{name}: the second person was seen", any(r["voice"]["second"][0] > 0 for r in answered))
            again = plan(["--of", "run"] + args)
            check(f"{name}: a finished run plans nothing", again["attempts"] == 0 and again["done"] == len(records))
            runs.append(str(out))

        rewrites = [r for r in REQUESTS if r["model"] in ("fake", "fake+voice")]
        voiced = [r for r in rewrites if r["model"] == "fake+voice"]
        plain = [r for r in rewrites if r["model"] == "fake"]
        check("keep-voice: the rule reached the model on every request of the variant",
              voiced and all(any(rule in r["system"] for rule in RULES) for r in voiced))
        check("keep-voice: in each language", all(any(rule in r["system"] for r in voiced) for rule in RULES))
        check("the shipped templates carry no voice rule",
              plain and not any(any(rule in r["system"] for rule in RULES) for r in plain))

        judge = tmp / "judge.jsonl"
        jargs = ["--endpoint", endpoint, "--name", "judge", "--in", ",".join(runs), "--out", str(judge)]
        planned = plan(["--of", "judge"] + jargs)
        bench("judge", *jargs)
        js = lines(judge)
        check("judge makes what plan counted", len(js) == planned["calls"], f"{len(js)} vs {planned}")
        meaning = [j for j in js if "of" in j and "verdict" in j]
        voice = [j for j in js if "of" in j and "voice" in j]
        check("each judged attempt has a meaning line and a voice line",
              meaning and sorted(j["of"] for j in meaning) == sorted(j["of"] for j in voice))
        check("the meaning lines are E4-5's shape", all(set(j) == {"key", "of", "judge", "verdict", "secs"} for j in meaning))
        check("the voice question was calibrated", any(j.get("voice_calib") == "same" for j in js))
        bench("judge", *jargs)
        check("a judged file is not judged again", len(lines(judge)) == len(js))

        summary = tmp / "summary.json"
        md = bench("report", "--in", ",".join(runs), "--judge", str(judge), "--summary", str(summary))
        table = md.split("\n### Voice", 1)[1].split("\n### ", 1)[0] if "\n### Voice" in md else ""
        rows = [line for line in table.splitlines() if line.startswith("| fake")]
        if "--show" in sys.argv[1:]:
            print("### Voice" + table)
        check("report prints the voice table, both runs, both picks", len(rows) == 8, f"{len(rows)} rows")
        check("the judge's voice answers are in it", rows and all(re.search(r"of \d+ \|$", row) for row in rows))
        check("the summary carries the voice", '"voice":' in summary.read_text())

        old = []
        for path in runs:
            stripped = tmp / ("old-" + pathlib.Path(path).name)
            stripped.write_text("".join(json.dumps({k: v for k, v in r.items() if k != "voice"}) + "\n" for r in lines(path)))
            old.append(str(stripped))
        old_judge = tmp / "old-judge.jsonl"
        old_judge.write_text("".join(json.dumps(j) + "\n" for j in js if "voice" not in j))
        old_md = bench("report", "--in", ",".join(old), "--judge", str(old_judge))
        old_table = old_md.split("\n### Voice", 1)[1].split("\n### ", 1)[0] if "\n### Voice" in old_md else ""
        old_rows = [line for line in old_table.splitlines() if line.startswith("| fake")]
        check("records without the new fields still report", len(old_rows) == 8)
        check("…with the same voice measures, recomputed", [r.rsplit("|", 2)[0] for r in old_rows] == [r.rsplit("|", 2)[0] for r in rows])
        check("…and no judged voice", old_rows and all(row.endswith("| – |") for row in old_rows))
    server.shutdown()
    print(f"{len(FAILED)} failed" if FAILED else "all passed")
    return len(FAILED)


if __name__ == "__main__":
    sys.exit(main())
