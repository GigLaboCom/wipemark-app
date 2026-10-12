#!/usr/bin/env python3
"""make-fixture.py — the long-paragraph documents Compare is checked and measured on (E7-11).

What it is for, and who asked
    The owner (2026-10-12, acceptance step A6) compares documents whose
    paragraphs are single very long lines; the coordinator's task
    `wipemark-task-compare-spacer-rows-2026-10-12` asks for a fixture with
    long paragraphs under `fixtures/`, for screenshots, and for a live resize
    measured on a 200-paragraph prose document and on an 8 MiB text. This
    writes them, deterministically, so every figure and picture can be made
    again.

What it does, step by step
    1. Builds prose from a fixed list of sentences with a seeded generator:
       `N` paragraphs, each one line of 3 to 9 sentences.
    2. Writes the rewrite beside it, paragraph for paragraph: about a third
       of the paragraphs get a sentence more (they wrap taller), about a
       fifth lose one (shorter), some have one word changed, one paragraph is
       put in after the fourth and one taken out near the middle — the
       shapes `Diff::padding` pairs (D670).
    3. `--size BYTES` instead repeats the original's paragraphs, each copy
       numbered so no two paragraphs are the same text (GPUI caches a shaped
       line by its text), until the text reaches that size, and cuts it back
       to no more than that (the 8 MiB measurement); the rewrite of that is
       written the same way.

How to run it (from the repository root)
    python3 scripts/verify/e7-11/make-fixture.py                       # the committed fixture
    python3 scripts/verify/e7-11/make-fixture.py --paragraphs 200 --out DIR
    python3 scripts/verify/e7-11/make-fixture.py --size 8388608 --out DIR
    Default: 60 paragraphs into fixtures/compare/long-paragraphs/.

What it needs
    Python 3 alone.

What its output means
    `original.txt` and `rewritten.txt` in the folder, and one line on stdout:
    their sizes, paragraph counts and the longest paragraph in characters.
"""

import argparse
import os
import random

SENTENCES = [
    "The harbour was quiet in the early hours, before the first ferry left",
    "Fishing boats came in one by one and tied up along the old stone wall",
    "Nobody spoke much, because the work was known and the light was coming",
    "By six the first crates were stacked on the quay, ice glittering between them",
    "The auctioneer cleared his throat for the first lot of the day",
    "Buyers stood with their hands in their pockets and pretended not to care",
    "A gull walked the length of the quay as if it owned every crate on it",
    "The bell at the end of the market rang twice, and the bidding began",
    "Prices were low for cod and high for anything that had come from deep water",
    "One of the older skippers kept a notebook of every price since the war",
    "He said the numbers told him more about the sea than the forecast ever did",
    "The fog lifted slowly from the water, first from the boats and then from the town",
    "Children on their way to school stopped to watch the crates being weighed",
    "A lorry from the city waited at the gate with its engine running",
    "The harbour master counted the boats twice and wrote the total on a board",
    "Rain was promised for the afternoon, so everyone worked a little faster",
    "The café by the slipway opened its shutters and the smell of coffee drifted out",
    "Two men argued about a net that had been mended badly the week before",
    "It was, everyone agreed later, a good morning for the whole season",
    "The tide turned just after nine, and the water began to creep up the wall",
]

WORDS = {
    "quiet": "still", "early": "first", "old": "ancient", "low": "poor",
    "high": "strong", "slowly": "gently", "faster": "quicker", "good": "fine",
}


def paragraph(rng):
    return ". ".join(rng.choice(SENTENCES) for _ in range(rng.randint(3, 9))) + "."


def rewrite(paragraphs, rng):
    out = []
    middle = len(paragraphs) // 2
    for i, text in enumerate(paragraphs):
        if i == middle:
            continue  # taken out
        roll = rng.random()
        sentences = text.rstrip(".").split(". ")
        if roll < 0.33:
            sentences.insert(rng.randrange(len(sentences) + 1), rng.choice(SENTENCES))
        elif roll < 0.53 and len(sentences) > 2:
            sentences.pop(rng.randrange(len(sentences)))
        elif roll < 0.73:
            for old, new in WORDS.items():
                if f" {old} " in f" {sentences[0]} ":
                    sentences[0] = f" {sentences[0]} ".replace(f" {old} ", f" {new} ", 1).strip()
                    break
        out.append(". ".join(sentences) + ".")
        if i == 3:
            out.append(paragraph(rng) + " This paragraph is new in the rewrite.")
    return out


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--paragraphs", type=int, default=60)
    parser.add_argument("--size", type=int, default=0, help="repeat until this many bytes")
    parser.add_argument("--seed", type=int, default=11)
    parser.add_argument("--out", default="fixtures/compare/long-paragraphs")
    args = parser.parse_args()

    rng = random.Random(args.seed)
    paragraphs = [paragraph(rng) for _ in range(args.paragraphs)]
    if args.size:
        # Repeated until the text reaches the size, then cut back under it:
        # the window refuses a text past its limit (compare::TEXT_LIMIT).
        # Each copy numbered, so that no two paragraphs are the same text:
        # GPUI caches a shaped line by its text, and a text of repeated
        # paragraphs would measure the cache rather than the wrapping.
        base = list(paragraphs)
        copy = 0
        while sum(len(p.encode()) + 1 for p in paragraphs) < args.size:
            copy += 1
            paragraphs.extend(f"({copy}.{i}) {p}" for i, p in enumerate(base))
        while sum(len(p.encode()) + 1 for p in paragraphs) > args.size:
            paragraphs.pop()
    rewritten = rewrite(paragraphs, random.Random(args.seed + 1))

    os.makedirs(args.out, exist_ok=True)
    for name, text in (("original.txt", paragraphs), ("rewritten.txt", rewritten)):
        with open(os.path.join(args.out, name), "w", encoding="utf-8") as out:
            out.write("\n".join(text) + "\n")
    sizes = [os.path.getsize(os.path.join(args.out, n)) for n in ("original.txt", "rewritten.txt")]
    print(
        f"{args.out}: original {sizes[0]} bytes, {len(paragraphs)} paragraphs; "
        f"rewritten {sizes[1]} bytes, {len(rewritten)} paragraphs; "
        f"longest {max(len(p) for p in paragraphs + rewritten)} characters"
    )


if __name__ == "__main__":
    main()
