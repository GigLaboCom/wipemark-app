#!/usr/bin/env python3
"""Gemma 4's catalogue pin against the repository's later re-upload: the
chat template each carries, read off the GGUF header by a range read.

What it is for
--------------
The catalogue pins `unsloth/gemma-4-12B-it-qat-GGUF` at `f18012b8` (D403),
the last commit whose file is the one the local engine was measured on
(E2-4). The repository's HEAD at E2-4, `980b060c` ("Added Gemma official
chat template update", 2026-07-17), re-uploaded the file with another chat
template, and that re-upload was never checked (E2-4's open question 1).
The coordinator asked (task `wipemark-task-models-pipeline-followups-
2026-10-09`, M9, 2026-10-09) for the facts that decide whether to move the
pin: the commits, the file at each, the two templates and their difference,
and whether `wipemark_llama::chat` still recognises and renders the new one.
It changes nothing: moving the pin needs the host's live gate on the real
file.

What it does
------------
1. Lists the repository's commits on `main`
   (`GET /api/models/<repo>/commits/main`).
2. Lists the tree at the pinned commit and at HEAD
   (`GET /api/models/<repo>/tree/<rev>`), with each GGUF's LFS sha256 and
   size; the pinned file's figures are held to the manifest's.
3. Reads `tokenizer.chat_template` out of the pinned file and HEAD's by HTTP
   range reads of the **header only** (`/resolve/<rev>/<file>`, `Range:
   bytes=0-N`): 16 MiB asked for first, doubled while the key/value section
   is not complete, never past 64 MiB. The header is walked straight off the
   response, a value at a time, and the connection is closed at the last
   key: what is consumed is the key/value section to the byte, never a
   tensor description and never a tensor's byte. Nothing is kept but the
   three strings it wants; nothing is written under a models folder.
4. Prints each template's sha256 and length, a unified diff of the two, and
   what `wipemark_llama::chat::family_of` makes of each — the markers it
   reads (`<|turn>` and `<turn|>` for the family; the empty thought channel
   `<|channel>thought\\n<channel|>`, spelled as the Jinja string literal, for
   the 12B's closed opener) and the generation prompt the renderer writes
   (`<|turn>model\\n`). `crates/wipemark-llama/src/chat.rs`, `family_of` and
   `gemma_4_is_rendered_as_its_template_renders_it`, are the other side.

How to run
----------
    python3 -I scripts/verify/models/gemma4-template.py [--repo R] [--pin REV]
        [--file NAME] [--head REV] [--out DIR]

`--out DIR` also writes the two templates as text files into DIR (a scratch
directory; never a models folder). The defaults are the catalogue's.

What it needs
-------------
Python 3.9+, the standard library only, and HTTPS to huggingface.co (the
range reads follow its redirect to the CDN). No account: the repository is
not gated. About 32 MiB is read in all.

What its output means
---------------------
`template: identical` — the re-upload kept the template, and the pin can move
on the size and sha256 alone, after the live gate. Otherwise the diff is the
change, and `family_of` / `thought_closed` say whether the renderer still
recognises it and writes the same opener; a change inside the turns the
renderer writes (not only around tools, images or thinking on) needs
`POST /apply-template` against llama.cpp's Jinja engine before the pin moves.
Exit 0 when every read succeeded, 1 when the pinned file's figures are not
the manifest's, 2 on a network or format error.
"""

import argparse
import difflib
import hashlib
import json
import struct
import sys
import urllib.request

HF = "https://huggingface.co"
REPO = "unsloth/gemma-4-12B-it-qat-GGUF"
PIN = "f18012b8f690e563b7f872cb764b4cb3de90b14a"
FILE = "gemma-4-12B-it-qat-UD-Q4_K_XL.gguf"
# The manifest's figures for the pinned file (manifests/models.v1.json).
PIN_SHA256 = "cc9ff072e0a8203429ed854e6662c17a6c2bc1e5dca5b475dd4736caaacbc165"
PIN_SIZE = 6716355328

