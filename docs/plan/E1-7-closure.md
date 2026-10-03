# E1-7 — Closing E1: the live gate, the documents, and the closure in Watchword

| | |
|---|---|
| Series | `docs/plan/` — epic E1, Layer A |
| Spec scopes | None of S1.1–S1.8 — the **closure** of E1: A §8 "живой гейт" (the live gate a test cannot take), the closure convention of CLAUDE.md "Specs" (an implementation spec is a Watchword FILE, its closure a TEXT beside it, ttl 0, read back), and the documents that E1 made untrue. |
| Depends on | **E1-1 … E1-6**, all committed on `feat/e0-e6-shell`, each with status *done* in `docs/plan/README.md` §3 and a report in `docs/plan/reports/`. |
| Unblocks | E2 and everything after it (`docs/plan/README.md` §2); the next series of plan documents is written from the closure. |
| Files touched | `docs/architecture/layer-a.md` · `docs/architecture/skeleton.md` · `docs/architecture/queue.md` (one sentence) · `CLAUDE.md` · `README.md` · `docs/README.md` · `docs/plan/README.md` (status row) · `docs/plan/reports/E1-7-<date>.md` · `crates/wipemark-i18n/i18n/{en-US,de,ru}/wipemark.ftl` (six window sentences, §4.2.7) · `crates/wipemark-core/src/*.rs` (**only** to remove the temporary `dead_code` allowances, §4.5). Watchword: one TEXT and one FILE. No other Rust change. |
| Size | ~1 day for one agent, one application launch. |

## §0 Ground rules — identical in every E1 document

### 0.1 Start here

You are an implementer agent working alone in `~/self/wipemark-app`
(GitHub `GigLaboCom/wipemark-app`), a Rust + GPUI desktop application
that strips AI-provenance marks from its owner's own text. This section
is identical in every document of the `docs/plan/` E1 series so that the
document is complete on its own. Read it, then read `CLAUDE.md` at the
repository root in full — if the two disagree, `CLAUDE.md` wins and you
say so in your report.

```sh
cd ~/self/wipemark-app
git switch feat/e0-e6-shell          # the working branch; never main
git status                           # must be clean apart from ` m vendor/gpui-component`
git submodule sync --recursive       # the submodule URL moved to GigLaboCom on 2026-10-03
git submodule update --init --recursive
scripts/pin-gpui-component.sh        # idempotent; skipping it = two `gpui` packages and a baffling type error
```

If the prompt tells you to work in a worktree, create it **from
`feat/e0-e6-shell`** (`git worktree add ../wipemark-<id> -b e1/<topic>
feat/e0-e6-shell`) and run the two submodule commands inside it. Commit
only when the prompt says so — one commit for the document, message
`E1-n: <title>`, ending with the co-author line the prompt gives you.
Never push unless the prompt says so (only E1-7 is ever told to), never
touch `main`, never `git checkout -- <file>` to undo a probe in a file
that has other uncommitted work.

### 0.2 Where code goes

```
core ← engine ← pipeline ← app / cli          (scripts/check-dep-direction.sh, the whole graph in a second)
models never depends on engine; image depends only on core; nothing depends on an app crate
```

- **`wipemark-core` has zero dependencies** — no `[dependencies]`, no
  `[dev-dependencies]`, **no `[build-dependencies]`**
  (`check-dep-direction.sh:120-136` reads all three). `build.rs` is
  written against `std` alone; a UCD line is `split(';')` and
  `u32::from_str_radix`. The crate is `#![forbid(unsafe_code)]`.
- **Only applications localize.** No library depends on
  `wipemark-i18n`. A library hands up structured values (ids, enums,
  numbers); the CLI and the app turn them into prose.
- **The module map and the interfaces between the E1 documents are
  fixed** in `docs/plan/README.md` §3.2–3.3. Build against them exactly;
  if one is wrong, implement it as written and say so in the report.
