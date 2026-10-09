#!/usr/bin/env python3
"""The red checks of E4-8, the prompt bench's voice (2026-10-08), as they were run.

What it is for
    The coordinator's task docs/plan/E4-8-bench-voice.md (Watchword
    `wipemark-task-bench-voice-2026-10-08`, written 2026-10-08 for the owner)
    asks that each protection it adds be shown red once without it — no
    mutation tables (`wipemark-mutations-not-needed-2026-10-06`) — and that
    each be recorded in a script beside the report with the header CLAUDE.md
    asks for. This is that script; the report is
    `E4-8-bench-voice-2026-10-08.md` beside it. It was run while the round
    was written; it is kept because a claim no script can reproduce is a
    claim nobody can check.

What it does
    For each named check: replace exact pieces of the repository (the
    protection) with the code, data or script line without it — or with the
    regression it prevents — run the named command, print what failed, and
    put every file back byte for byte, also when the run is interrupted.
    The command is a `cargo test` filter (the bench's unit tests run with
    `--features local-llama --examples`; the variant walk and the script's
    tests are the library's integration tests), the V4 lint itself, or the
    end-to-end smoke beside this script.

How to run
    From the repository root, on `e4/bench-voice`:
        python3 -I docs/plan/reports/E4-8-bench-voice-2026-10-08-red.py           # all
        python3 -I docs/plan/reports/E4-8-bench-voice-2026-10-08-red.py V1-sie V3-refuse
        python3 -I docs/plan/reports/E4-8-bench-voice-2026-10-08-red.py --check   # pieces only
    `CARGO_TARGET_DIR` is honoured.

What it needs
    Python 3 (standard library), cargo, bash. Nothing needs llama.cpp or a
    model: the bench builds over the `local-llama` shim. About a minute the
    first time, seconds a check after.

What the output means
    One line per check: RED with what failed (the protection is guarded),
    GREEN (it is not — the test does not fail without it), BROKEN (the
    revert did not build, or the filter matched no test: no verdict), or
    MISSING (the source moved and the piece is no longer there — update the
    entry).
"""

import re
import subprocess
import sys

BENCH = ["cargo", "test", "--locked", "-p", "wipemark-pipeline", "--features", "local-llama",
         "--examples", "--"]
VARIANTS_TEST = ["cargo", "test", "--locked", "-p", "wipemark-pipeline", "--test", "bench_variants", "--"]
SCRIPT_TEST = ["cargo", "test", "--locked", "-p", "wipemark-pipeline", "--test", "bench_run_voice", "--"]
LINT = ["cargo", "clippy", "--locked", "-p", "wipemark-pipeline", "--features", "local-llama",
        "--examples", "--", "-D", "warnings"]
SMOKE = ["python3", "-I", "docs/plan/reports/E4-8-bench-voice-2026-10-08-smoke.py"]

MEASURE = "crates/wipemark-pipeline/examples/bench/measure.rs"
ANALYSE = "crates/wipemark-pipeline/examples/bench/analyse.rs"
JUDGE = "crates/wipemark-pipeline/examples/bench/judge.rs"
RUN = "crates/wipemark-pipeline/examples/bench/run.rs"
VARIANT = "crates/wipemark-pipeline/examples/bench/variant.rs"
EN_LIST = "crates/wipemark-pipeline/bench/register/en.txt"
KV = "crates/wipemark-pipeline/bench/variants/keep-voice"
SCRIPT = "crates/wipemark-pipeline/bench/run-voice.sh"