FIRST = 16 << 20
LIMIT = 64 << 20

# What wipemark_llama::chat::family_of reads (chat.rs), as the template
# spells it: a Jinja string literal, so `\n` is a backslash and an `n`.
TURN_OPEN = "<|turn>"
TURN_CLOSE = "<turn|>"
THOUGHT_CLOSED = "<|channel>thought\\n<channel|>"
MODEL_OPENER = "<|turn>model\\n"

USER_AGENT = "wipemark-verify/gemma4-template (stdlib urllib)"


def get_json(url):
    request = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
    with urllib.request.urlopen(request, timeout=60) as answer:
        return json.load(answer)


class Short(Exception):
    """The range ends before the key/value section does."""


class Walker:
    """A GGUF header read off `source` (a response), a value at a time:
    exactly the bytes walked are consumed, and none past `limit`."""

    def __init__(self, source, limit):
        self.source = source
        self.limit = limit
        self.at = 0

    def take(self, n):
        if self.at + n > self.limit:
            raise Short()
        out = self.source.read(n)
        if len(out) < n:
            raise Short()
        self.at += n
        return out

    def u32(self):
        return struct.unpack("<I", self.take(4))[0]

    def u64(self):
        return struct.unpack("<Q", self.take(8))[0]

    def string(self):
        return self.take(self.u64())

    SIZES = {0: 1, 1: 1, 2: 2, 3: 2, 4: 4, 5: 4, 6: 4, 7: 1, 10: 8, 11: 8, 12: 8}

    def skip_value(self, kind):
        if kind in self.SIZES:
            self.take(self.SIZES[kind])
        elif kind == 8:
            self.string()
        elif kind == 9:
            inner = self.u32()
            count = self.u64()
            if inner == 8:
                for _ in range(count):
                    self.string()
            elif inner in self.SIZES:
                self.take(self.SIZES[inner] * count)
            else:
                raise ValueError("an array of type %d" % inner)
        else:
            raise ValueError("a value of type %d" % kind)


def header_of(source, limit):
    """The GGUF header's keys this script wants, or Short when cut."""
    walk = Walker(source, limit)
    if walk.take(4) != b"GGUF":
        raise ValueError("not a GGUF file")
    version = walk.u32()
    if version not in (2, 3):
        raise ValueError("GGUF version %d" % version)
    tensors = walk.u64()
    keys = walk.u64()
    found = {"version": version, "tensors": tensors, "keys": keys}
    for _ in range(keys):
        key = walk.string().decode("utf-8", "replace")
        kind = walk.u32()
        if kind == 8 and key in ("tokenizer.chat_template", "general.name", "general.architecture"):
            found[key] = walk.string().decode("utf-8")
        else:
            walk.skip_value(kind)
    found["kv_end"] = walk.at
    return found


def read_header(url):
    """The header of the file at `url`, by range reads: 16 MiB asked for,
    then 32, then 64, the walk starting again each time."""
    length = FIRST
    while True:
        request = urllib.request.Request(
            url,
            headers={"User-Agent": USER_AGENT, "Range": "bytes=0-%d" % (length - 1)},
        )
        with urllib.request.urlopen(request, timeout=300) as answer:
            if answer.status != 206:
                # A server that ignored the range would be sending the whole
                # file: nothing is read of it.
                raise RuntimeError("HTTP %d to a range request at %s" % (answer.status, url))
            try:
                header = header_of(answer, length)
            except Short:
                header = None
        if header is not None:
            header["asked_bytes"] = length
            return header
        if length >= LIMIT:
            raise RuntimeError("the key/value section runs past %d MiB" % (LIMIT >> 20))
        length = min(length * 2, LIMIT)


def recognised(template):
    family = TURN_OPEN in template and TURN_CLOSE in template
    return {
        "gemma4 family (<|turn> and <turn|>)": family,
        "thought channel closed (12B opener)": family and THOUGHT_CLOSED in template,
        "generation prompt <|turn>model\\n": MODEL_OPENER in template,
    }