- Positions are **byte offsets into the source `&str`**, always.
- Nothing long runs on the GPUI thread (CLAUDE.md "Nothing blocks the
  GPUI thread"); Layer A is O(n) and is called from the MCP socket
  thread and the CLI, never from a render.

### 0.3 Rules of this repository that bind Layer A

- **The third shelf is never empty.** Every report that leaves the
  process carries `not_established` with the ids of
  `wipemark_core::report::not_established::ALL`. Nothing anywhere says
  "undetectable" — in any language.
- **No epic number leaves this repository.** A string a user or an agent
  reads says "not in this version yet", never `E2`. Epic ids live in
  code comments, docs and log lines.
- **Every string a person reads comes from the catalogue; nothing a
  machine reads does.** Catalogue: `crates/wipemark-i18n/i18n/en-US/
  wipemark.ftl` is the source of truth, `de` and `ru` must carry every
  key (the build generates `Message` from en-US; a missing key in
  another language fails the suite). JSON fields, class ids
  (`UnicodeClass::as_str`), confidence/action ids, `--json` output and
  everything the MCP server says are formats and are never translated.
  The CLI uses `Rendering::PlainText` — Fluent's U+2068/U+2069 isolates
  are `BidiControl`, and a CLI that printed them would be marking the
  files it was pointed at.
- **A tool that cannot do the work refuses; it never reports nothing.**
  MCP refusals are *results* with `isError: true`; CLI refusals exit 2.
  An empty report for a scan that did not run is the failure this
  product exists to avoid.
- **Exit codes are the CLI's interface:** `0` clean, `1` findings, `2`
  usage or refusal, `3` partial — *inconclusive is not clean*.
- **Layer A is never licence-gated.**

### 0.4 Tests: how this repository writes them

- **RED first, then green.** Write the test, watch it fail for the right
  reason, then write the code.
- **Delete the protection and watch it go red.** For every protection
  listed in your document's test section, apply the stated mutation
  locally, run the test, confirm it fails, restore the code, and record
  the result in your report (protection · mutation · test that went red).
  A test that stays green with its subject deleted is removed, not kept.
- Test names are sentences in `snake_case` that state the behaviour
  (`a_family_stays_a_family`, `positions_are_byte_offsets_into_the_source`)
  — the names in your document are the names to use.
- Fixtures under `fixtures/text/` are byte-exact (`.gitattributes`
  `fixtures/text/** -text`) and every fixture is asserted somewhere.
  Tests that need UCD data or fixtures read them with `include_str!` /
  `include_bytes!`; **no test touches the network**.
- Tests that exercise `pub(crate)` items are unit tests in their module
  (`#[cfg(test)] mod tests`, reading data with `include_str!` from `ucd/`
  or `fixtures/` when they need it); only tests that use the public API
  go to `crates/wipemark-core/tests/` — an integration test cannot see
  `pub(crate)`.

### 0.5 Gates — all green before you report done

```sh
rustup run nightly rustfmt --edition 2021 --check $(find crates apps -name '*.rs')   # not `cargo fmt --all`: never reformat vendor/
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
scripts/check-dep-direction.sh
cargo check --workspace --no-default-features --locked
cargo check --workspace --features local-llama --locked
```

While iterating, `cargo test -p wipemark-core` and `cargo clippy -p
wipemark-core --all-targets -- -D warnings` are minutes faster; the full
set runs before you report. `--locked` means `Cargo.lock` must not move
under you: `wipemark-core` never gains a dependency. A document that adds
a dependency to another crate (only E1-6 does) records it with one
`cargo check -p <crate>` **without** `--locked`, commits the moved
`Cargo.lock` together with the manifest, and then runs the gates with
`--locked` as above.

### 0.6 Do not

- launch the application unless your document's acceptance criteria ask
  for a live check — and then once, at the end, and kill it afterwards
  (a running `wipemark` holds MCP port 5056 and makes
  `a_port_something_else_holds_is_stepped_past` fail for an unrelated
  reason);
- edit anything under `vendor/` or reformat it;
- add a dependency to `wipemark-core`, or a `unicode-*` crate anywhere;
- change the module map or the interfaces in `docs/plan/README.md`
  §3.2–3.3 on your own;
- leave a `TODO` where your document asks for behaviour — implement it or
  report it as not done.

### 0.7 Definition of done (every document)

1. All six commands of §0.5 green; the mutation checks of your document
   performed and recorded.
2. The acceptance criteria of your document ticked, each with evidence
   (test name or command).
3. `docs/architecture/layer-a.md` has the section your document names,
   written for someone working on this code next year (what, where, why);
   `CLAUDE.md` touched only where your document says.
4. The status row in `docs/plan/README.md` §3 set to *done* with the
   report's file name.
5. A closing report at `docs/plan/reports/<id>-<YYYY-MM-DD>.md`: what was
   built; every deviation from the document and why; the mutation table;
   the commands you ran and their results; what you found that the next
   document should know. Report failures as failures.

## §1 Goal

E1 ends here. Three things, in this order of importance:

1. **The live gate** (A §8, "тестом не берётся" — not something a test can
   take): `wipemark-cli clean` on a real file whose zero-width space came
   through the clipboard writes `name.cleaned.ext` beside it, exits `1`, and
   its `--json` is read by `jq`; and a real MCP client — Claude Code, over
   `POST /mcp` — calls `clean` and gets text and a report back, not a
   refusal. This is the first time an agent gets a real answer from this
   product. Everything a test could have caught has been caught by E1-1…E1-6;
   what is left is what CLAUDE.md "A live check is not a substitute for a
   gate" says a live check is for: a sentence that is wrong, a control that
   truncates, a click that lands somewhere unexpected — and a protocol that a
   real client reads differently from our tests.
2. **The documents tell the truth.** `docs/architecture/layer-a.md` is
   complete; `CLAUDE.md`, `docs/architecture/skeleton.md`, `README.md` and
   `docs/README.md` stop saying Layer A does not exist; and six window
   sentences that say "cleaning is not in this version" — false since E1-6,
   because this version cleans from the command line and over MCP — say what
   is true: the *windows* do not clean yet.
3. **The closure in Watchword**: a TEXT `wipemark-core-layer-a-closed-<date>`
   beside the spec FILE `wipemark-core-layer-a-2026-09-21`, and a FILE snapshot
   of `docs/architecture/layer-a.md`, both ttl 0 and read back.

Then the final gates, one commit, and — only when your prompt says so — the
push of `feat/e0-e6-shell`.

## §2 Read first

Every line number is at `497eafa`; E1-1…E1-6 will have moved some, and
commit `ecd4e36` (2026-10-03, the submodule move) added nine lines to
`CLAUDE.md` after line 20 — at `a2f6ba3` and later every `CLAUDE.md` line
number above 20 is **+9**. Re-verify each against HEAD.

1. `CLAUDE.md`: `:7-15` ("Neither layer exists yet"), `:63-89` ("Working
   here": *Do not relaunch*, *Kill it when the check is done*, *A live check
   is not a substitute for a gate*), `:91-114` (the run-steering table:
   `WIPEMARK_DATA_DIR`, `WIPEMARK_LOG`, `--settings`, `--import`,
   `--compare`), `:95`, `:116-138` (the crate table), `:183-185`, the MCP rule
   as E1-6 rewrote it (was `:640-651`), `:1042-1079` ("Specs").
2. `docs/plan/README.md` — §3 (the status table), §4 (D1–D25, and any added since), §5 (owner
   questions), §9 (= §0).
3. **The six reports** `docs/plan/reports/E1-1-*.md` … `E1-6-*.md`: every
   mutation table, every deviation, every "what the next document should
   know". The closure is assembled from them.
4. The E1 spec `ssd-docs/wipemark-core-layer-a-2026-09-21.md` (Russian;
   Watchword FILE `wipemark-core-layer-a-2026-09-21`): §1 (deviations from
   the overview), §8 (the gates table and the live-gate paragraph), §9 (Q-A1…
   Q-A6).
5. `docs/plan/E1-6-mcp-and-cli.md` §4.2 and §4.4 — what the live gate
   exercises, and what each command should print.
6. `docs/architecture/layer-a.md` as E1-1…E1-6 left it.
7. `docs/architecture/skeleton.md` (`:10-27`, `:118-126`, `:132-145`);
   `README.md` (`:20-27`, `:112-130`, `:267-274`); `docs/README.md`;
   `docs/architecture/queue.md:40`.
8. The previous closure, for its shape: Watchword TEXT
   `wipemark-intake-drag-and-drop-closed-2026-09-11` (read it with
   `get_entry_by_word`) — Russian; sections *Что закрыто*, *Что появилось*,
   *Главная находка*, *Гейты* (with the RED checks), *Живой гейт*,
   *Отклонения*, *Открыто*.
9. For the scratch world of the live gate: `crates/wipemark-models/src/layout.rs:60`
   (`WIPEMARK_DATA_DIR`), `:82-87` (`discover`), `:126-128` (`wipemark.db`),
   `:176-178` (`logs/`); `crates/wipemark-store/src/lib.rs:83-105` (one
   migration, `user_version` 1); `apps/wipemark-app/src/config.rs:93-105`
   (`mcp.*` keys), `:877-878` (`mcp.enabled` defaults to `false`), `:226` and
   `:845-847` (`ui.setup.done`, a JSON boolean);
   `apps/wipemark-app/src/main.rs:545`, `:569`, `:581`, `:592` (the
   `--settings`, `--panel`, `--import=`, `--compare=` flags; they combine —
   tests at `:1184`, `:1225`); `apps/wipemark-app/src/mcp/mod.rs:320-342`
   (the Claude Code snippet); `apps/wipemark-app/src/mcp/server.rs:77-80`,
   `:379-382` (the 1 MiB body, the 413).

## §3 What is true today

### 3.1 At `497eafa` — what E1 makes untrue (verified)

| where | what it says | why it is wrong after E1 |
|---|---|---|
| `CLAUDE.md:7-15` | "**Neither layer exists yet.** … the MCP server (whose tools refuse by name) … Layer A is epic E1, Layer B is E2" | Layer A exists and two surfaces call it |
| `CLAUDE.md:95` | "`# the CLI (E5; most of it refuses by name)`" | `inspect` and `clean` run |
| `CLAUDE.md:118` | "Ten libraries under `crates/`" | there are **eleven** (the table below it, `:128-138`, lists eleven; so do the workspace members, `Cargo.toml:3-16`) — wrong already, fixed while the paragraph is open |
| `CLAUDE.md:128` | `wipemark-core` … "types; the classifier and scrubber are **E1**" | real |
| `CLAUDE.md:1061-1073` | the Specs table ends at `wipemark-status-2026-10-03` | the closure and the snapshot are missing |
| `skeleton.md:12` | core stub: "classifier, scrubber, UCD tables, guard implementations" | built |
| `skeleton.md:22-23` | app stub "the work those MCP tools will do, which refuses and names epic E1"; cli stub "every command body" | the tools run; `inspect`/`clean` have bodies |
| `skeleton.md:25-27` | "`wipemark-cli` exits **2** and names the epic" | stale twice: no epic is named since the epic-number rule, and two commands no longer refuse |
| `skeleton.md:120-121` | "UCD table generation and the classifier (E1)" under *Deliberately absent* | present |
| `skeleton.md:132-145` | owner questions Q1–Q8; "None of the open ones block E1, which is the next epic." | E1 is closed; Q-A1…Q-A6 are missing |
| `README.md:20-27` | "The **E0 skeleton** … `wipemark-cli` exits 2 and says which epic implements what you asked for" | stale |
| `README.md:121-127` | "**What it cannot do yet is the work.** Layer A is epic E1, so `tools/call` refuses by name …" | the tools run |
| `docs/README.md` | the `plan/README.md` row exists only as an **uncommitted** working-tree change (2026-10-03); no `layer-a.md` row | — |
| `docs/architecture/queue.md:40` | "the MCP tool that refuses by name today" | no MCP tool refuses by name now |
| catalogue (en-US `:65`, `:92`, `:190`, `:307`, `:379`, `:800`; de `:33`, `:51`, `:109`, `:166`, `:212`, `:443`; ru `:38`, `:57`, `:116`, `:173`, `:220`, `:456`) | `toolbar-help-pending`, `queue-pending`, `setup-welcome-body`, `panel-pending`, `compare-pending`, `settings-retention-pending` — each says cleaning (or "neither layer") is not in this version | this version cleans, from the command line and over MCP; the *windows* do not (A §7.4: the window seams are E7) |

Watchword (verified 2026-10-03): eleven `wipemark*` entries, among them
FILE `wipemark-core-layer-a-2026-09-21` (the spec) and TEXT
`wipemark-intake-drag-and-drop-closed-2026-09-11` (the previous closure,
whose `get_entry_by_word` answer carries no `expires_at`). No E1 closure, no
`layer-a.md` snapshot.

### 3.2 What E1-1…E1-6 delivered — the contract this document closes

Treat as given; verify against HEAD and the reports, and record any
difference.

- **`wipemark-core`** (README §3.2–3.3): the UCD 18.0.0 files under
  `crates/wipemark-core/ucd/` with `SHA256SUMS`, `scripts/fetch-ucd.sh`, the
  std-only `build.rs` (`UNICODE_VERSION` from the file headers, two versions
  fail the build), the tables and `Script`, `name_of` (E1-1); `class_of`, the
  context rules, `TextStats::of` (E1-2); `inspect`, `clean`, `Options`,
  `Cleaned`, `kept` on both reports, aggregation, the second pass after NFKC,
  UAX #15 NFKC with `NormalizationTest.txt` as its gate, idempotence, the
  §7.1 JSON with `not_established` (D9), `fixtures/text/` (E1-3); homoglyphs
  in both directions with the mixed-word and 10 % rules (E1-4); the five
  guards and `default_guards()` (E1-5).
- **The surfaces** (E1-6): MCP `inspect {text, aggressive}` → the §7.1
  report as `content[0].text` and `structuredContent`; `clean {text,
  aggressive, nfkc}` → `{ "text", "report" }` in both; argument problems are
  `isError` results naming the argument; client strings said back spelled;
  the pane's banner ends with `settings-mcp-tools-layer-a`. CLI `inspect
  <path|-> [--json]`, `clean <path|-> [-o out|-o -] [--nfkc] [--aggressive]
  [--json]`: UTF-8/16/32 with BOM preserved; beside as `name.cleaned.ext` via
  `wipemark_intake::name::with_infix`; exit `0`/`1` by the *input*'s
  `suspicious`, `2` for usage, `3` for anything not read or not written;
  `--json` per D12; the human report from the catalogue in PlainText ending
  with the third shelf; `rewrite`/`models`/`audit` still refuse at `2`.
- **The reports**: each `docs/plan/reports/E1-n-*.md` carries a mutation
  table (protection · mutation · test that went red). The closure needs all
  of them; a missing row is a gap you report, not one you fill by guessing.

## §4 Deliverables

### Order of work

1. §4.2.7 (the six window sentences) and `cargo test -p wipemark-i18n` —
   *before* the launch, so the one launch shows them (CLAUDE.md "Do not
   relaunch the application to watch it").
2. §4.5 (the `dead_code` allowances come off) and `cargo clippy -p
   wipemark-core --all-targets --locked -- -D warnings` — also before the
   launch, so the binary the live gate runs is the one that gets committed.
3. §4.1 the live gate, CLI half then application half; kill the app.
4. §4.2.1–4.2.6, §4.2.8 the documents, and the `.gitattributes` check of
   §4.6.
5. §4.4 the six gates, then the commit (when the prompt says to commit).
6. §4.3 Watchword — after the commit, because the closure names its SHA.
7. If a Watchword key came back different from the one written into
   `CLAUDE.md` (a collision suffix), fix the row and amend the commit.
8. §4.4 the push — only if the prompt says so.

If the live gate fails — a wrong exit code, a refusal where a report should
be, a document line in a log — **stop**: record it in the report as a
failure, do not fix Rust code in this document (§4.5 is the only Rust change
it makes), do not write the closure. The coordinator decides the fix.

### 4.1 The live gate

Run from the repository root in one shell (bash or zsh). `$LIVE` keeps
everything out of the real installation (CLAUDE.md, the
`WIPEMARK_DATA_DIR` row). The ZWSP is created with `printf` as UTF-8
`E2 80 8B` and in JSON as `\u200b` — never pasted raw into a command or a
document.

**Step 0 — preconditions.**

```sh
cd ~/self/wipemark-app
git status --short                         # nothing but ` m vendor/gpui-component`
git log --oneline -8                       # E1-1 … E1-6 on top of 497eafa (E1-5 may be a merge)
pgrep -fl 'target/(debug|release)/wipemark( |$)' || echo "no wipemark running"
lsof -nP -iTCP:5056 -sTCP:LISTEN || echo "5056 free"    # must print "5056 free"
```

If a `wipemark` is running or 5056 is held, stop it (it is a previous check
nobody killed — CLAUDE.md "Kill it when the check is done") before going on:
a server that steps to 5057 makes every URL below wrong.

**Step 1 — a scratch world.**

```sh
LIVE="$(mktemp -d -t wipemark-live.XXXXXX)"
export WIPEMARK_DATA_DIR="$LIVE/data"
export WIPEMARK_LANG=en-US
unset WIPEMARK_LOG RUST_LOG
cargo build -p wipemark-cli --locked
CLI="$PWD/target/debug/wipemark-cli"
```

**Step 2 — a real file whose ZWSP came through the clipboard.**

```sh
printf 'Hello\xe2\x80\x8bworld\n' | pbcopy
pbpaste > "$LIVE/note.md"
xxd "$LIVE/note.md"            # 4865 6c6c 6fe2 808b 776f 726c 640a   Hello...world.
shasum -a 256 "$LIVE/note.md" > "$LIVE/note.sha"
```

**Step 3 — `inspect` sees it and exits 1.**

```sh
"$CLI" inspect "$LIVE/note.md"; echo "exit=$?"
```

Expected (en-US): a first line naming `note.md` and "looks like a mark"; a
`Would be removed:` heading; the row `U+200B ZERO WIDTH SPACE · zero-width
character · confirmed · once, at byte 5`; `Checked against Unicode 18.0.0.`;
`Not established:` and three `  - ` lines; `exit=1`. Read every line as a
user would: a truncated or wrong sentence is a finding of this gate.

**Step 4 — `clean` writes beside, never over, and still exits 1.**

```sh
"$CLI" clean "$LIVE/note.md"; echo "exit=$?"          # exit=1; "The result is in …/note.cleaned.md." and "… itself was not changed."
ls -1 "$LIVE"                                          # data  note.cleaned.md  note.md  note.sha
shasum -a 256 -c "$LIVE/note.sha"                      # note.md: OK
xxd "$LIVE/note.cleaned.md"                            # 4865 6c6c 6f77 6f72 6c64 0a   Helloworld.
"$CLI" inspect "$LIVE/note.cleaned.md"; echo "exit=$?" # exit=0, "none of the characters this version looks for were found"
```

**Step 5 — `--json` is read by `jq`.**

```sh
"$CLI" clean --json "$LIVE/note.md" | jq .; echo "exit=${PIPESTATUS[0]}"     # pretty JSON; exit=1
"$CLI" clean --json "$LIVE/note.md" | jq -e --arg w "$LIVE/note.cleaned.md" '
     .written == $w
 and .report.unicode_version == "18.0.0"
 and .report.findings[0].codepoint == "U+200B"
 and .report.findings[0].name == "ZERO WIDTH SPACE"
 and .report.findings[0].positions == [5]
 and .report.removed["zero-width"] == 1
 and .report.not_established == ["vendor-detector-evasion","human-authorship","unknown-mark-schemes"]'
"$CLI" inspect --json "$LIVE/note.md" | jq -e '.suspicious == true and (.not_established | length) == 3'
pbpaste | "$CLI" clean - --json | jq -e '.text == "Helloworld\n" and (has("written") | not)'
pbpaste | "$CLI" clean - | xxd                          # 4865 6c6c 6f77 6f72 6c64 0a on stdout; the report on stderr
```

(In zsh use `${pipestatus[1]}` for `${PIPESTATUS[0]}`.) Every `jq -e` must
print `true` and exit 0.

**Step 6 — what must refuse, refuses, with the right code.**

```sh
"$CLI" inspect "$LIVE/nope.md"; echo "exit=$?"                       # exit=2, "… does not exist."
printf '\xf0\xd2\xc9\xd7\xc5\xd4 hello\n' > "$LIVE/koi8.txt"
"$CLI" inspect "$LIVE/koi8.txt"; echo "exit=$?"                      # exit=3, "… 8-bit encoding … not read is not clean"
"$CLI" clean "$LIVE/note.md" -o "$LIVE/note.md"; echo "exit=$?"      # exit=2, "--out names the file being read …"
shasum -a 256 -c "$LIVE/note.sha"                                    # note.md: OK — still untouched
"$CLI" rewrite "$LIVE/note.md"; echo "exit=$?"                       # exit=2, "… is not implemented yet …"
```

**Step 7 — the CLI's log has the shape, never the document.**

```sh
ls "$WIPEMARK_DATA_DIR/logs"                                          # wipemark-cli_<date>.log
grep -rn 'Hello\|note\.md' "$WIPEMARK_DATA_DIR/logs" && echo "LEAK" || echo "no document text, no file name"
grep -rhn '<elided' "$WIPEMARK_DATA_DIR/logs" | head -3
```

**Step 8 — a scratch database with the MCP server switched on.** The
server is off on a fresh install (`config.rs:877-878`), and the walk-through
would open over the main window (`ui.setup.done`). The store has exactly one
migration at `497eafa` (`crates/wipemark-store/src/lib.rs:89-99`, target
`user_version = 1`); **check that it still has one** — if E1 added any, do not
seed, and instead switch "Serve over MCP" on by hand in the window of step 9
(clicks by Quartz `CGEvent`; `osascript` clicks are ignored by GPUI).

```sh
mkdir -p "$WIPEMARK_DATA_DIR"
sqlite3 "$WIPEMARK_DATA_DIR/wipemark.db" <<'SQL'
CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
PRAGMA user_version = 1;
INSERT OR REPLACE INTO settings (key, value) VALUES ('mcp.enabled', 'true'), ('ui.setup.done', 'true');
SQL
sqlite3 "$WIPEMARK_DATA_DIR/wipemark.db" 'SELECT key, value FROM settings'
```

**Step 9 — build once, run once.** Run the built binary rather than `cargo
run` (same program; `$!` is then the application's own PID and the kill in
step 13 is certain). One launch opens everything this gate looks at: the
Settings window on the MCP page, the panel, the queue with one row, and the
Compare window.

```sh
cargo build -p wipemark-app --locked
"$PWD/target/debug/wipemark" --settings=mcp --panel \
    --import="$LIVE/note.md" --compare="$LIVE/note.md" > "$LIVE/app.out" 2>&1 &
APP=$!
URL=http://127.0.0.1:5056/mcp
H='Content-Type: application/json'
until curl -s -o /dev/null -X POST "$URL" -H "$H" -d '{"jsonrpc":"2.0","id":0,"method":"ping"}'; do sleep 0.5; done
```

(In Claude Code, wait with the Monitor tool or a background until-loop
rather than a foreground `sleep` if the harness refuses one.)

**Step 10 — the protocol by hand, over `curl`.**

```sh
curl -sS -X POST "$URL" -H "$H" -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"live-gate","version":"0"}}}' \
  | jq -e '.result.protocolVersion == "2025-06-18" and .result.serverInfo.name == "wipemark"'
