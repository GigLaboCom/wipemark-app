#!/usr/bin/env python3
# Print a GGUF's architecture and the parts of its chat template that decide whether
# wipemark_llama::chat can write its format with thinking off (D182, D407).
#
# What it is for
#   The owner, 2026-10-10: try Qwen3.5-9B and Qwen3.6-35B-A3B for 8–16 GB machines and MacBooks.
#   Before a model is loaded, check that its template is ChatML with an `enable_thinking` switch
#   and the empty think block `chat::family_of` looks for (Family::ChatmlThinkingOff, as Qwen3.8's).
#
# What it does
#   Reads the GGUF header only (never a tensor): general.architecture, general.name, the context
#   length key, the expert counts if any, and tokenizer.chat_template; then says whether the
#   template has each marker family_of reads ("<|im_start|>", "enable_thinking",
#   r"<think>\n\n</think>\n\n" — written in the template as a Jinja string,
#   so with a backslash and an n, as family_of's raw string has it).
#
# How to run
#   python3 -I scripts/verify/models/gguf-chat-template.py <file.gguf> [...]
#
# What it needs
#   Python 3 alone.
#
# What its output means
#   One block per file. "chatml-thinking-off: yes" means the markers are there; the load's own
#   verdict (chat_support) is still the one that counts.
import struct
import sys

TYPES = {0: "B", 1: "b", 2: "H", 3: "h", 4: "I", 5: "i", 6: "f", 7: "?", 10: "Q", 11: "q", 12: "d"}


def read_str(f):
    (n,) = struct.unpack("<Q", f.read(8))
    return f.read(n).decode("utf-8", "replace")


def read_val(f, t):
    if t == 8:
        return read_str(f)
    if t == 9:
        (et,) = struct.unpack("<I", f.read(4))
        (n,) = struct.unpack("<Q", f.read(8))
        return [read_val(f, et) for _ in range(n)] if n < 64 else f"<array of {n}>" if skip(f, et, n) else None
    fmt = "<" + TYPES[t]
    return struct.unpack(fmt, f.read(struct.calcsize(fmt)))[0]


def skip(f, et, n):
    if et == 8:
        for _ in range(n):
            read_str(f)
    elif et == 9:
        for _ in range(n):
            read_val(f, 9)
    else:
        f.seek(n * struct.calcsize("<" + TYPES[et]), 1)
    return True


for path in sys.argv[1:]:
    with open(path, "rb") as f:
        assert f.read(4) == b"GGUF", path
        version, n_tensors, n_kv = struct.unpack("<IQQ", f.read(20))
        kv = {}
        for _ in range(n_kv):
            key = read_str(f)
            (t,) = struct.unpack("<I", f.read(4))
            kv[key] = read_val(f, t)
    arch = kv.get("general.architecture")
    tpl = kv.get("tokenizer.chat_template", "")
    print(f"== {path}")
    print(f"  architecture: {arch}, name: {kv.get('general.name')}")
    for k in ("context_length", "expert_count", "expert_used_count", "block_count"):
        if f"{arch}.{k}" in kv:
            print(f"  {k}: {kv[f'{arch}.{k}']}")
    print(f"  chat template: {len(tpl)} chars")
    markers = ("<|im_start|>", "enable_thinking", r"<think>\n\n</think>\n\n")
    for marker in markers:
        print(f"  has {marker!r}: {marker in tpl}")
    print(f"  chatml-thinking-off: {'yes' if all(m in tpl for m in markers) else 'no'}")