def gguf_files(repo, rev):
    tree = get_json("%s/api/models/%s/tree/%s" % (HF, repo, rev))
    out = {}
    for entry in tree:
        if entry.get("type") == "file" and entry["path"].endswith(".gguf"):
            lfs = entry.get("lfs") or {}
            out[entry["path"]] = (lfs.get("oid"), lfs.get("size", entry.get("size")))
    return out


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--repo", default=REPO)
    parser.add_argument("--pin", default=PIN)
    parser.add_argument("--file", default=FILE)
    parser.add_argument("--head", default=None, help="a revision to compare (default: main's newest)")
    parser.add_argument("--out", default=None, help="write the two templates here")
    args = parser.parse_args()

    try:
        commits = get_json("%s/api/models/%s/commits/main" % (HF, args.repo))
    except Exception as error:  # noqa: BLE001 — any failure is "no answer"
        print("could not list the commits: %s" % error)
        return 2
    print("commits on main, newest first:")
    for commit in commits:
        print("  %s  %s  %s" % (commit["id"][:12], commit.get("date", "?")[:19], commit.get("title", "").strip()))
    head = args.head or commits[0]["id"]
    print("pin  %s\nhead %s" % (args.pin, head))

    status = 0
    files = {}
    for label, rev in (("pin", args.pin), ("head", head)):
        listed = gguf_files(args.repo, rev)
        files[label] = listed.get(args.file)
        print("\n%s: %d GGUF files at %s" % (label, len(listed), rev[:12]))
        for name, (oid, size) in sorted(listed.items()):
            mark = "  <-" if name == args.file else ""
            print("  %-48s %14s  %s%s" % (name, "{:,}".format(size or 0).replace(",", " "), oid, mark))
    if files["pin"] != (PIN_SHA256, PIN_SIZE) and args.pin == PIN and args.file == FILE:
        print("\nthe pinned file is not the manifest's: %r" % (files["pin"],))
        status = 1
    if files["head"] is None:
        print("\n%s is not at HEAD" % args.file)
        return 2
    print(
        "\nthe file at HEAD is %s the pinned one"
        % ("the same as" if files["head"] == files["pin"] else "another file than")
    )

    templates = {}
    for label, rev in (("pin", args.pin), ("head", head)):
        url = "%s/%s/resolve/%s/%s" % (HF, args.repo, rev, args.file)
        try:
            header = read_header(url)
        except Exception as error:  # noqa: BLE001
            print("could not read %s's header: %s" % (label, error))
            return 2
        template = header.get("tokenizer.chat_template")
        templates[label] = template or ""
        print(
            "\n%s header: GGUF v%d, %d tensors, %d keys, key/value section %d bytes "
            "(consumed exactly that, of a %d MiB range), architecture %s, name %r"
            % (
                label,
                header["version"],
                header["tensors"],
                header["keys"],
                header["kv_end"],
                header["asked_bytes"] >> 20,
                header.get("general.architecture"),
                header.get("general.name"),
            )
        )
        if template is None:
            print("  no tokenizer.chat_template")
            continue
        digest = hashlib.sha256(template.encode("utf-8")).hexdigest()
        print("  template: %d bytes, sha256 %s" % (len(template.encode("utf-8")), digest))
        for what, holds in recognised(template).items():
            print("  %-40s %s" % (what, "yes" if holds else "NO"))
        if args.out:
            path = "%s/%s-%s.jinja" % (args.out, label, rev[:12])
            with open(path, "w", encoding="utf-8") as out:
                out.write(template)
            print("  written to %s" % path)

    if templates["pin"] == templates["head"]:
        print("\ntemplate: identical")
    else:
        print("\ntemplate: changed — unified diff, pin → head:")
        sys.stdout.writelines(
            difflib.unified_diff(
                templates["pin"].splitlines(keepends=True),
                templates["head"].splitlines(keepends=True),
                fromfile="pin/" + args.pin[:12],
                tofile="head/" + head[:12],
            )
        )
    return status


if __name__ == "__main__":
    sys.exit(main())