# name: ([(file, the protection as it is, what it becomes), ...], command)
CHECKS = {
    # V1 (D420): a German "Sie" that opens a sentence is "she" or "they"
    # unless the pair says formal. Count every capitalised Sie.
    "V1-sie": (
        [(MEASURE, "            if !opens || formal_here {\n", "            if true {\n")],
        BENCH + ["measure::tests::german_sie"],
    ),
    # V1 (D420): bare "ihr" is her/their. Count it as the plural you.
    "V1-ihr": (
        [(MEASURE, '        "du", "dich", "dir",', '        "ihr", "du", "dich", "dir",')],
        BENCH + ["measure::tests::german_sie"],
    ),
    # V1 (D420): English "I" only as written, "US" never — take the case away.
    "V1-us": (
        [(MEASURE, '        if lang == Lang::En && (word == "I" || word == "US") {\n',
          '        if lang == Lang::En && false {\n')],
        BENCH + ["measure::tests::english_counts"],
    ),
    # V1 (D420): the switch between ты/вы and du/Sie is its own figure.
    "V1-switch": (
        [(MEASURE, "            (_, true, false) => Some(a.formal > 0),\n",
          "            (_, true, false) => Some(false),\n")],
        BENCH + ["measure::tests::"],
    ),
    # V1 (D420): kept is at most all of it — min(answer, source).
    "V1-kept-capped": (
        [(MEASURE, "f64::from(answer.min(source)) / f64::from(source)",
          "f64::from(answer) / f64::from(source)")],
        BENCH + ["measure::tests::english_counts"],
    ),
    # V1: words x leaves the placeholders out.
    "V1-words-placeholders": (
        [(MEASURE, "        .filter(|w| !w.starts_with('\\u{27E6}'))\n        .collect()\n}\n\n/// The voice",
          "        .collect()\n}\n\n/// The voice")],
        BENCH + ["measure::tests::the_words_ratio"],
    ),
    # V1: an empty source has no ratio (not infinity).
    "V1-empty": (
        [(MEASURE, "        (self.words.0 > 0).then(|| f64::from(self.words.1) / f64::from(self.words.0))",
          "        Some(f64::from(self.words.1) / f64::from(self.words.0))")],
        BENCH + ["measure::tests::an_empty_text"],
    ),
    # V1 (D421): the source's own word in another form is not a shift.
    "V1-register-stem": (
        [(MEASURE, ".filter(|w| !stems.contains(stem(w)) && list.iter().any(|e| e.matches(w)))",
          ".filter(|w| list.iter().any(|e| e.matches(w)))")],
        BENCH + ["measure::tests::the_register_proxy"],
    ),
    # V1 (D421): the lists are data the tests read — take an entry out.
    "V1-register-list": (
        [(EN_LIST, "possess*\n", "")],
        BENCH + ["measure::tests::"],
    ),
    # V1 (D422): report recomputes the voice from the texts — read it from
    # the record instead, as if old records had to carry it.
    "V1-report-old-records": (
        [(ANALYSE, "fn voice_of(r: &Value) -> Option<Voice> {\n    Some(Voice::of(\n",
          "fn voice_of(r: &Value) -> Option<Voice> {\n    r[\"voice\"].as_object()?;\n    Some(Voice::of(\n")],
        BENCH + ["analyse::tests::records_made_before"],
    ),
    # V1: a chunk kept as it was keeps its voice — skip it instead.
    "V1-report-kept-chunk": (
        [(ANALYSE, "            let lang = p.lang?;\n", "            let lang = p.lang?;\n            p.winner?;\n")],
        BENCH + ["analyse::tests::a_chunk_kept"],
    ),
    # V1 (D429): a cell with two candidates shows its CPU 1 x 2 pick.
    "V1-report-cpu": (
        [(ANALYSE, 'voice_rows.extend(row("gpu-2x2").or_else(|| row("cpu-1x2")));',
          'voice_rows.extend(row("gpu-2x2"));')],
        BENCH + ["analyse::tests::a_cell_with_two_candidates"],
    ),
    # V1 (D423): the judge's voice answer is read from its own field.
    "V1-report-judged-voice": (
        [(ANALYSE, "    let voices = answers(judge::Question::Voice.field());",
          "    let voices = answers(judge::Question::Meaning.field());")],
        BENCH + ["analyse::tests::the_judges_voice"],
    ),
    # V1 (D423): the meaning question is E4-5's, word for word.
    "V1-meaning-unchanged": (
        [(JUDGE, "Differences in wording, word order, style and sentence boundaries do not matter. Answer EQUIVALENT or CHANGED.\",",
          "Differences in wording, word order and sentence boundaries do not matter; keep the voice. Answer EQUIVALENT or CHANGED.\",")],
        BENCH + ["judge::tests::the_meaning_question"],
    ),
    # V1 (D423): each question takes only its own answers.
    "V1-judge-answers": (
        [(JUDGE, 'Question::Voice => &["YES", "PARTLY", "NO"],', 'Question::Voice => &["YES", "PARTLY", "NO", "CHANGED"],')],
        BENCH + ["judge::tests::each_question"],
    ),
    # V1 (D423): an old judge file is asked the voice and never the meaning
    # again — give the voice line the meaning's key.
    "V1-judge-resume": (
        [(JUDGE, '[("judge", Question::Meaning), ("voice", Question::Voice)]',
          '[("judge", Question::Meaning), ("judge", Question::Voice)]')],
        BENCH + ["judge::tests::every_judged_attempt"],
    ),
    # V1 (D429): plan loads nothing — fall through to the run.
    "V1-plan-loads-nothing": (
        [(RUN, "    if plan_only {\n", "    if plan_only && false {\n")],
        BENCH + ["run::tests::plan_counts"],
    ),
    # V1: the record carries its voice — only the end-to-end smoke sees
    # `run`'s record, since an attempt needs an engine.
    "V1-record-voice": (
        [(RUN, '            "voice": record["answer"].as_str().map_or(Value::Null, |answer| {\n',
          '            "voice": record["answer"].as_str().filter(|_| false).map_or(Value::Null, |answer| {\n')],
        SMOKE,
    ),
    # V2 (D427): a variant is admitted as an edit would be — accept anything.
    "V2-admit": (
        [(VARIANT, "        if !admission.admitted() {\n", "        if false {\n")],
        VARIANTS_TEST + ["a_variant_the_product_would_refuse"],
    ),
    # V2 (D427): a directory that is not a language is refused, not skipped.
    "V2-strict-names": (
        [(VARIANT, "            .ok_or_else(|| Refused::Name(shown(&lang_dir)))?;\n",
          "            .unwrap_or(Lang::En);\n")],
        VARIANTS_TEST + ["a_variant_the_product_would_refuse"],
    ),
    # V2: a broken variant file — the walk refuses it.
    "V2-broken-file": (
        [(f"{KV}/ru/humanize.1.system.txt", "\n{PROTECTED}", "\n")],
        VARIANTS_TEST + ["every_variant_is_admitted"],
    ),
    # V2 (D424): keep-voice covers paraphrase and humanize in every language
    # — take the German humanize rule out.
    "V2-coverage": (
        [(f"{KV}/de/humanize.1.system.txt",
          "- Bewahre die Stimme des Autors: Sprich die Leser so an wie der Text – duzt er sie, dann duze auch du; siezt er sie, dann sieze auch du. Mach den Text nicht förmlicher und nicht länger.\n",
          "")],
        VARIANTS_TEST + ["keep_voice_is_the_shipped_contract"],
    ),
    # V3 (D426): no variable, no start.
    "V3-refuse": (
        [(SCRIPT, '[ -z "$missing" ] || die', 'true || die')],
        SCRIPT_TEST + ["it_refuses_to_start"],
    ),
    # V3: a dry run writes nothing.
    "V3-dry-writes-nothing": (
        [(SCRIPT, '  echo "# run-voice.sh --dry-run: nothing is built',
          '  mkdir -p "$OUT/runs"\n  echo "# run-voice.sh --dry-run: nothing is built')],
        SCRIPT_TEST + ["the_dry_run_prints"],
    ),
    # V3: cargo is offline — it downloads nothing.
    "V3-offline": (
        [(SCRIPT, "B=(cargo run -q --offline --locked", "B=(cargo run -q --locked")],
        SCRIPT_TEST + ["it_downloads_nothing"],
    ),
    # V3: the +voice parts run the variant.
    "V3-variant": (
        [(SCRIPT, '  if [ "$id" != "$1" ]; then printf \'%s\\n\' --variant "$VARIANT"; fi\n', "")],
        SCRIPT_TEST + ["the_dry_run_prints"],
    ),
    # V4 (D428): the gate's new lint fails on a lint in the bench.
    "V4-lint": (
        [(ANALYSE, "fn round3(v: f64) -> f64 {\n", "fn round3(v: f64) -> f64 {\n    let _copied = 1u8.clone();\n")],
        LINT,
    ),
}