curl -sS -o /dev/null -w '%{http_code}\n' -X POST "$URL" -H "$H" -d '{"jsonrpc":"2.0","method":"notifications/initialized"}'   # 204
curl -sS -X POST "$URL" -H "$H" -d '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' \
  | jq -e '[.result.tools[].name] == ["inspect","clean"] and all(.result.tools[]; .description | contains("1 MB"))'

curl -sS -X POST "$URL" -H "$H" \
  -d '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"clean","arguments":{"text":"Hello\u200bworld"}}}' > "$LIVE/clean.json"
jq -e '   .result.isError == false
      and .result.structuredContent.text == "Helloworld"
      and .result.structuredContent.report.removed["zero-width"] == 1
      and .result.structuredContent.report.findings[0].positions == [5]
      and (.result.structuredContent.report.not_established | length) == 3
      and (.result.content[0].text | fromjson) == .result.structuredContent' "$LIVE/clean.json"

curl -sS -X POST "$URL" -H "$H" \
  -d '{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"inspect","arguments":{"text":"Hello\u200bworld"}}}' \
  | jq -e '.result.structuredContent.suspicious == true and (.result.structuredContent | has("text") | not)'

curl -sS -X POST "$URL" -H "$H" \
  -d '{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"clean","arguments":{}}}' \
  | jq -e '.result.isError == true and (.result.content[0].text | contains("`text`")) and (.result | has("structuredContent") | not)'

