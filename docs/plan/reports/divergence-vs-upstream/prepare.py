#!/usr/bin/env python3
"""Inputs for the divergence-vs-upstream study: a one-item bench corpus and
upstream's paraphrase prompt, as files the prompt bench reads.

What it is for
--------------
The owner asked on 2026-10-07 why a test rewrite (a 2 285-word Markdown
article, Qwen3.8 27B, paraphrase moderate) diverged so far from its source,
compared with upstream `guillaumemeyer/watermarks-remover`, whose Layer B
ours followed. The report is
`docs/plan/reports/divergence-vs-upstream-2026-10-07.md`. This script
builds what the bench needs to measure it; `run.sh` beside it runs the
bench, `analyse.py` makes the tables.

What it does
------------
1. Reads upstream's `service/scripts/rewrite_text.py` from a clone (under
   `tmp/`, gitignored) **as text**: it finds the `"paraphrase": ( ... )`
   entry of `PROMPTS` and evaluates that one parenthesised string literal
   with `ast.literal_eval` — nothing of upstream's is imported or run.
   It drops the `\\n\\n---\\n{TEXT}` tail and checks the rest is a sentence
   that ends "Output only the rewritten text."
2. Writes, under `--out`:
   * `corpus/en.txt` — the article as one bench item (`id: art01`, format
     markdown), and empty `ru.txt`, `de.txt` (the bench loads all three);
   * `article.md` — the article as it is, for `bench whole`;
   * `upstream-paraphrase.txt` — the instruction alone, for `bench whole`;
   * `variants/upstream-prompt/en/paraphrase.1.{system,user}.txt` — the
     instruction as our user template (`{INTENSITY}`, `{PREV_CONTEXT}` and
     `{TEXT}` after it, so only the wording differs from the shipped one),
     and a system turn that is only `{PROTECTED}` (upstream has no system
     turn; the placeholder sentence is the one line the validator requires);
   * `PROVENANCE.txt` — the upstream commit and the article's sha256.
   Nothing it writes is committed: the article is the owner's draft, and
   the prompt text is upstream's (MIT) — kept out of the tree rather than
   given a NOTICE entry for a measurement.

How to run
----------
    git clone https://github.com/guillaumemeyer/watermarks-remover tmp/watermarks-remover
    python3 -I docs/plan/reports/divergence-vs-upstream/prepare.py \\
        --upstream tmp/watermarks-remover \\
        --article ~/Downloads/article-01-screenshot-mcp-server.md \\
        --out tmp/divergence

Needs Python 3.9+ and git; nothing else.

Output
------
The files above, and one line per file on stdout. Exit 1 if upstream's
prompt cannot be found or no longer has the shape described in step 1
(the report then needs re-reading against the new upstream).
"""

import argparse
import ast
import hashlib
import pathlib
import re
import subprocess
import sys

TAIL = "\n\n---\n{TEXT}"


def upstream_paraphrase(clone: pathlib.Path) -> str:
    source = (clone / "service/scripts/rewrite_text.py").read_text(encoding="utf-8")
    start = source.find("PROMPTS = {")
    if start < 0:
        sys.exit("PROMPTS not found in rewrite_text.py")
    m = re.compile(r'"paraphrase":\s*(\((?:\s*"(?:[^"\\]|\\.)*")+\s*\))', re.S).search(source, start)
    if not m:
        sys.exit('PROMPTS["paraphrase"] not found as a parenthesised string literal')
    text = ast.literal_eval(m.group(1))
    if not text.endswith(TAIL):
        sys.exit("upstream's paraphrase prompt no longer ends with the ---/{TEXT} tail")
    text = text[: -len(TAIL)]
    if "{" in text or not text.endswith("Output only the rewritten text."):
        sys.exit("upstream's paraphrase prompt changed shape: " + text[-80:])
    return text


def main() -> None:
    p = argparse.ArgumentParser()
    p.add_argument("--upstream", required=True, type=pathlib.Path)
    p.add_argument("--article", required=True, type=pathlib.Path)
    p.add_argument("--out", required=True, type=pathlib.Path)
    a = p.parse_args()

    instruction = upstream_paraphrase(a.upstream)
    commit = subprocess.run(
        ["git", "-C", str(a.upstream), "log", "-1", "--format=%H %ci"],
        capture_output=True, text=True, check=True,
    ).stdout.strip()
    article = a.article.read_text(encoding="utf-8")
    if any(line.startswith("=== ") for line in article.splitlines()):
        sys.exit("the article has a line starting '=== ', which the corpus format reads as a header")

    out = a.out
    written = []

    def write(rel: str, text: str) -> None:
        path = out / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")
        written.append(path)

    header = (
        "=== id: art01 | kind: article | format: markdown"
        f" | source: {a.article.name} (the owner's draft) | licence: the owner's own text, not committed\n"
    )
    write("corpus/en.txt", header + article)
    write("article.md", article)
    write("corpus/ru.txt", "")
    write("corpus/de.txt", "")
    write("upstream-paraphrase.txt", instruction + "\n")
    write("variants/upstream-prompt/en/paraphrase.1.system.txt", "{PROTECTED}\n")
    write(
        "variants/upstream-prompt/en/paraphrase.1.user.txt",
        instruction + "\n{INTENSITY}\n\n{PREV_CONTEXT}\n\n{TEXT}\n",
    )
    write(
        "PROVENANCE.txt",
        f"upstream guillaumemeyer/watermarks-remover {commit}\n"
        f"article {a.article.name} sha256 {hashlib.sha256(article.encode()).hexdigest()}\n",
    )
    for path in written:
        print(path)


if __name__ == "__main__":
    main()