def run(name):
    pieces, command = CHECKS[name]
    sources = {}
    for path, _, _ in pieces:
        if path not in sources:
            sources[path] = open(path, encoding="utf-8").read()
    changed = dict(sources)
    for path, old, new in pieces:
        if old not in changed[path]:
            return f"{name}: MISSING ({path})"
        changed[path] = changed[path].replace(old, new, 1)
    try:
        for path, text in changed.items():
            with open(path, "w", encoding="utf-8") as out:
                out.write(text)
        done = subprocess.run(command, capture_output=True, text=True)
    finally:
        for path, text in sources.items():
            with open(path, "w", encoding="utf-8") as out:
                out.write(text)
    output = done.stdout + done.stderr
    if command is LINT:
        lints = sorted(set(re.findall(r"^error: (.+)$", output, re.M)) - {"could not compile `wipemark-pipeline` (example \"bench\") due to 1 previous error"})
        return f"{name}: RED (clippy: {'; '.join(lints)})" if done.returncode else f"{name}: GREEN — the lint passed"
    if command is SMOKE:
        failed = [line[5:] for line in output.splitlines() if line.startswith("FAIL ")]
        if failed:
            return f"{name}: RED {'; '.join(failed)}"
        return f"{name}: GREEN — the smoke passed" if done.returncode == 0 else f"{name}: BROKEN (exit {done.returncode})"
    failed = [line.split()[1] for line in output.splitlines()
              if line.startswith("test ") and line.endswith("FAILED")]
    if failed:
        return f"{name}: RED {', '.join(failed)}"
    ran = sum(int(n) for n in re.findall(r"^running (\d+) tests?$", output, re.M))
    if done.returncode != 0:
        return f"{name}: BROKEN (did not build or did not run: exit {done.returncode})"
    if ran == 0:
        return f"{name}: BROKEN (the filter matched no test)"
    return f"{name}: GREEN — the protection is not guarded"


def present(name):
    """Whether every piece of `name` is in the repository as it is now."""
    pieces, _ = CHECKS[name]
    missing = [path for path, old, _ in pieces if old not in open(path, encoding="utf-8").read()]
    return f"{name}: {'MISSING (' + ', '.join(missing) + ')' if missing else 'present'}"


if __name__ == "__main__":
    names = [name for name in sys.argv[1:] if name != "--check"]
    unknown = [name for name in names if name not in CHECKS]
    if unknown:
        sys.exit(f"no such check: {', '.join(unknown)}")
    if "--check" in sys.argv[1:]:
        for name in names or CHECKS:
            print(present(name), flush=True)
    else:
        for name in names or CHECKS:
            print(run(name), flush=True)