python3 -c 'import json; print(json.dumps({"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"inspect","arguments":{"text":"a"*1100000}}}))' > "$LIVE/big.json"
curl -sS -o /dev/null -w '%{http_code}\n' -X POST "$URL" -H "$H" --data-binary @"$LIVE/big.json"    # 413 — refused whole (D13)
```

**Step 11 — Claude Code as the client, with the snippet the pane shows.**
The Claude Code tab's snippet is `{"mcpServers":{"wipemark":{"type":"http","url":…}}}`
(`mcp/mod.rs:325-327`). Load it for one non-interactive session only — do
not add it to the user's own Claude Code configuration. Check the flag
names with `claude --help` first; drop `--strict-mcp-config` if the
installed version lacks it.

```sh
printf '{"mcpServers":{"wipemark":{"type":"http","url":"%s"}}}\n' "$URL" > "$LIVE/mcp.json"
claude -p --mcp-config "$LIVE/mcp.json" --strict-mcp-config \
  --allowedTools "mcp__wipemark__clean" \
  'Call the clean tool of the wipemark MCP server exactly once. Its text argument is the JSON string "Hello\u200bworld" - the five letters Hello, then the character U+200B ZERO WIDTH SPACE, then world. Then print the tool result exactly as you received it, and nothing else.' \
  > "$LIVE/claude.out" 2>&1; echo "exit=$?"
cat "$LIVE/claude.out"
```

Expected: the printed result carries `Helloworld` and a report with
`"zero-width"` among `removed` and three `not_established` ids — **not** a
refusal and not an error. If the model dropped the invisible character on
the way in, the result says `removed` is empty — still a report, not a
refusal, and it passes the gate's question ("text and report, not a
refusal"); record it, and rerun once with a rephrased prompt to see a
removal. Cross-check in the application's log that the call arrived:

```sh
grep -rhn 'MCP: tools/call answered' "$WIPEMARK_DATA_DIR/logs" | tail -3      # tool=clean … text=<elided chars=… bytes=…>
```

**Step 12 — what a person sees.** With the windows of step 9 open, read
(or capture with `screencapture -x "$LIVE/screen.png"` and look at the
PNG; that needs Screen Recording permission for the terminal — if it is not
granted, say "not checked by eye" in the report rather than guessing):

- the MCP page banner: `Running, and answering on http://127.0.0.1:5056/mcp`
  over the `settings-mcp-tools-layer-a` sentence, nothing truncated;
