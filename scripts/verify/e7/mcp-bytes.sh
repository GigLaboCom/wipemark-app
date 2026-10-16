#!/usr/bin/env bash
# mcp-bytes.sh — the MCP server's answers before and after D289, byte for byte.
#
# What it is for
#   Host verification of the E7 second follow-ups, X1–X14 (asked by the
#   coordinator, 2026-10-06). X14 (D289) moved the third shelf of a text
#   report into `wipemark_core`'s report and made `to_json()` write it from
#   there. The MCP tools `inspect` and `clean` answer with that JSON as
#   `content[0].text` and, parsed, as `structuredContent`. This script holds
#   the server's whole answer — envelope, text block and structured content —
#   to the answer of the commit before the change, headlessly: no window, no
#   port, no running application.
#
# What it does
#   1. Writes JSON-RPC request bodies into "$MCP_DIR/requests": `tools/call`
#      of `inspect` and `clean` over a varied set of texts (U+200B in
#      Markdown, a homoglyph in a word, a soft hyphen, bidi controls, an
#      emoji ZWJ family and VS16, a stray ZWJ and VS16, tag characters, an
#      inner BOM, exotic spaces, NFKC-compatibility letters, CRLF, a fenced
#      block, Cyrillic with a Latin letter, the empty text), each with no
#      option, `aggressive`, `nfkc` and both; plus `initialize`,
#      `tools/list`, `ping` and two refused calls.
#   2. In each of two source trees — OLD_TREE (a checkout or export of feat,
#      9e92358) and NEW_TREE (the branch head; default the repository root) —
#      appends a temporary `#[ignore]`d test to
#      apps/wipemark-app/src/mcp/protocol.rs that hands every body to
#      `mcp::protocol::respond` (the function the HTTP server calls) and
#      writes the answer to "$MCP_DIR/<old|new>/<name>"; runs it with
#      `cargo test -p wipemark-app --locked mcp_bytes_probe -- --ignored`;
#      puts protocol.rs back from a copy whatever happened (trap).
#   3. `cmp`s every answer of OLD against NEW's.
#
# How to run
#   LIBRARY_PATH=<dir with libxkbcommon-x11.so> \
#   OLD_TREE=<feat tree> MCP_DIR=$(mktemp -d) scripts/verify/e7/mcp-bytes.sh
#   CARGO_TARGET_DIR is honoured and shared by both trees, so GPUI and the
#   other dependencies are compiled once.
#
# What it needs
#   cargo and the repository's toolchain, the GPUI system libraries the app's
#   tests link against, python3 (only to write the bodies), cmp. No network.
#
# What its output means
#   One line per answer that differs, then "N answers compared, K different".
#   K = 0 is the expected answer: an agent reads exactly what it read before.
#   The exit code is K (capped at 100); 101 if either tree's probe did not run.
set -uo pipefail

ROOT=$(git rev-parse --show-toplevel)
: "${OLD_TREE:?OLD_TREE=<a source tree of feat, 9e92358>}"
NEW_TREE=${NEW_TREE:-$ROOT}
MCP_DIR=${MCP_DIR:-$(mktemp -d)}
mkdir -p "$MCP_DIR/requests" "$MCP_DIR/old" "$MCP_DIR/new"

python3 - "$MCP_DIR/requests" <<'EOF'
import json, os, sys
d = sys.argv[1]
texts = {
    "zwsp": "# Notes\n\nTwo​ marks​ here, and `code​span`.\n",
    "homoglyph": "Log in to your pаypal account and сheck it.\n",
    "softhyphen": "soft­hyphen and hy­phen­\n",
    "bidi": "abc ‮evil‬ ⁦iso⁩ ⁧r⁩ ⁨f⁩ x‏y‎\n",
    "emoji": "family \U0001F468‍\U0001F469‍\U0001F467, heart ❤️, flag \U0001F3F4\U000E0067\U000E0062\U000E0065\U000E006E\U000E0067\U000E007F\n",
    "stray-joiners": "a‍b ❤️ c️d \U0001F468‍\U0001F469\n",
    "tags": "plain\U000E0041\U000E0042 text\U000E007F\n",
    "bom-inside": "start﻿middle\n",
    "spaces": "no break, ideographic　space, thin space, narrow nb\n",
    "nfkc": "ﬁle ① ＡＢＣ Ω Å x²\n",
    "crlf": "line one​\r\nline two\r\n",
    "fenced": "Text​.\n\n```\nlet x​ = 1;\n```\n",
    "cyrillic": "Привет, мир. Это тeкст с латинской e.\n",
    "empty": "",
}
n = 0
def put(name, body):
    global n
    n += 1
    with open(os.path.join(d, f"{n:03d}-{name}.json"), "w", encoding="utf-8") as f:
        f.write(json.dumps(body))
