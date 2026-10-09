#!/usr/bin/env python3
"""Gemma 4's chat templates rendered for the one conversation this product
sends, against what `wipemark_llama::chat` writes for it.

What it is for
--------------
The companion of `gemma4-template.py` (same task: the coordinator's M9,
`wipemark-task-models-pipeline-followups-2026-10-09`, 2026-10-09). That
script reads the chat template out of the catalogue's pinned Gemma 4 file and
out of the repository's later re-upload, and diffs them; this one answers the
question the diff leaves — does the change reach the conversation the local
engine renders? `wipemark_llama::chat` does not run Jinja: it writes Gemma 4's
turns itself (D181), so a template whose rendering moved for that
conversation would be one the renderer no longer matches.

What it does
------------
For each template file given, renders three conversations with Jinja2 —
`bos_token` empty (the tokenizer adds BOS), `add_generation_prompt` true,
`enable_thinking` false, no tools — exactly the ones
`crates/wipemark-llama/src/chat.rs` pins in
`gemma_4_is_rendered_as_its_template_renders_it` and
`an_empty_system_message_is_what_each_template_makes_of_it`:

1. a system message `"  Rewrite the text.\\n"` and a user message
   `"The quick brown fox.\\n\\n"`;
2. a user message `"Привет."` alone;
3. a system message `" \\n"` and a user message `"x"`;

and compares each with the string the renderer's test expects (copied here
from `chat.rs`). Prints `same` or the two strings, per template and
conversation.

Jinja2 is not llama.cpp's Jinja engine (minja); the renderer's expected
strings were checked against llama.cpp's own at the pin in E2-4. Two
templates that render one conversation identically under Jinja2 and match
those strings are evidence, not proof: the host's `POST /apply-template`
against the pinned llama.cpp is the check before a pin moves.

How to run
----------
    python3 -I scripts/verify/models/gemma4-template.py --out DIR
    VENV/bin/python -I scripts/verify/models/gemma4-render.py DIR/*.jinja

What it needs
-------------
Python 3.9+ and Jinja2 (3.1.4 was used: `python3 -m venv VENV &&
VENV/bin/pip install jinja2==3.1.4`; nothing in the repository depends on
it), and the template files `gemma4-template.py --out` writes.

What its output means
---------------------
`same` for every conversation of every template: the renderer writes what
each template renders for it. Exit 0 then, 1 when any differs, 2 when a
template does not render.
"""

import sys

try:
    import jinja2
except ImportError:
    print("needs Jinja2: python3 -m venv VENV && VENV/bin/pip install jinja2==3.1.4")
    sys.exit(2)

# What `wipemark_llama::chat::render` writes for Gemma 4 12B, as its tests
# pin it (chat.rs).
CONVERSATIONS = [
    (
        [
            {"role": "system", "content": "  Rewrite the text.\n"},
            {"role": "user", "content": "The quick brown fox.\n\n"},
        ],
        "<|turn>system\nRewrite the text.<turn|>\n"
        "<|turn>user\nThe quick brown fox.<turn|>\n"
        "<|turn>model\n<|channel>thought\n<channel|>",
    ),
    (
        [{"role": "user", "content": "Привет."}],
        "<|turn>user\nПривет.<turn|>\n<|turn>model\n<|channel>thought\n<channel|>",
    ),
    (
        [
            {"role": "system", "content": " \n"},
            {"role": "user", "content": "x"},
        ],
        "<|turn>system\n<turn|>\n<|turn>user\nx<turn|>\n<|turn>model\n<|channel>thought\n<channel|>",
    ),
]


def raise_exception(message):
    raise jinja2.exceptions.TemplateError(message)


def main(paths):
    if not paths:
        print(__doc__.split("\n")[0])
        print("usage: gemma4-render.py TEMPLATE.jinja [...]")
        return 2
    environment = jinja2.Environment(
        trim_blocks=True,
        lstrip_blocks=True,
        extensions=["jinja2.ext.loopcontrols"],
    )
    environment.globals["raise_exception"] = raise_exception
    status = 0
    for path in paths:
        with open(path, encoding="utf-8") as source:
            text = source.read()
        try:
            template = environment.from_string(text)
        except jinja2.exceptions.TemplateError as error:
            print("%s: does not compile: %s" % (path, error))
            return 2
        print(path)
        for number, (messages, expected) in enumerate(CONVERSATIONS, start=1):
            try:
                rendered = template.render(
                    messages=messages,
                    bos_token="",
                    add_generation_prompt=True,
                    enable_thinking=False,
                    tools=None,
                )
            except jinja2.exceptions.TemplateError as error:
                print("  %d: does not render: %s" % (number, error))
                return 2
            if rendered == expected:
                print("  %d: same" % number)
            else:
                status = 1
                print("  %d: DIFFERS\n    template: %r\n    renderer: %r" % (number, rendered, expected))
    return status


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