- the queue's footer, the panel and the Compare window's banner: the
  §4.2.7 wording (cleaning from the window is not in this version yet; it
  runs from the command line and over MCP);
- the queue row for `note.md`: kind Text, format Markdown, encoding UTF-8.

**Step 13 — kill it, and prove the port is free.**

```sh
grep -rn 'Hello' "$WIPEMARK_DATA_DIR/logs" && echo "LEAK" || echo "no document text in any log"
kill "$APP"; wait "$APP" 2>/dev/null
pgrep -fl 'target/(debug|release)/wipemark( |$)' || echo "no wipemark running"
lsof -nP -iTCP:5056 -sTCP:LISTEN || echo "5056 free"
cargo test -p wipemark-app --locked a_port_something_else_holds_is_stepped_past     # green: nothing holds a port it should not
printf '' | pbcopy                                                                    # leave no ZWSP on the user's clipboard
rm -rf "$LIVE"
```

Record every command and its observed output in the report (§5 is the
checklist). The report's transcript is what the closure's *Живой гейт*
section is written from.

### 4.2 The documents

#### 4.2.1 `docs/architecture/layer-a.md` — completed

E1-1…E1-6 each added the section their document named. Make it one
document a reader next year can start from:

- a top section **"What Layer A is"** (one screen): deterministic,
  verifiable, the first shelf; Unicode 18.0.0 read from the UCD headers; what
  it finds (the eleven classes), what it keeps and why (orthography and
  presentation, `LikelyFalsePositive`), what it never looks at (A §2: C0/C1
  controls, U+2028/2029, unassigned code points, formats, stylometry); its two
  callers today and its callers to come (E4's pipeline, E7's windows);
- check that the sections of E1-1 (tables and `build.rs`), E1-2
  (classifier and context), E1-3 (scrubber, report, NFKC, JSON, fixtures),
  E1-4 (homoglyphs), E1-5 (guards) and E1-6 (Surfaces) are all present and
  agree with each other and with the code — two sections that describe one
  thing differently is the failure to fix here;
- **"The gates"**: the A §8 table with each test's name as it landed, the
  mutation applied and the document that applied it (from the reports);
- **"The live gate"**: what §4.1 checked and what it showed (short — the
  transcript is in the report and the closure);
- **"What is left open"**: Q-A1 per-class overrides and a Clean page (E7/E8);
  Q-A2 pairing of bidi embeddings (after real files); Q-A3 unassigned code
  points (E7); Q-A4 answered by the transport (D13); Q-A5 `arabic_ratio` /
  `hebrew_ratio` (E4); Q-A6 names exception taken as D16, owner may veto;
  plus every open item the reports raised.

#### 4.2.2 `docs/architecture/skeleton.md`

It is the status page CLAUDE.md sends a reader to "before assuming anything
works" (`CLAUDE.md:14-15`), so it follows the code:

- crate table (`:10-23`): `wipemark-core` real = "the UCD 18.0.0 tables
  and their `build.rs`, the classifier and its context rules, the scrubber,
  NFKC, homoglyphs, the five guards, `TextStats`, the report with its JSON
  and three shelves", stub = "nothing in Layer A; the guards have no caller
  until E4; per-class overrides are owner question Q-A1"; `wipemark-app`
  stub loses "the work those MCP tools will do, which refuses and names epic
  E1" (the tools run; what the windows do not do yet is clean — E7);
  `wipemark-cli` real gains "`inspect` and `clean`", stub becomes
  "`rewrite`, `models`, `audit` (E2/E3/E5)";
- `:25-27`: stubs refuse loudly — `wipemark-cli` exits **2** and says what
  is not in this version yet (no epic is named any more; CLAUDE.md "No epic
  number leaves this repository");
- *Deliberately absent* (`:118-126`): drop "UCD table generation and the
  classifier (E1)";
- *Owner questions still open* (`:132-145`): add Q-A1 … Q-A6 with their
  state (Q-A4: **answered** — the transport's 413 at 1 MiB is the limit,
  never a truncation, D13; Q-A6: **taken** as D16, the owner may veto, and
  then windows show only `U+XXXX` and the class while `name_of` stays for
  `--json`/MCP; Q-A1, Q-A2, Q-A3, Q-A5 open with the epic each blocks); and
  replace "None of the open ones block E1, which is the next epic" with the
  next epic per `docs/plan/README.md` §2 (E2 ‖ E5 rest).

#### 4.2.3 `CLAUDE.md`

Exactly these, nothing else (E1-6 already changed four other places):

- `:7-15` — replace the paragraph. Proposed:
  > **Layer A exists; Layer B does not yet.** Layer A — the UCD 18.0.0
  > tables, the classifier and what it keeps, the scrubber, NFKC,
  > homoglyphs and the five guards, in `wipemark-core` — is real, and two
  > surfaces call it: `wipemark-cli inspect|clean` and the MCP tools
  > `inspect`/`clean`. Real around it: the workspace and its four gates, the
  > GPUI shell and its Settings window, preferences as rows in SQLite, the
  > API key in the OS credential store, the model catalogue and its
  > verifying downloader, the MCP server, the rule that decides who would
  > rewrite if anything could, and the panel that takes a drop and says what
  > it was. The windows do not clean yet (E7), and Layer B is E2; both say
  > so out loud wherever a user could mistake them for present — see
  > `docs/architecture/skeleton.md` and `docs/architecture/layer-a.md`
  > before assuming anything works.
- `:95` — `cargo run -p wipemark-cli -- --help  # the CLI (inspect and clean
  run; the rest refuses by name)`.
- `:118` — "Eleven libraries under `crates/`".
- `:128` — `| wipemark-core | Layer A: the UCD tables, the Unicode taxonomy,
  the classifier and scrubber, NFKC, homoglyphs, the guards, the report and
  its JSON | real (the guards have no caller until **E4**) |`.
- *Specs* table (after `:1073`) — two rows, keys exactly as stored (§4.3):
  `| wipemark-core-layer-a-closed-<date> | TEXT | its closure: what landed in
  E1-1…E1-7, the gates and every RED check, the live gate, the deviations,
  what is left open |` and `| wipemark-layer-a-architecture-<date> | FILE | a
  snapshot of docs/architecture/layer-a.md at the closure |`.
- *Specs* — the sentence "What exists so far:" stays; if D1's plan copies
  were uploaded by the coordinator, their rows are the coordinator's.

#### 4.2.4 `README.md` (root)

- *What this build is* (`:20-27`): Layer A is real and runs from
  `wipemark-cli inspect|clean` and over MCP; the windows take what arrives,
  say what it is, compare, and do not clean yet; Layer B is not in this
  version. Point at `docs/architecture/skeleton.md` and `layer-a.md`.
- *MCP* (`:112-130`): replace "**What it cannot do yet is the work.** …" with
  what the tools answer (the report as text and `structuredContent`; `clean`
  adds the text), that a call it cannot run is refused as a result naming the
  argument, and the 1 MiB limit (413, never truncated). Keep the rest of the
  section.
- The exit-code table (`:267-274`) was E1-6's; check it reads as E1-6 §4.4.7.

#### 4.2.5 `docs/README.md`

Add the row `| [architecture/layer-a.md](architecture/layer-a.md) | Layer A:
the UCD 18.0.0 tables, what the classifier finds and what it keeps, the
scrubber, NFKC, homoglyphs, the guards, the report and its JSON, and the two
surfaces that call it |` after `skeleton.md`'s. The `plan/README.md` row was
added to the working tree on 2026-10-03 and was uncommitted then: if it is
committed by now, leave it; if it is still uncommitted, it is the
coordinator's change — leave the line as it is and say so in the report
(do not commit someone else's working-tree edit as yours, and do not add a
second row).

#### 4.2.6 `docs/plan/README.md`

Set **your** row of §3 to *done* with your report's file name (§0.7 item
4). Check that E1-1…E1-6 read *done* with a report each; if one does not,
stop and report it — E1 is not closed. Edit nothing else in that file: §1
(where the project stands), §3.2–3.3 and §4 are the coordinator's, and the
report lists what in §1 is now out of date.