put("initialize", {"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}})
put("tools-list", {"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}})
put("ping", {"jsonrpc": "2.0", "id": 3, "method": "ping"})
put("refused-no-text", {"jsonrpc": "2.0", "id": 4, "method": "tools/call",
                        "params": {"name": "clean", "arguments": {}}})
put("refused-flag", {"jsonrpc": "2.0", "id": 5, "method": "tools/call",
                     "params": {"name": "inspect", "arguments": {"text": "a", "aggressive": "yes"}}})
for tool in ("inspect", "clean"):
    for name, text in texts.items():
        for opts in ({}, {"aggressive": True}, {"nfkc": True}, {"aggressive": True, "nfkc": True}):
            if tool == "inspect" and "nfkc" in opts:
                continue
            tag = "-".join(sorted(opts)) or "default"
            put(f"{tool}-{name}-{tag}", {"jsonrpc": "2.0", "id": 100 + n, "method": "tools/call",
                                         "params": {"name": tool, "arguments": dict(text=text, **opts)}})
EOF

probe() { # tree side
  local tree=$1 side=$2
  local rs=$tree/apps/wipemark-app/src/mcp/protocol.rs
  local backup
  backup=$(mktemp)
  cp "$rs" "$backup"
  # shellcheck disable=SC2064
  trap "cp '$backup' '$rs'; rm -f '$backup'" EXIT
  cat >>"$rs" <<'EOF'

#[cfg(test)]
mod mcp_bytes_probe {
    #[test]
    #[ignore]
    fn mcp_bytes_probe() {
        let dir = std::path::PathBuf::from(std::env::var("MCP_DIR").expect("MCP_DIR"));
        let side = std::env::var("MCP_SIDE").expect("MCP_SIDE");
        let mut names: Vec<_> = std::fs::read_dir(dir.join("requests"))
            .expect("requests")
            .map(|e| e.expect("entry").path())
            .collect();
        names.sort();
        for path in names {
            let body = std::fs::read_to_string(&path).expect("body");
            let answer = super::respond(&body).unwrap_or_else(|| String::from("<no answer>"));
            let out = dir.join(&side).join(path.file_name().unwrap());
            std::fs::write(out, answer).expect("answer");
        }
    }
}
EOF
  (cd "$tree" && MCP_DIR=$MCP_DIR MCP_SIDE=$side cargo test --locked -p wipemark-app mcp_bytes_probe -- --ignored >"$MCP_DIR/$side.log" 2>&1)
  local code=$?
  cp "$backup" "$rs"; rm -f "$backup"
  trap - EXIT
  return $code
}

probe "$OLD_TREE" old || { echo "the probe did not run in OLD_TREE: $MCP_DIR/old.log"; exit 101; }
probe "$NEW_TREE" new || { echo "the probe did not run in NEW_TREE: $MCP_DIR/new.log"; exit 101; }

compared=0; different=0
for o in "$MCP_DIR"/old/*; do
  n=$(basename "$o")
  compared=$((compared + 1))
  if ! cmp -s "$o" "$MCP_DIR/new/$n"; then different=$((different + 1)); echo "DIFFERENT: $n"; fi
done
[ "$(ls "$MCP_DIR/old" | wc -l)" = "$(ls "$MCP_DIR/requests" | wc -l)" ] || { echo "OLD answered $(ls "$MCP_DIR/old" | wc -l) of $(ls "$MCP_DIR/requests" | wc -l)"; different=$((different + 1)); }
echo "$compared answers compared, $different different"
echo "a sample (clean, U+200B):"
head -c 300 "$MCP_DIR"/new/*-clean-zwsp-default.json; echo
echo "MCP_DIR=$MCP_DIR"
exit $((different > 100 ? 100 : different))