#### 4.2.7 The window sentences E1 made untrue

Six catalogue messages say cleaning is not in this version. Since E1-6 it is
— from the command line and over MCP — and what is not there yet is
cleaning *in the windows* (A §7.4, E7). Reword them; keep every key, every
`$variable` (there are none), and the rule that a pending surface says "not
in this version yet" and never an epic number. The tests that read these
keys compare by message (`reads_as`) and stay green;
`the_retention_banner_always_says_nothing_is_written_yet` still holds — no
window writes. Two constraints shape the wording below:
`the_footer_says_cleaning_is_not_here_yet` (`apps/wipemark-app/src/queue.rs:1428-1435`)
asserts that `queue-pending` contains "not in this version" (in the
process's language — English on the machines that run the suite), so every
English line keeps that phrase; and `compare-pending` is shown twice — on the
Compare window and on the Settings › Compare page
(`apps/wipemark-app/src/settings.rs:3779`) — so it names the Compare window
instead of saying "this window".

| key | en-US | de | ru |
|---|---|---|---|
| `toolbar-help-pending` | Cleaning from this window is not in this version yet: it takes what arrives and says what it is. In this version cleaning runs from the command line (wipemark-cli clean) and, for an agent, over MCP. | Bereinigen aus diesem Fenster gibt es in dieser Version noch nicht: Es nimmt entgegen, was ankommt, und sagt, was es ist. In dieser Version läuft die Bereinigung über die Kommandozeile (wipemark-cli clean) und, für einen Agenten, über MCP. | Очистки из этого окна в этой версии пока нет: оно принимает то, что в него попадает, и говорит, что это. В этой версии очистка работает из командной строки (wipemark-cli clean) и, для агента, по MCP. |
| `queue-pending` | Cleaning from this list is not in this version yet. What it does today is take what you drop or import and say what it is; cleaning itself runs from the command line and over MCP. | Bereinigen aus dieser Liste gibt es in dieser Version noch nicht. Was sie heute tut: entgegennehmen, was abgelegt oder importiert wird, und sagen, was es ist; die Bereinigung selbst läuft über die Kommandozeile und über MCP. | Очистки из этого списка в этой версии пока нет. Что он делает уже сейчас: принимает то, что перетащили или импортировали, и говорит, что это; сама очистка работает из командной строки и по MCP. |
| `panel-pending` | Cleaning from this window is not in this version yet. What is real here today is that it takes what you drop and says what it is — and that it opens where you told it to. Cleaning itself runs from the command line and over MCP. | Bereinigen aus diesem Fenster gibt es in dieser Version noch nicht. Echt ist heute, dass es annimmt, was man darauf ablegt, und sagt, was es ist — und dass es dort aufgeht, wo man es hingelegt hat. Die Bereinigung selbst läuft über die Kommandozeile und über MCP. | Очистки из этого окна в этой версии пока нет. Сегодня настоящее здесь — то, что оно принимает брошенное и говорит, что это, и то, что оно открывается там, где вы указали. Сама очистка работает из командной строки и по MCP. |
| `compare-pending` | Cleaning in the Compare window is not in this version yet: the result starts as a copy of the original. Edit it, and every line that differs is marked on both sides. | Bereinigen im Vergleichsfenster gibt es in dieser Version noch nicht: Das Ergebnis beginnt als Kopie des Originals. Bearbeiten Sie es, und jede Zeile, die abweicht, wird auf beiden Seiten markiert. | Очистки в окне сравнения в этой версии пока нет: результат начинается как копия оригинала. Отредактируйте его — и каждая строка, которая отличается, будет отмечена с обеих сторон. |
| `settings-retention-pending` | No window writes anything yet: none of them cleans in this version, and { -layer-b } is not in it. These choices decide what happens to a file, and to what you paste, once they do. The command line never reads them. | Noch schreibt kein Fenster etwas: Keines bereinigt in dieser Version, und { -layer-b } ist nicht enthalten. Diese Einstellungen legen fest, was mit einer Datei und mit Eingefügtem geschieht, sobald sie es tun. Die Kommandozeile liest sie nie. | Пока ни одно окно ничего не записывает: в этой версии окна не очищают, а { -layer-b } отсутствует. Эти настройки решают, что случится с файлом и со вставленным, когда окна начнут это делать. Командная строка их никогда не читает. |
| `setup-welcome-body` | { -brand-name } strips AI provenance marks from your own content in two layers: { -layer-a }, which removes the invisible characters and is deterministic, and { -layer-b }, which asks a language model for a paraphrase. In this version { -layer-a } runs from the command line and over MCP but not yet from these windows, and { -layer-b } does not run at all. What these steps settle is what { -layer-b } will need when it does: who would rewrite, and what that takes. | { -brand-name } entfernt KI-Herkunftsspuren aus Ihren eigenen Inhalten in zwei Schichten: { -layer-a }, die die unsichtbaren Zeichen entfernt und deterministisch ist, und { -layer-b }, das ein Sprachmodell um eine Umformulierung bittet. In dieser Version läuft die { -layer-a } über die Kommandozeile und über MCP, aber noch nicht aus diesen Fenstern, und { -layer-b } läuft gar nicht. Diese Schritte klären, was das Umschreiben brauchen wird, wenn es kommt: wer umschreibt, und was dafür nötig ist. | { -brand-name } убирает следы ИИ из вашего собственного содержимого в два слоя: { -layer-a } удаляет невидимые символы и детерминирована, а { -layer-b } просит языковую модель о перефразировании. В этой версии { -layer-a } работает из командной строки и по MCP, но ещё не из этих окон, а { -layer-b } не работает совсем. Эти шаги решают, что понадобится слою переписывания, когда он появится: кто переписывает и что для этого нужно. |

Also rewrite the en-US comment above `settings-retention-pending`
(`:780-781`, "it stays until E1 and E4 land: nothing is written yet") to say
that it stays until a window writes (E4/E7). Run `cargo test -p
wipemark-i18n` and `cargo test -p wipemark-app` after this change, before the
launch.

#### 4.2.8 The sweep

```sh
git grep -n -i -E 'refuses by name|is epic E1|names epic E1|not implemented yet|Layer A is not|neither layer|Neither runs|skeleton status' -- \
  CLAUDE.md README.md docs crates apps ':!docs/plan' ':!vendor'
```

Every hit is either still true (say why in the report — `cli-not-implemented`
for `rewrite`/`models`/`audit` is), or fixed here if it is a document, or
listed in the report if it is code (crate docs that still say "Skeleton
status: … E1" belong to the document that owned that crate; name them for
the coordinator). `docs/architecture/queue.md:40` is a document: the
"assign a keyword" tool is a future MCP tool, not one that refuses — say
"the MCP tool that will assign one does not exist yet".

### 4.3 Watchword — the closure and the snapshot

After the commit (the closure names its SHA). Tools: `store_entry`,
`upload_file`, `get_entry_by_word`, `download_file` of the `watchword` MCP
server. **Always use the word the server returns** — on a collision it
appends a suffix — and write that word into `CLAUDE.md`.

**1. The snapshot FILE.**

```
upload_file(word: "wipemark-layer-a-architecture-<YYYY-MM-DD>",
            filename: "layer-a.md", content_type: "text/markdown", ttl_hours: 0)
```

then upload the committed file to the returned presigned URL, sending the
same content type it was signed for:

```sh
curl -sS -X PUT -H 'Content-Type: text/markdown' -T docs/architecture/layer-a.md '<presigned_url>'
```

Read back: `get_entry_by_word(<returned word>)` → `entry_type` `file`,
`status` `active`, **no `expires_at`**; `download_file(<returned word>)` and
compare `shasum -a 256` with the committed file.

**2. The closure TEXT** — `store_entry(word:
"wipemark-core-layer-a-closed-<YYYY-MM-DD>", payload: <below>, ttl_hours:
0)`. Language: Russian, like the spec it closes and the previous closure
(D1 governs `docs/`, not Watchword TEXTs; if your prompt says English, write
English). Outline, each section filled from the reports and §4.1 — facts,
SHAs, numbers, never "should":

```
# Закрытие: Layer A — детерминированный Unicode-скраббер (E1)

**Дата:** <date>. **Репозиторий:** wipemark-app, ветка feat/e0-e6-shell,
коммиты E1-1 <sha> … E1-7 <sha> (поверх 497eafa); запушено: <да/нет, SHA на origin>.
**Спека:** Watchword FILE wipemark-core-layer-a-2026-09-21.
**План:** docs/plan/README.md и docs/plan/E1-1 … E1-7 (в репозитории); отчёты docs/plan/reports/.
**Архитектурный документ:** docs/architecture/layer-a.md; снимок — Watchword FILE <returned word>.

## Что закрыто
S1.1–S1.8 по одному абзацу: что сделано, где лежит.

## Что появилось
| артефакт | что это | — ucd/ (версия, SHA256SUMS), build.rs, таблицы и их размер, бинарная дельта из отчёта E1-1,
модули core (§3.2 плана), fixtures/text/, CLI input/report/run, MCP-инструменты, ключи каталога (+44/−1, затем 6 переформулированных),
with_infix в wipemark-intake, docs.

## Главные находки
Из разделов отчётов «что знать следующему документу» — то, что меняет понимание, а не пересказ.

## Гейты
Шесть команд §0.5, все зелёные; число тестов: 576 на 497eafa → N на E1-7.

## Мутационный контроль
Полная таблица: защита · мутация · тест, ставший красным · документ (E1-n).
Каждая строка таблицы A §8 обязана в ней быть; строка без записи о «покраснении» — пробел, назвать его.

## Живой гейт
CLI: файл с ZWSP из буфера → note.cleaned.md рядом, код 1, jq читает --json; отказы 2/3; лог без текста.
MCP: curl initialize / tools/list / tools/call (clean, inspect, отказ, 413); Claude Code вызвал clean и получил текст и отчёт.
Что видно в окнах. Процесс убит, порт 5056 свободен.

## Решения плана и отклонения от спеки
D3–D25 (и все добавленные позже) одной строкой каждое; отклонения, о которых сообщили отчёты, и почему.

## Открыто
Q-A1, Q-A2, Q-A3, Q-A5 — открыты (что блокируют); Q-A4 — отвечен транспортом (D13); Q-A6 — принят как D16, владелец может наложить вето;
Q-D5/Q-D6 — как их коснулся E1-6; всё открытое из отчётов.

## Что знать E2 / E5 / E7
```

Read back: `get_entry_by_word(<returned word>)` → `entry_type` `text`,
`status` `active`, payload identical to what was sent, **no `expires_at`**
(the read-back CLAUDE.md "Specs" asks for).

**3.** If either returned word differs from what `CLAUDE.md` says, fix the
two rows and amend the commit.

Nothing else goes to Watchword from this document. D1 asks for copies of
the plan documents as dated FILE entries — that is the coordinator's upload
unless your prompt says otherwise.

### 4.4 Final gates, commit, push

1. The six commands of §0.5, all green, run after the last edit. Record the
   test count.
2. `git status --short` shows only your files and ` m vendor/gpui-component`.
3. Commit when the prompt says so: one commit, `E1-7: Closing E1 — the live
   gate, the documents, and the closure`, with the co-author line the prompt
   gives.
4. **Push only when the prompt says so** — the one exception to §0.1's
   "never push", and only in this document:
   ```sh
   git push origin feat/e0-e6-shell
   git ls-remote origin refs/heads/feat/e0-e6-shell     # the SHA equals `git rev-parse HEAD`
   ```
   Never `main`, never `--force`. Write the pushed SHA into the closure's
   header (`update` is not available for a TEXT: if the closure was stored
   before the push, store nothing new — the report records the pushed SHA).

### 4.5 The temporary `dead_code` allowances come off

E1-1 had to land lookup functions with no caller — the classifier (E1-2),
the scrubber (E1-3), homoglyphs (E1-4) and the guards (E1-5) arrive later —
so it scoped `#![cfg_attr(not(test), allow(dead_code))]` to
`crates/wipemark-core/src/tables.rs` and `src/script.rs`, and later documents
may have added the same for the same reason. With every caller in place,
they have no reason left, and an allowance with no reason hides the next
genuinely dead function.

```sh
git grep -n -E 'allow\(dead_code|expect\(dead_code' -- crates/wipemark-core
```

1. Remove every module-level `#![cfg_attr(not(test), allow(dead_code))]`
   (and any `#![allow(dead_code)]`) in `wipemark-core`.
2. Run `cargo clippy -p wipemark-core --all-targets --locked -- -D warnings`.
   For each item now reported dead: if no document of `docs/plan/` and no
   epic in `docs/plan/README.md` §7 names a caller for it, **delete it**
   (with its table, if `build.rs` generates one only for it — then rebuild
   and re-run the core suite); if a later epic does, put a narrow
   item-level `#[expect(dead_code, reason = "<the epic and the caller>")]` on
   that one item — `expect`, so the allowance fails the build the day the
   caller arrives — and list it in the report.
3. `cargo test -p wipemark-core --locked` green, then the full §0.5 set at
   the end as usual. `git grep` above returns nothing but the item-level
   `expect`s you listed.

### 4.6 `.gitattributes` says what D17 needs

D25 replaced the plan's first spelling (`ucd/*.txt -diff`, `fixtures/text/*
-text`), which matched nothing — a pattern with a slash is anchored at the
repository root, and `*` does not reach `ucd/emoji/`. E1-1 landed these three
lines; check they are there, unchanged, and that git applies them:

```
crates/wipemark-core/ucd/**/*.txt -text -diff
crates/wipemark-core/ucd/SHA256SUMS -text
fixtures/text/** -text
```

```sh
cat .gitattributes
git check-attr text diff -- crates/wipemark-core/ucd/UnicodeData.txt crates/wipemark-core/ucd/emoji/emoji-data.txt
git check-attr text -- crates/wipemark-core/ucd/SHA256SUMS
git check-attr text -- "$(git ls-files fixtures/text | grep -v README.md | head -1)"
```

Expected: `text: unset` and `diff: unset` for both UCD files, `text: unset`
for `SHA256SUMS` and for the fixture. Anything else is a finding for the
report — a fixture git normalises is a fixture whose bytes the tests no
longer pin (D17). `docs/architecture/layer-a.md` quotes these three lines,
not the plan's.

## §5 The live-gate checklist

Every row is an observation with evidence in the report (the command and
what it printed). A row that does not hold is a failure: record it as one.

| # | check | expected | evidence |
|---|---|---|---|
| 0 | nothing running before the start | "no wipemark running", "5056 free" | step 0 output |
| 1 | the file came through the clipboard | `xxd` shows `e2 80 8b` between `Hello` and `world` | step 2 |
| 2 | `inspect` finds it | exit 1; row `U+200B ZERO WIDTH SPACE · zero-width character · confirmed · once, at byte 5`; Unicode 18.0.0; three shelf lines | step 3 |
| 3 | `clean` writes beside | exit 1; `note.cleaned.md` without `e2 80 8b`; `note.md` checksum unchanged | step 4 |
| 4 | the result is clean by the same tool | `inspect note.cleaned.md` exit 0 | step 4 |
| 5 | `--json` is JSON | every `jq -e` prints `true`; `written` is the beside path; `positions == [5]`; `not_established` the three ids | step 5 |
| 6 | stdin and stdout | `clean - --json` has `text`, no `written`; `clean -` puts `Helloworld\n` on stdout | step 5 |
| 7 | refusals | missing → 2; KOI8-R → 3; `-o` = input → 2 and `note.md` unchanged; `rewrite` → 2 | step 6 |
| 8 | the CLI log | a `wipemark-cli_*.log` exists; no `Hello`, no `note.md`; `<elided` present | step 7 |
| 9 | one build, one launch | the app started once with `--settings=mcp --panel --import= --compare=`; `ping` answers on 5056 | step 9 |
| 10 | MCP handshake and listing | `initialize` names `2025-06-18` and `wipemark`; `notifications/initialized` → 204; tools `inspect`, `clean`, both descriptions say "1 MB" | step 10 |
| 11 | MCP `clean` | `isError` false; `text` `Helloworld`; `removed["zero-width"] == 1`; `positions == [5]`; three shelf ids; text block parses to `structuredContent` | step 10 |
| 12 | MCP `inspect` | `suspicious` true; no `text` | step 10 |
| 13 | MCP refusal | `clean` with `{}` → `isError` true, names `` `text` ``, no `structuredContent` | step 10 |
| 14 | MCP limit | a 1.1 MB body → HTTP 413 | step 10 |
| 15 | Claude Code | the session called `mcp__wipemark__clean` once and printed text and a report — not a refusal; the app log shows `MCP: tools/call answered … tool=clean` with an `<elided …>` text | step 11 |
| 16 | what a person reads | the MCP banner's two lines, the queue footer, the panel and the Compare banner read as §4.2.7 / E1-6 §4.5, untruncated — or "not checked by eye" with the reason | step 12 |
| 17 | no document text in any log | `grep Hello` over `logs/` finds nothing | step 13 |
| 18 | killed, port free | no `wipemark` process; "5056 free"; `a_port_something_else_holds_is_stepped_past` green | step 13 |
| 19 | nothing left behind | clipboard emptied; `$LIVE` removed | step 13 |

Two closure checks that are not part of the live gate but are on the same
list:

| # | check | expected | evidence |
|---|---|---|---|
| 20 | no temporary `dead_code` allowance left in `wipemark-core` | `git grep -n -E 'allow\(dead_code\|expect\(dead_code' -- crates/wipemark-core` prints nothing but item-level `expect`s each listed with its reason; `cargo clippy -p wipemark-core --all-targets --locked -- -D warnings` green | §4.5 output |
| 21 | `.gitattributes` holds the three E1-1 lines and git applies them | `git check-attr` prints `text: unset` (and `diff: unset` for the UCD `.txt`) for the UCD files, `SHA256SUMS` and a fixture | §4.6 output |

## §6 Acceptance criteria

- [ ] Every row of §5 holds, with evidence in the report — rows 20 and 21
      included: no module-level `dead_code` allowance left in
      `wipemark-core` with clippy `-D warnings` green, and `.gitattributes`
      applied as §4.6.
- [ ] `docs/architecture/layer-a.md` has "What Layer A is", the sections of
      E1-1…E1-6 consistent with each other and the code, "The gates", "The
      live gate", "What is left open".
- [ ] `skeleton.md`, `CLAUDE.md` (the five places of §4.2.3), `README.md`,
      `docs/README.md` and `queue.md:40` say what is true; the §4.2.8 sweep
      leaves no hit unexplained.
- [ ] The six window sentences reworded in en-US, de, ru; the i18n suite
      and the app suite green.
- [ ] `docs/plan/README.md` §3: E1-1…E1-7 all *done*, each with a report.
- [ ] The six §0.5 commands green after the last edit; one commit (when
      asked).
- [ ] Watchword: the snapshot FILE and the closure TEXT stored with ttl 0,
      read back with no `expires_at`, the snapshot's checksum equal to the
      committed file's; the returned words are the ones in `CLAUDE.md`.
- [ ] The closure carries the full mutation table, every A §8 row
      accounted for, and the live-gate record.
- [ ] Pushed and verified with `git ls-remote` — only if the prompt said so.
- [ ] Report at `docs/plan/reports/E1-7-<date>.md`, including what in
      `docs/plan/README.md` §1 is now out of date for the coordinator.

## §7 Out of scope

- Any change to Rust source other than §4.5 (removing the temporary
  `dead_code` allowances, and deleting an item that is then dead with no
  planned caller). A defect found by the live gate is reported, not fixed
  here.
- E7's window seams (A §7.4) — the Compare result as `clean(original)`, the
  panel's findings count, the queue's "Clean" action, the Inspector; only the
  six sentences are reworded so they stop being false.
- E5 (`rewrite`, `audit`, `models`, `--sarif`, the in-place flag), E2
  (engines), Q-A1 preference rows.
- Editing `docs/plan/README.md` beyond the status row; the plan's Watchword
  copies (D1) unless the prompt asks.
- Pushing without the prompt's say-so; anything on `main`.

## §8 Basis and references

- **E1 spec** `wipemark-core-layer-a-2026-09-21` (A): §8 — the gates
  table ("каждая защита … красится указанным способом **до** того, как
  считается сделанной, и закрытие (TEXT рядом с этой спекой) перечисляет,
  что и как красили" — every protection is painted red before it counts, and
  the closure TEXT beside the spec lists what was painted and how) and the
  live gate ("`wipemark-cli clean` на реальном файле с ZWSP из буфера обмена →
  `.cleaned.` рядом, код 1, `--json` читается `jq`; MCP-клиент (Claude Code с
  `POST /mcp`) вызывает `clean` и получает текст и отчёт, а не отказ"); §7.4
  (window seams are E7); §9 (Q-A1…Q-A6).
- **Overview** `heretic-unmark-overview-decomposition-2026-09-07` §10 (an
  epic closes with its gates; the closure convention).
- **Plan** `docs/plan/README.md`: §3 (statuses), §3.1 item 4 ("after E1-6:
  the live gate in E1-7 is the first time an agent gets a real answer from
  this product"), §3.1 item 5 and R5 (push after each landed document — here
  only on the prompt's say-so), D1 (English in `docs/`, Watchword copies),
  D13 (the 413), D15 (the banner), D16 (names).
- **CLAUDE.md** rules quoted by name: "Do not relaunch the application to
  watch it"; "Kill it when the check is done"; "A live check is not a
  substitute for a gate"; "Delete the protection and watch it go red"; "No
  epic number leaves this repository"; "The third shelf is never empty";
  "Diagnostics go to a file, and the document never does"; "Exit codes are
  the CLI's interface"; "Specs" (an implementation spec is a Watchword FILE,
  ttl 0; its closure a TEXT beside it, read back for no `expires_at`; a
  snapshot is re-uploaded under a new dated key and the table moved).
- **Model Context Protocol** 2025-06-18 — lifecycle (`initialize`,
  `notifications/initialized`), streamable HTTP transport, tools:
  <https://modelcontextprotocol.io/specification/2025-06-18>,
  <https://modelcontextprotocol.io/specification/2025-06-18/server/tools>.
- **JSON-RPC 2.0**: <https://www.jsonrpc.org/specification>.
- **Project Fluent** (the six reworded messages keep their shape):
  <https://projectfluent.org/>.
- `jq` manual (`-e` sets the exit status from the last output):
  <https://jqlang.org/manual/>.
