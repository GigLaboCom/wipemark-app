# E1-6 — The first callers: MCP inspect and clean, CLI inspect and clean

| | |
|---|---|
| Series | `docs/plan/` — epic E1, Layer A |
| Spec scopes | **S1.8** — A §1 (the row "§10 E1 … + S1.8"), A §2 (decoding is the caller's), A §5.5 (catalogue keys, the third shelf), A §7.1 (JSON), A §7.2 (MCP), A §7.3 (CLI), A §8 rows *третья полка*, *MCP*, *CLI*; A §9 Q-A4 and Q-A6. Plan decisions D9–D16 and D26–D29 (`docs/plan/README.md` §4). |
| Depends on | **E1-3** (`inspect`, `clean`, `Options`, `Cleaned`, the report changes, `to_json`) and **E1-4** (homoglyphs inside `collect_hits`), committed on `feat/e0-e6-shell`; transitively E1-1 and E1-2. Not E1-5: no surface calls a guard. |
| Unblocks | **E1-7** (the live gate is the first time an agent gets a real answer from this product), **E5** (the rest of the CLI), **E7** (the window seams of A §7.4) |
| Files touched | `crates/wipemark-intake/src/{name,lib}.rs` · `apps/wipemark-app/src/retention.rs` · `apps/wipemark-app/src/mcp/{protocol,mod}.rs` · `apps/wipemark-app/src/mcp/server.rs` (one test) · `apps/wipemark-app/src/settings.rs` · `apps/wipemark-cli/Cargo.toml` · `apps/wipemark-cli/src/main.rs` + new `src/{input,report,run}.rs` + new `tests/cli.rs` · `crates/wipemark-i18n/i18n/{en-US,de,ru}/wipemark.ftl` · `crates/wipemark-i18n/src/tests.rs` · `Cargo.lock` · `docs/architecture/{i18n,layer-a,retention}.md` · `CLAUDE.md` (four places, §4.6) · `README.md` (exit codes) · `docs/plan/README.md` (status row) · `docs/plan/reports/E1-6-<date>.md` |
| Size | ~3 days for one agent. No application launch: the live check of this work is E1-7's. |

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

Layer A exists after E1-3 and E1-4, and nothing calls it. Two surfaces have
been waiting for it since E0/E6 with their arguments, their tests and a
refusal by name: the MCP tools `inspect` and `clean`
(`apps/wipemark-app/src/mcp/protocol.rs:159`, `:260`) and the CLI commands
`wipemark-cli inspect` and `wipemark-cli clean`
(`apps/wipemark-cli/src/main.rs:485-550`). This document connects them —
S1.8, the scope A §1 added to E1 because "a library nobody calls has no live
gate".

When it is done:

1. **MCP** `tools/call inspect { text, aggressive }` answers with the §7.1
   report as `content[0].text` (a JSON string) and as `structuredContent`
   (the parsed object); `tools/call clean { text, aggressive, nfkc }`
   answers with `{ "text": <cleaned>, "report": <§7.1> }` in the same two
   places. A call that cannot run — `text` missing or not a string, a flag
   that is not a boolean, an argument the tool does not take — is refused as
   a *result* with `isError: true` that names the argument. The pane's banner
   stops saying the tools do not run and says that rewriting is not in this
   version (D15).
2. **CLI** `inspect <path|->` and `clean <path|-> [-o out] [--nfkc]
   [--aggressive]`, both with `--json`, read UTF-8/UTF-16/UTF-32 through
   `wipemark-intake`, run Layer A, write the result beside the file as
   `name.cleaned.ext` (or to `-o`, or to stdout), print a human report from
   the catalogue in `Rendering::PlainText` or the JSON of D12, and exit
   `0`/`1`/`3` by the rules of A §7.3 and D11. `rewrite`, `models` and
   `audit` keep refusing at `2`.
3. **Every answer carries the third shelf** — `not_established` with the
   three ids of `not_established::ALL` in every JSON, the three translated
   lines in every human report — gated by `every_layer_a_answer_carries_the_third_shelf`
   in both applications.
4. The **catalogue** gains the eleven `unicode-class-<id>` keys, the four
   `confidence-<id>` keys, the CLI report and refusal lines, and
   `settings-mcp-tools-layer-a`, in en-US, de and ru (A §5.5, D15); character
   names are written down as the one exception to the catalogue rule (D16).
5. `with_infix`, `RESULT_INFIX` and `ORIGINAL_INFIX` move to
   `wipemark_intake::name` so the CLI names a result exactly as the app does
   (D10).

## §2 Read first

Read in this order; every line number is at `497eafa` and must be
re-verified against HEAD (E1-1…E1-5 will have moved some of them).

1. `CLAUDE.md` — at `497eafa`: "No epic number leaves this repository"
   (`:209-219`), "Every string a person reads comes from the catalogue"
   (`:220-225`), "`Rendering::PlainText` for anything that is not a window"
   (`:230-234`), "The bytes decide what a thing is" (`:449-477`),
   "Diagnostics go to a file, and the document never does" (`:625-638`),
   "The MCP server answers; its tools refuse" (`:640-651`), "Nothing the MCP
   server says comes from the catalogue" (`:652-661`), "A result goes beside
   the file …" (`:986-1026`), "Exit codes are the CLI's interface"
   (`:1030-1036`). Commit `ecd4e36` (2026-10-03, the submodule move) added
   nine lines to `CLAUDE.md` after line 20, so at `a2f6ba3` and later every
   `CLAUDE.md` line number above 20 is **+9**. None of the code files this
   document cites changed between `497eafa` and `a2f6ba3`.
2. `docs/plan/README.md` — §3.2–§3.3 (the contract), §4 D9–D16 (binding
   for this document), §9 (= §0 above).
3. The E1 spec `ssd-docs/wipemark-core-layer-a-2026-09-21.md` (Watchword
   FILE `wipemark-core-layer-a-2026-09-21`; Russian): §2, §5.5, §7.1–§7.5,
   §8 (the rows named in the header), §9 Q-A4, Q-A6. This document is a
   precise English rendering of those sections plus the plan's decisions;
   where they differ, the plan's decision wins and §8 below says why.
4. `apps/wipemark-app/src/mcp/protocol.rs` (all 574 lines),
   `apps/wipemark-app/src/mcp/server.rs:53-80` (constants), `:325-409`
   (`answer`, `route`, the 413), `apps/wipemark-app/src/mcp/mod.rs:1-66`.
5. `apps/wipemark-app/src/settings.rs:640-657` (`Status`), `:5463-5546`
   (the MCP page and its banner), `:6065-6159` (`Tone`, `models_banner` —
   the pure-banner pattern), `:6386-6391` (`on_screen`), `:7095-7127` and
   `:7480-7562` (the banner gates and `reads_as`).
6. `apps/wipemark-cli/src/main.rs` (all 784 lines) and
   `apps/wipemark-cli/Cargo.toml`.
7. `apps/wipemark-app/src/retention.rs:54-78`, `:396-473`, `:475-555`,
   `:826-832`; `docs/architecture/retention.md:59-91`.
8. `crates/wipemark-intake/src/lib.rs` (all), `src/text.rs` (all),
   `src/name.rs` (all), `src/format.rs:134-280`.
9. `crates/wipemark-i18n/i18n/en-US/wipemark.ftl:1-35` (rules, terms),
   `:476-500`, `:832-895` (MCP), `:920-932` (third shelf), `:951-1013`
   (CLI); the same sections of `de` and `ru`;
   `crates/wipemark-i18n/src/tests.rs` (all); `docs/architecture/i18n.md`.
10. `scripts/check-dep-direction.sh:35-98`; `crates/wipemark-log/src/lib.rs:232-272`
    (`Elided`).
11. `docs/architecture/layer-a.md` as E1-1…E1-5 left it — you add the
    "Surfaces" section.

## §3 What is true today

### 3.1 At `497eafa` (verified)

**MCP protocol** (`apps/wipemark-app/src/mcp/protocol.rs`)

- `Tool { Inspect, Clean }` (`:70-74`), `Tool::ALL` (`:78`), names `"inspect"`,
  `"clean"` (`:81-86`).
- Descriptions (`:89-102`): `inspect` names six kinds (zero-width, bidi
  controls, tag characters, variation selectors, private-use, noncharacters)
  and omits soft hyphens, exotic spaces, other default-ignorables and
  homoglyphs; neither mentions a size limit or byte offsets; `inspect`'s
  contains an em dash (non-ASCII).
- `schema()` (`:105-140`): `text` string, required; `aggressive` boolean,
  default `false`, described as *"Also act on homoglyphs and exotic spaces"*
  (`:119-120`) — **wrong after D3/A §5.2**: `aggressive` only turns
  `Homoglyph` from Keep to Replace; exotic spaces belong to the
  `normalize_spaces` knob, which no surface exposes in E1. `nfkc` boolean on
  `clean` only (`:123-133`). No `additionalProperties`.
- `refusal()` (`:159-173`): one text for every call, `"`{name}` is not
  implemented yet. …"`, `isError: true`.
- `respond()` (`:181-224`); `dispatch()` (`:227-257`) — the unknown-method
  answer `format!("method not found: {other}")` (`:252-255`) **echoes the
  client's method name verbatim**.
- `call()` (`:260-283`): no `name` → `-32602` (`:261-266`); a known tool →
  `refusal()` whatever the arguments (`:271`); an unknown tool → `-32602`
  whose message **echoes the client's tool name verbatim** (`:275-281`).
- Constants `PARSE_ERROR` … `INVALID_PARAMS` (`:57-60`); no `INTERNAL_ERROR`.
- Module docs `:8-24` ("The tools refuse, and say why", "with the epic in
  the message" — already stale: the refusal text names no epic).
- Tests: `answer()` helper (`:323-326`);
  `a_tool_that_cannot_run_refuses_rather_than_reporting_nothing`
  (`:376-401`) sends `{"text":"hi"}` and asserts the text contains
  `"not implemented"` — it goes red the moment the tools run (D14);
  `a_refusal_reaches_the_model_rather_than_the_client` (`:407-417`) sends
  `"arguments":{}`; `a_tool_this_build_does_not_have_is_refused_at_the_protocol_level`
  (`:422-435`); `every_tool_is_listed_with_something_an_agent_can_act_on`
  (`:440-458`); `no_two_tools_read_the_same` (`:461-469`);
  `the_id_comes_back_exactly_as_it_was_sent` (`:512-522`);
  `nothing_the_server_says_carries_an_invisible_character` (`:533-573`) —
  ten requests, forbidden set at `:560-567`
  (U+2066–2069, U+200B–200F, U+202A–202E, U+FEFF, U+E0000–E007F).

**MCP transport and module** — `server.rs:77-80` `LARGEST_BODY = 1024 * 1024`;
`:379-382` a `POST` whose `Content-Length` exceeds it is answered
`413 Payload Too Large` *before* the body is read; `:383-389` a body that is
not UTF-8 is `400`; `:201-211` one thread per connection, so Layer A runs on
a socket thread, never on the GPUI thread; `:67-70` `PATIENCE` 5 s on read
and write. `mod.rs:20-28` says the tools "refuse, by name, and say which epic
implements them" (stale after this document); `:62` `SERVER_NAME =
"wipemark"`, `:66` `PATH = "/mcp"`, `:74` `DEFAULT_PORT = 5056`; `:325-327`
the Claude Code snippet `{"mcpServers":{"wipemark":{"type":"http","url":…}}}`.
`config.rs:877-878`: `mcp.enabled` defaults to `false`.

**MCP pane** (`apps/wipemark-app/src/settings.rs`) — `Status` (`:644-657`);
`state_of_the_server` (`:5494-5546`) is a method on `SettingsView` that picks
glyph, colour and lines from `Status` and always pushes
`t(Message::SettingsMcpToolsPending)` last (`:5544`); its doc comment
(`:5497-5502`) says "the tools land in E1". There is **no test** of this
banner: the Placement gate's comment (`:7500-7505`) claims "the Engine, Models
and MCP banners" keep the bargain, but only the Engine (`:7105-7127`), Models
(`:7213-7231`), Placement (`:7507-7518`) and Retention (`:7548-7562`) banners
are gated. The pure-function pattern to copy is `on_screen` (`:6386-6391`),
`models_banner` (`:6119-6159`) and `Tone` (`:6071-6095`, `colour` maps
`Quiet/Good/Warn/Bad` to `muted_foreground/success/warning/danger`).
`reads_as` (`:7489-7498`) compares a line against a message in every shipped
language.

**CLI** (`apps/wipemark-cli/src/main.rs`)

- Header `:8-15` documents exit `1` as *"marks were found (inspect), or remain
  (clean/rewrite)"* — **contradicts A §7.3**, which makes `clean` exit `1` when
  the *input* was suspicious, "even if cleaned: a pre-commit hook wants to know
  what was there". Root `README.md:267-274` says the same as the header.
  `:35-39` "Skeleton status".
- `Exit` (`:50-61`) with `#[allow(dead_code, reason = "…E5 constructs the
  rest")]` (`:52-55`).
- `Action::Inspect { path: String, json }`, `Action::Clean { path: String,
  out: Option<PathBuf>, nfkc, aggressive, json }` (`:94-110`); `epic()`
  (`:162-169`), `name()` (`:171-179`), `summary()` (`:189-230`),
  `render_out()` (`:233-238`).
- `ARGUMENT_HELP` (`:248-263`) and its comment (`:240-247`): "only `inspect`
  takes `-` for stdin". `command()` (`:375-396`) gives `inspect`'s `path` the
  `CliArgPathOrStdin` help (`:380-383`) and `clean` none (`:384`) — **against
  OV §7 and A §7.3**, which both spell `clean <path|->`.
- `configured_language()` (`:448-457`) opens the database read-only;
  `init_logging()` (`:475-483`) mirrors to stderr only when `WIPEMARK_LOG` is
  set (`:479`).
- `main()` (`:485-550`): logging, language pre-parse, `Rendering::PlainText`
  (`:497`), parse, the unknown-language warning (`:508-527`),
  `tracing::info!(command, epic, "not implemented")` (`:533-537`), the
  `cli-not-implemented` refusal on stderr (`:539-548`), `Exit::Usage`.
- Tests `:552-784`, among them `exit_codes_are_pinned` (`:715-721`),
  `stdin_is_addressable_as_dash` (`:723-733`), `summary_echoes_every_flag`
  (`:759-774`), `help_carries_no_character_layer_a_would_strip` (`:646-662`,
  its own independent set at `:655-657`).
- `apps/wipemark-cli/Cargo.toml:15-29`: core, i18n, pipeline, models,
  tracing, log, store, clap. **No `wipemark-intake`, no `serde_json`.** The
  comment at `:27-29` ("Only to read `[ui] language` out of config.toml") is
  stale since preferences moved to SQLite.
- `scripts/check-dep-direction.sh:97` — `"wipemark-cli": LIBS`, and `LIBS`
  (`:35-47`) contains `wipemark-intake` (`:42`): the new dependency is
  allowed. `Cargo.lock` lists `wipemark-cli`'s dependencies, so adding two
  **moves `Cargo.lock`** (§4.7).

**Result names** (`apps/wipemark-app/src/retention.rs`) — `RESULT_INFIX =
"cleaned"` (`:61-68`), `ORIGINAL_INFIX = "original"` (`:70-78`), `with_infix`
(`:457-473`); callers only inside `plan` (`:408`, `:412`, `:416`, `:432`);
tests import all three (`:481-484`); `a_result_is_named_after_its_file`
(`:523-539`), `a_result_beside_a_file_is_never_the_file_itself` (`:544-555`),
`the_infixes_are_formats_and_stay_ascii` (`:826-832`). Nothing outside
`retention.rs` names them.

**Intake** (`crates/wipemark-intake`, zero dependencies) — `HEAD = 4096`
(`lib.rs:87`); `Intake` (`:148-165`); `is_textual()` (`:187-189`) is true for
`PlainText, Markdown, Html, Xml, Json, Csv, Url, Svg, Rtf`
(`format.rs:272-278`); `contradicted()` (`lib.rs:193-198`); `identify(head,
name)` is pure (`:207-252`) and sets `encoding` only when the format is textual
(`:245`); `of_path` (`:327-360`) reads the head itself and turns an unreadable
file into `Evidence::Name` *without an error* (`:324-326`), and treats a
zero-byte file as empty text (`:354-358`); module status `:62-66` says
"scrubbing the text … is E1". `Encoding` (`text.rs:27-38`) with `name()`
(`:41-51`: `UTF-8`, `UTF-16LE`, `UTF-16BE`, `UTF-32LE`, `UTF-32BE`, `8-bit`);
`mark()` (`:71-76`) longest BOM first (`:61-67`); `utf8()` refuses a head
with a control other than tab, CR, LF, VT, FF, ESC (`:113-129`);
UTF-16 without a BOM is inferred only from an all-ASCII two-byte pattern
(`:137-157`); any NUL ends an eight-bit verdict (`:163-175`). The test idiom
for scratch directories is `Scratch` (`lib.rs:377-401`) — no `tempfile` crate.

**Catalogue** — terms `-layer-a = cleaning` (`en-US:30`), `-layer-b =
rewriting` (`:35`); de `Bereinigung`/`Umschreiben`, ru
`очистка`/`переписывание` (lines 8-9). `settings-mcp-tools-pending` at
en-US `:851`, de `:475`, ru `:488`, and named in en-US comments at `:489`
and `:840`. Third shelf `report-not-established-title` and three items
(en-US `:929-932`, de `:517-520`, ru `:530-533`). CLI section en-US
`:951-1013`: `cli-arg-path-or-stdin` (`:986`), `cli-arg-out` (`:988`),
`cli-arg-aggressive` (`:990`, same wrong "and exotic spaces"; de `:556`, ru
`:569`), `cli-not-implemented` (`:1009-1013`). 421 keys per language. The
gates every new key must pass (`crates/wipemark-i18n/src/tests.rs`):
`every_message_renders_in_every_language` (`:73-88`),
`shipped_languages_are_complete` (`:93-109`),
`no_language_defines_a_message_nobody_asks_for` (`:114-128`),
`variables_match_the_fallback` (`:134-172`),
`no_language_promises_more_than_the_product_does` (`:247-287`, forbidden
words at `:253-269`), `no_message_carries_a_character_layer_a_would_strip`
(`:298-314`, set at `:319-337` — **no U+00A0, U+2000–200F, U+202F, U+205F,
U+3000 in any translation**). Numbers: `fluent-bundle` 0.16's
`FluentNumber::as_string` (`types/number.rs:148-160` in the registry
source) does not group digits and `wipemark-i18n` installs no formatter, so a
count renders as plain ASCII digits — the CLI test in §5 still guards it.

**Core at `497eafa`** (E1-3 changes all of this): `UnicodeClass::ALL`
(`class.rs:75-87`) and ids (`:90-104`: `zero-width`, `zwj`, `bidi-control`,
`tag-character`, `variation-selector`, `soft-hyphen`, `exotic-space`,
`noncharacter`, `private-use`, `default-ignorable`, `homoglyph`);
`Confidence` (`:35-41`); `not_established::ALL` (`report.rs:40-44`: ids
`vendor-detector-evasion`, `human-authorship`, `unknown-mark-schemes`).

**Docs** — `docs/architecture/i18n.md:71-87` ("Which strings are localized"),
`:170-191` (gates table); `docs/architecture/layer-a.md` does not exist at
`497eafa` (E1-1 creates it); `README.md:267-274` (exit codes).

### 3.2 What E1-1…E1-5 delivered — the contract you build against

From `docs/plan/README.md` §3.3. Treat it as given; if HEAD differs,
implement against what HEAD has and record the difference in your report
(§0.2: you do not change the contract).

```rust
// wipemark_core (lib.rs re-exports)
pub const UNICODE_VERSION: &str;                         // "18.0.0", read from the UCD headers (D18)
pub fn name_of(c: char) -> Option<Cow<'static, str>>;    // D7: Some for every finding-capable code point
pub fn inspect(text: &str, options: &Options) -> InspectReport;
pub fn clean(text: &str, options: &Options) -> Cleaned;
pub struct Cleaned { pub text: String, pub report: CleanReport }
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Options { pub aggressive: bool, pub nfkc: bool, pub normalize_spaces: bool, pub keep_soft_hyphen: bool }
impl Options { pub fn action_for(&self, class: UnicodeClass) -> Action; }
pub enum Action { Remove, Replace, Keep }                 // as_str: "remove" | "replace" | "keep"
impl Confidence { pub fn as_str(self) -> &'static str; } // "confirmed" | "probable" | "informational" | "likely-false-positive"
impl UnicodeClass { pub fn as_str(self) -> &'static str; } // unchanged ids, §3.1
pub struct UnicodeFinding { pub codepoint: char, pub class: UnicodeClass, pub count: u32,
                            pub positions: Vec<usize>, pub confidence: Confidence }   // byte offsets into the SOURCE &str
pub struct InspectReport { pub findings: Vec<UnicodeFinding>, pub kept: Vec<UnicodeFinding>,
                           pub suspicious: bool, pub stats: TextStats, pub unicode_version: &'static str }
pub struct CleanReport { pub findings: Vec<UnicodeFinding>, pub kept: Vec<UnicodeFinding>,
                         pub removed: Vec<(UnicodeClass, u32)>, pub normalized: Vec<(NormKind, u32)>,
                         pub suspicious: bool, pub stats: TextStats,                 // over the SOURCE text (D28)
                         pub output_len: usize, pub unicode_version: &'static str }
impl InspectReport { pub fn to_json(&self) -> String; }  // A §7.1, not_established included (D9)
impl CleanReport   { pub fn to_json(&self) -> String; }
pub mod report::not_established { pub const ALL: [(&str, &str); 3]; }
```

What the contract guarantees and you rely on:

- **D3/D4/D6** — homoglyph detection always runs; `aggressive` only moves
  homoglyphs from `kept` (Keep, `Probable`) to `findings` (Replace).
  `suspicious` ⇔ some row of `findings ∪ kept` has confidence ≥ `Probable`.
  A row is in `findings` when acted on and in `kept` when not; the action of a
  `findings` row is `options.action_for(row.class)` (`Remove` or `Replace`),
  of a `kept` row `Keep`.
- **A §5.2** — `inspect` and `clean` are one decision over one pass: for the
  same text and options their `findings` and `kept` are equal (with `nfkc`
  too: what the later rounds remove goes to `removed`/`normalized` as counts
  without positions, A §5.3, D26), and so are their `suspicious` and `stats`,
  both computed over the **source** text (D28).
- **D26/D27** — with `nfkc` on, `clean` runs NFKC and the decision pass in
  rounds until a pass acts on nothing (at most 8); `normalized` then always
  carries `(Nfkc, n)`, `n` possibly 0.
- **A §4.1** — a U+FEFF at byte 0 is a byte order mark: not a finding, and
  **kept in `clean`'s output**. The CLI's BOM preservation (§4.4.3) rests on
  this.
- **D9** — `to_json()` writes the §7.1 form, std-only, *including*
  `"not_established": ["vendor-detector-evasion", "human-authorship",
  "unknown-mark-schemes"]` in every report; it contains no user text (ids,
  `U+XXXX`, UCD names, numbers).
- **D29** — the writer escapes every character from U+007F upward as `\u` +
  four lowercase hex digits, so every `to_json()` string is **pure ASCII by
  construction**. Tests below may assert it.
- **D28** — `CleanReport::to_json()` is A §7.1's clean form exactly,
  `suspicious` and `stats` included; the key order is the one E1-3 pins
  (`json_keys_come_in_the_documented_order`); every row carries `"action"`.

The exit codes therefore read core's own answer and never re-derive it:
`inspect` exits by `report.suspicious`, `clean` by
`cleaned.report.suspicious` (§4.4.7) — over the input, as A §7.3 asks, with
no second pass and no second spelling of D4 in the CLI.

## §4 Deliverables

### 4.1 `with_infix` moves into `wipemark-intake` (D10)

1. **`crates/wipemark-intake/src/name.rs`** gains, verbatim from
   `retention.rs` with their doc comments: `pub const RESULT_INFIX: &str =
   "cleaned";`, `pub const ORIGINAL_INFIX: &str = "original";`, `pub fn
   with_infix(name: &str, infix: &str) -> String` (body unchanged:
   `rsplit_once('.')`, infix before the last extension when the stem is
   non-empty, appended otherwise, never collapsed). Extend the module docs
   (`:1-17`) with a third question: *3. what a result made from this name is
   called* — "a file name is this crate's vocabulary, and the CLI and the
   app must spell a result the same way" — and keep the "formats, never
   localized" wording of the constants.
2. **`apps/wipemark-app/src/retention.rs`**: delete the three definitions
   (`:61-78`, `:457-473`) and put in their place, after the `use` block
   (`:54-59`):
   ```rust
   /// The result and set-aside names. They live in `wipemark_intake::name`
   /// because the CLI writes results too and must not depend on this crate
   /// (D10); re-exported so every caller here keeps its spelling.
   pub use wipemark_intake::name::{with_infix, ORIGINAL_INFIX, RESULT_INFIX};
   ```
   `plan` (`:400-437`) is unchanged. In the test module, drop
   `with_infix, ORIGINAL_INFIX, RESULT_INFIX` from the `use super::{…}` list
   (`:481-484`) once the two tests below have moved — an unused import is a
   clippy failure.
3. **Tests**: move `a_result_is_named_after_its_file` (`:523-539`) and
   `the_infixes_are_formats_and_stay_ascii` (`:826-832`) into `name.rs`'s
   test module, names unchanged. **Keep**
   `a_result_beside_a_file_is_never_the_file_itself` in `retention.rs` — it
   tests `plan`, and CLAUDE.md names it there. Add
   `with_infix_never_returns_the_name_it_was_given` to `name.rs` (§5).
4. **`crates/wipemark-intake/src/lib.rs:62-66`** ("Skeleton status"):
   recognising what arrived is still all this crate does; replace the
   sentence that says scrubbing is E1 with one that says the CLI and the MCP
   server now act on its verdict through `wipemark-core`, and that images
   (E11) and archives (Q-D2) are still only named.
5. **`apps/wipemark-cli/Cargo.toml`** gains `wipemark-intake.workspace =
   true` (with a one-line comment: the CLI reads a file the way a drop is
   read, and names its result the way the app does) and
   `serde_json.workspace = true` (§4.4.9). Replace the stale comment at
   `:27-29` with one that says what `wipemark-store` is for
   (`configured_language`, a read-only open of `wipemark.db`).
   `scripts/check-dep-direction.sh` needs no change (`:97`, `:42`).

### 4.2 MCP: the tools run (`apps/wipemark-app/src/mcp/protocol.rs`)

#### 4.2.1 Reading the arguments

`params.arguments`:

| what arrives | answer |
|---|---|
| absent, or `null` | treated as `{}` |
| present and not a JSON object | **protocol error** `-32602`, message `` tools/call `arguments` must be an object `` — the request fails the `CallToolRequest` shape itself, which both MCP revisions put at the protocol level |
| an object | checked as below; every problem found is collected, and any problem makes the call a **refusal result** (§4.2.3) |

The arguments each tool takes, and nothing else:

| tool | `text` | `aggressive` | `nfkc` |
|---|---|---|---|
| `inspect` | string, **required** | boolean, optional, default `false` | — (not taken) |
| `clean` | string, **required** | boolean, optional, default `false` | boolean, optional, default `false` |

Problems, in the order they are reported: `text` missing; `text` present
and not a string (`null` included); `aggressive` present and not a boolean
(`null` included); `nfkc` present and not a boolean; then every key the tool
does not take, **sorted** (so the message does not depend on whether
`serde_json` was built with `preserve_order`). An empty string `""` is valid
text: Layer A runs over it and reports nothing, which is a true report of a
scan that ran.

```rust
/// One call, read and checked.
struct Call {
    text: String,
    options: wipemark_core::Options,   // aggressive, nfkc from the call; normalize_spaces and
                                       // keep_soft_hyphen stay false on every surface in E1 (A §7.4, Q-A1)
}

/// What is wrong with a call, in the order the refusal says it.
enum Problem {
    Missing(&'static str),                                   // "text"
    WrongType { name: &'static str, wants: &'static str },   // wants: "a string" | "true or false"
    NotTaken(String),                                        // already spelled (§4.2.4)
}

fn read_call(tool: Tool, arguments: &Map<String, Value>) -> Result<Call, Vec<Problem>>;
```

`inspect` gets `Options { aggressive, ..Options::default() }`; `clean` gets
`Options { aggressive, nfkc, ..Options::default() }`.

#### 4.2.2 The results

`Tool::refusal()` is replaced by `Tool::run(&Call) -> Answer` and
`Tool::refuse(&[Problem]) -> Value`. `call()` becomes: name → tool (unchanged:
missing name and unknown tool stay protocol errors) → arguments object (or
`-32602`) → `read_call` → `run` or `refuse`.

```rust
const INTERNAL_ERROR: i64 = -32603;

impl Tool {
    fn run(self, call: &Call) -> Answer {
        let json = match self {
            Self::Inspect => wipemark_core::inspect(&call.text, &call.options).to_json(),
            Self::Clean => {
                let cleaned = wipemark_core::clean(&call.text, &call.options);
                // `to_string` of a `&str` cannot fail; the report goes in
                // verbatim so the text block keeps §7.1's key order.
                let text = serde_json::to_string(&cleaned.text).unwrap_or_default();
                format!(r#"{{"text":{text},"report":{}}}"#, cleaned.report.to_json())
            }
        };
        answered(json)
    }
}

/// The answer MCP asks for when a tool returns structured content: the
/// object, and the same object serialized in a text block "for backwards
/// compatibility" (MCP 2025-06-18, Tools › Structured Content).
fn answered(json: String) -> Answer {
    match serde_json::from_str::<Value>(&json) {
        Ok(structured) => Answer::Result(json!({
            "content": [{ "type": "text", "text": json }],
            "structuredContent": structured,
            "isError": false,
        })),
        // Unreachable while `to_json` is right; `an_mcp_report_is_json_a_client_can_parse`
        // is what keeps it so. A server that broke says so at the protocol level.
        Err(error) => {
            tracing::error!(%error, "MCP: a Layer A report is not JSON");
            Answer::Error { code: INTERNAL_ERROR, message: "the report could not be rendered".to_owned() }
        }
    }
}
```

The shapes, exactly:

| tool | `result.content[0].text` | `result.structuredContent` |
|---|---|---|
| `inspect` | `InspectReport::to_json()`, **verbatim** | that string parsed: the §7.1 object (`unicode_version`, `suspicious`, `findings`, `kept`, `stats`, `not_established`; no `removed`, `normalized`, `output_len`) |
| `clean` | `{"text":<cleaned, serde_json-escaped>,"report":<CleanReport::to_json() verbatim>}` | that string parsed: `{ "text": string, "report": §7.1 object }` |

Why the text block is assembled rather than re-serialized: `serde_json` in
this workspace is built with `preserve_order` in some builds and without it in
others (`Cargo.lock` lists `indexmap` among its dependencies; which features
are on depends on the packages built together), so `to_string(&parsed)`
would order keys differently from one build to the next. The text block is
what an older client shows a model; it should be the same bytes every time.

`positions` in both are byte offsets into the UTF-8 text **as the server
received it** (after JSON decoding) — not UTF-16 code units, which is what a
JavaScript client's string indices are. The descriptions say "byte offsets
into the UTF-8 text".

Layer A runs on the connection thread (`server.rs:201-211`); it is O(n) and a
body is at most 1 MiB, so no channel or executor is needed. A response can be
larger than the request (the text twice, plus positions); `reply` has no
size limit and the 5 s write deadline is ample on loopback.

#### 4.2.3 Refusals and errors

| case | answer | text (a format, English, never from the catalogue) |
|---|---|---|
| any `Problem` | result, `isError: true`, no `structuredContent` | `` `{tool}` did not run: {problems joined by "; "}. Its arguments are {list}. Refusing rather than answering — a report about text that was never read would say that nothing was found. `` |
| — `Missing("text")` | | `` the argument `text` is missing `` |
| — `WrongType` | | `` the argument `{name}` must be {wants} `` |
| — `NotTaken(n)` | | `` it takes no argument `{n}` `` |
| `arguments` not an object | error `-32602` | `` tools/call `arguments` must be an object `` |
| no `name` | error `-32602` (unchanged, `:261-266`) | `` tools/call needs a `name` `` |
| unknown tool | error `-32602` (unchanged shape, `:275-281`) | `unknown tool: {spelled name}. This build offers: inspect, clean` |
| body over 1 MiB | HTTP `413` from the transport (`server.rs:380`), before dispatch (**D13**) | — |
| report not JSON (a bug) | error `-32603` | `the report could not be rendered` |

`{list}` for `inspect`: `` `text` (a string, required), `aggressive` (true or false) ``;
for `clean` the same plus `` , `nfkc` (true or false) ``.

The refusal keeps the existing `{"content":[{"type":"text","text":…}],"isError":true}`
shape (`:160-172`). A refusal is never a report: no `structuredContent`, no
`findings`. That is the whole point of
`a_tool_that_cannot_run_refuses_rather_than_reporting_nothing` (D14).

Why `isError` results for argument problems when MCP 2025-06-18 lists
"invalid arguments" among *protocol* errors: A §7.2 asks for `isError`, the
existing `a_refusal_reaches_the_model_rather_than_the_client` (`:407-417`)
already pins it, and the next revision of the specification (2025-11-25,
Tools › Error Handling) moved exactly this case — "input validation errors"
— to tool execution errors "that language models can use to self-correct".
A model that forgot `text` reads the refusal and retries; a JSON-RPC error is
swallowed by the client. Only a request that is not a `CallToolRequest` at all
(`arguments` not an object) stays a protocol error.

#### 4.2.4 What the server says back — the invisible-character rule

CLAUDE.md "Nothing the MCP server says comes from the catalogue" exists so
that no answer carries a character Layer A removes. Today two answers echo
client-supplied strings verbatim — the unknown method (`:254`) and the
unknown tool (`:278`) — and this document adds a third (argument names). A
client that sends `"name":"cl` + U+200B + `ean"` gets the U+200B back.
Add one function and use it for all three:

```rust
/// A string the client sent, safe to say back: printable ASCII as itself,
/// anything else spelled as U+XXXX.
fn spelled(client: &str) -> String
```

Two things are said back *unspelled*, deliberately, and the test (§5) names
both:

- the JSON-RPC **`id`**, which the client must get back exactly
  (`the_id_comes_back_exactly_as_it_was_sent`, `:512-522`);
- the **cleaned text** of a `clean` result, which carries exactly what Layer A
  kept: a ZWJ inside an emoji sequence, a VS16 after an emoji base, a U+FEFF at
  byte 0. That is not a leak, it is the user's text; the test permits a
  forbidden character only where the same response's `report.kept` declares
  it (or, for U+FEFF, where the input began with one), and counts it. The
  rule is **not** weakened into "escape everything as `\uXXXX`": escaping
  would hide a character from this test without hiding it from the agent,
  which decodes the JSON — the test would go blind and the agent would not.
  D29 is not that trick: it makes the *report* ASCII, and the report carries
  no user text, so nothing is hidden by it; the cleaned text is the user's
  own and is serialized by `serde_json` as it is.

#### 4.2.5 The listing

- `inputSchema` gains `"additionalProperties": false` (it is now true: an
  extra argument is refused). `inspect`'s schema has no `nfkc` (unchanged).
- Descriptions are rewritten and the whole listing is **ASCII** (the em dash
  goes): a listing is what an agent reads to call this tool, and a
  non-ASCII letter in it is exactly the sort of thing this product flags.
  Proposed texts (formats, English; adjust the wording, keep the facts, the
  "1 MB", and never "undetectable"):

| where | text |
|---|---|
| `inspect` | `Report what Wipemark's deterministic Unicode scrubber finds in a piece of text, without changing anything: zero-width characters, bidi controls, tag characters, variation selectors, soft hyphens, unusual spaces, private-use and noncharacter code points, other invisible format characters, and letters borrowed from another script inside a word (homoglyphs). Each finding has its code point, its Unicode name, a confidence and every position as a byte offset into the UTF-8 text. Characters that carry real orthography - an emoji sequence, a Persian non-joiner - are listed as kept. Every report also lists what it does not establish. Takes up to about 1 MB of text.` |
| `clean` | `Remove the invisible Unicode from a piece of text and return the cleaned text with a report of exactly what was removed or replaced and where (byte offsets into the UTF-8 text you sent). Deterministic: the same text and options always give the same result, and cleaning the result again changes nothing. Characters that carry real orthography are kept and listed as kept; homoglyphs are replaced only with aggressive. Every report also lists what it does not establish. Takes up to about 1 MB of text.` |
| `text` | `The text itself, as a string - not a path. Up to about 1 MB; a larger request is refused whole, never truncated.` |
| `aggressive` on `inspect` | `List homoglyphs as findings clean would replace, rather than as kept. They are reported either way. Default false.` |
| `aggressive` on `clean` | `Also replace homoglyphs - a letter from another script inside a word, such as a Cyrillic letter that looks like a Latin one in an English word - with the matching letter of the word's own script. They are reported either way; higher false-positive rate, hence opt-in. Default false.` |
| `nfkc` | `Apply NFKC normalisation after cleaning, and clean again whatever NFKC uncovers until nothing is left to clean. Off by default: NFKC changes more than provenance marks (ligatures, full-width letters, superscripts), code included.` |

No `outputSchema` in E1: it is optional (2025-06-18, Tools › Output
Schema), and a schema written here would be a second description of the
§7.1 format that can drift from `json.rs` — the drift D9 exists to prevent.
No `title` either.

#### 4.2.6 Logging

On every answered call, one line, never the text (CLAUDE.md "Diagnostics go
to a file, and the document never does"):

```rust
tracing::info!(
    tool = self.name(),
    text = %wipemark_log::Elided::from(&call.text),
    aggressive = call.options.aggressive,
    nfkc = call.options.nfkc,
    findings,          // rows in `findings`, read off the report struct before `to_json`
    kept,              // rows in `kept`
    "MCP: tools/call answered"
);
```

On a refusal: `tracing::info!(tool, problems = ?kinds, "MCP: tools/call
refused")` where `kinds` are static strings (`"missing text"`, `"wrong type:
aggressive"`, `"not taken"`) — not the argument names, which are the
client's.

#### 4.2.7 Module docs

Rewrite `protocol.rs:8-24` ("The tools refuse, and say why") as "The tools
run, and refuse only what they cannot run": what each answers, why a refusal
is a result, the D14 test, and the invisible-character rule of §4.2.4. Rewrite
`mod.rs:20-28` ("What answers, and what refuses") to match. Keep `:26-37` of
`protocol.rs` ("Nothing here is localized").

### 4.3 The MCP pane's banner (D15)

1. **Extract** the decision in `state_of_the_server` (`settings.rs:5503-5546`)
   into a free function beside `on_screen` (`:6386`), in the shape of
   `models_banner`:
   ```rust
   /// The MCP page's banner, as values: what is running, and what the
   /// tools do. The last line never changes with the server — it is what
   /// the tools are, not what the socket is doing.
   fn mcp_banner(status: &Status) -> (IconName, Tone, Vec<String>)
   ```
   `Off`/`Starting` → `IconName::CircleInfo`, `Tone::Quiet`;
   `Listening` → `IconName::Plug`, `Tone::Good` (plus the `moved` line when
   `endpoint.port != wanted`, as `:5521-5532`); `Failed` →
   `IconName::TriangleExclamation`, `Tone::Bad`. Last line:
   `t(Message::SettingsMcpToolsLayerA)`. The colours are exactly today's
   (`Tone::colour`, `:6086-6094`, maps `Quiet/Good/Bad` to
   `muted_foreground/success/danger` — `:5508`, `:5513`, `:5533`, `:5537`).
   `state_of_the_server` becomes three lines: read the status, call
   `mcp_banner`, `notice(glyph, tone.colour(cx), lines, cx)`. Rewrite its doc
   comment (`:5494-5502`): the second fact is now that the tools clean and
   nothing rewrites.
2. **Catalogue**: add `settings-mcp-tools-layer-a` (texts in §4.5), delete
   `settings-mcp-tools-pending` from all three catalogues, and rewrite the
   en-US comments that name it (`:485-489` in the Engine section — "the same
   bargain `settings-mcp-tools-pending` and `cli-not-implemented` make" →
   the MCP banner's last line and `cli-not-implemented`; `:832-850` the MCP
   section header and the comment above the key).
3. **Test** `the_mcp_banner_always_says_what_the_tools_do` in `settings.rs`'s
   test module (§5). Fix the Placement gate's comment (`:7500-7505`) only if
   its wording becomes false — after this it is true.

### 4.4 The CLI (`apps/wipemark-cli`)

#### 4.4.1 Shape of the change

```
apps/wipemark-cli/src/
  main.rs     the argument surface, help, language, logging and dispatch — as today,
              minus the blanket refusal; `Exit` becomes pub(crate)
  input.rs    NEW  Source, reading, the intake verdict, decode/encode, same_file
  report.rs   NEW  the human report: labels, rows, the third shelf
  run.rs      NEW  inspect and clean: the two flows, where each output goes, the exit code
apps/wipemark-cli/tests/cli.rs   NEW  end-to-end through the built binary (env!("CARGO_BIN_EXE_wipemark-cli"))
```

`main` dispatches:

```rust
let exit = match &cli.command {
    Action::Inspect { path, json } => run::inspect(path, *json, &mut Io::standard()),
    Action::Clean { path, out, nfkc, aggressive, json } =>
        run::clean(path, out.as_deref(), *nfkc, *aggressive, *json, &mut Io::standard()),
    other => refuse(other),   // today's :529-549 moved into a function, unchanged
};
exit.into()
```

`run::Io { stdin: &mut dyn Read, stdout: &mut dyn Write, stderr: &mut dyn
Write }` so every flow is testable in-process with `Cursor`/`Vec<u8>`;
`Io::standard()` locks the three standard streams. Name clash: the CLI's
clap enum is `Action`; import `wipemark_core::Action` as `LayerAction` (or
qualify it). Remove `#[allow(dead_code, …)]` from `Exit` (`:52-55`): all four
codes are constructed now.

#### 4.4.2 Reading the input (`input.rs`)

```rust
pub(crate) enum Source { Stdin, File(PathBuf) }
impl Source { pub(crate) fn of(argument: &str) -> Self }   // "-" is Stdin, anything else a path

pub(crate) struct Read {
    pub text: String,             // decoded; a byte order mark, if there was one, is U+FEFF at byte 0
    pub encoding: Encoding,       // never Encoding::Other
    pub note: Option<(Format, Format)>,   // (what the name said, what the bytes are) — Evidence::Disagreed
}

pub(crate) enum Unread {
    Missing,                                         // exit 2
    Folder,                                          // exit 2
    Unreadable(std::io::Error),                      // exit 3
    NotText { found: Option<Format>, named: Option<Format> },   // exit 3
    UnnamedEncoding,                                 // Encoding::Other, exit 3
    Invalid { encoding: Encoding, offset: usize },   // exit 3
}

pub(crate) fn read(source: &Source, stdin: &mut dyn std::io::Read) -> Result<Read, Unread>;
```

For `Source::File(path)`, in this order — one open, the head read first so a
4 GB model file is refused after 4 KB, not after 4 GB:

1. `std::fs::metadata(path)`: `NotFound` → `Missing`; any other error →
   `Unreadable`; a directory → `Folder`.
2. `File::open` → error → `Unreadable`. Read at most `wipemark_intake::HEAD`
   bytes (`file.by_ref().take(HEAD as u64).read_to_end(&mut bytes)`) → error →
   `Unreadable`.
3. Zero bytes read and `metadata.len() == 0` → `Ok(Read { text: "", encoding:
   Utf8, note: None })` — the rule `of_path` keeps (`lib.rs:354-358`).
4. `let intake = wipemark_intake::identify(&bytes, file_name)` (pure,
   `lib.rs:207`; `file_name` = `path.file_name()` lossily). Do **not** use
   `of_path`: it reads the head itself and turns an unreadable file into a
   name-only answer without an error (`:324-326`), which would make an
   unreadable `notes.txt` look like "not text".
5. `!intake.is_textual()` → `NotText { found: intake.format, named:
   intake.contradicted() }`.
6. `intake.encoding`: `None` → `NotText`; `Some(Encoding::Other)` →
   `UnnamedEncoding`; `Some(e)` → go on.
7. Read the rest on the same handle (`file.read_to_end(&mut bytes)`) → error →
   `Unreadable`.
8. `decode(&bytes, e)` → `Err(offset)` → `Invalid { encoding: e, offset }`.
9. `note = intake.contradicted().zip(intake.format)`.

For `Source::Stdin`: read everything (`read_to_end`) → error → `Unreadable`;
empty → empty UTF-8 text; otherwise `identify(&bytes[..len.min(HEAD)], None)`
and steps 5–8.

Consequences, stated so nobody "fixes" them: a UTF-8 file whose first 4 KB
contain a NUL or a control such as BEL is *not text* to intake and exits 3
(C0 controls are not Layer A findings, A §2); a UTF-16 file without a BOM
whose head is not plain ASCII is not recognised and exits 3 (owner question
Q-D6, open); a control character after the first 4 KB of valid UTF-8 is read
normally.

#### 4.4.3 Decoding and encoding (D11)

```rust
pub(crate) fn decode(bytes: &[u8], encoding: Encoding) -> Result<String, usize>;  // Err = byte offset of the first bad unit
pub(crate) fn encode(text: &str, encoding: Encoding) -> Vec<u8>;
```

| encoding | decode | error offset | encode |
|---|---|---|---|
| `Utf8` | `std::str::from_utf8` | `valid_up_to()` (a sequence cut at the end of the file is an error too) | the bytes |
| `Utf16Le` / `Utf16Be` | `u16::from_{le,be}_bytes` per pair, `char::decode_utf16` | odd length: `len - 1`; a lone surrogate: `2 ×` the index of its unit | `encode_utf16()` → `to_{le,be}_bytes` |
| `Utf32Le` / `Utf32Be` | `u32::from_{le,be}_bytes` per quad, `char::from_u32` | length not a multiple of 4: `len - len % 4`; a surrogate or > U+10FFFF: `4 ×` the index | `u32::from(c).to_{le,be}_bytes()` |
| `Other` | never called (step 6 refuses first); return `Err(0)` | | |

**The byte order mark is never stripped.** Decoding keeps it as U+FEFF at
byte 0 (`FF FE` → U+FEFF, `EF BB BF` → U+FEFF); Layer A does not treat a
U+FEFF at byte 0 as a finding and keeps it in `clean`'s output (A §4.1);
encoding the output in the input's encoding then writes the same mark back.
That is how D11's "in the input's encoding, with its BOM if it had one" is
kept without a flag to carry around — and why, for a UTF-8 file, `positions`
are exactly the file's byte offsets. For UTF-16/UTF-32 input they are byte
offsets into the text *as UTF-8*, and the human report says so (§4.4.8).

Standard output is always UTF-8 (D11) and carries the text as decoded, a
leading U+FEFF included when the input had a mark — the mark is part of what
was read and Layer A kept it.

#### 4.4.4 `inspect`

```
inspect <path|-> [--json]
```

1. `read` → `Err(u)` → the refusal line for `u` on stderr (§4.5), exit 2
   (`Missing`, `Folder`) or 3 (the rest). Stdout stays **empty**.
2. A `note` → `cli-name-disagrees` on stderr (Q-D5: the file is read by its
   bytes and the exit code is decided by findings like any other's).
3. `let report = wipemark_core::inspect(&read.text, &Options::default());`
4. `--json`: `report.to_json()` and a newline on stdout, nothing else.
   Otherwise the human report (§4.4.8) on stdout.
5. Exit `1` if `report.suspicious`, else `0`. Any error writing or flushing
   stdout → `3` (the answer did not reach the caller), logged, never a panic:
   write through the `Io` handles with `write_all`, never `println!` (which
   panics on a closed pipe).

`inspect` never writes a file.

#### 4.4.5 `clean`

```
clean <path|-> [-o <out>|-o -] [--nfkc] [--aggressive] [--json]
```

1. **Decide where the result goes, before reading anything:**

   | input | `-o` | destination |
   |---|---|---|
   | a file | absent | **beside**: `input.with_file_name(with_infix(file_name, RESULT_INFIX))` — `note.md` → `note.cleaned.md`, `x.cleaned.md` → `x.cleaned.cleaned.md` |
   | a file or `-` | `-` | **stdout** |
   | a file or `-` | a path | **that file** |
   | `-` | absent | **stdout** |

   An `-o` path that is an existing folder → `cli-out-is-a-folder`, exit 2.
   An `-o` path that is the input file → `cli-out-is-input`, exit 2 —
   "the same file" by `same_file(input, out)`: on Unix the same `(dev,
   ino)` (`std::os::unix::fs::MetadataExt`), elsewhere equal
   `canonicalize()`; false when `out` does not exist. This catches
   `-o note.md`, `-o ./note.md`, a symlink and a hard link to the input.
   (`-o -` is this document's addition to D12: it generalises "stdin and no
   `-o`" to "the result goes to standard output".)
2. `read` → as `inspect` step 1–2.
3. `let options = Options { aggressive, nfkc, ..Options::default() };`
   `let cleaned = wipemark_core::clean(&read.text, &options);`
   — the exit code is `cleaned.report.suspicious`, which core computes over
   the **input** (D28, A §7.3), not over the result and not by a second
   spelling of D4 in the CLI.
4. **Write** (destination a file): `encode(&cleaned.text, read.encoding)`,
   written atomically — a temporary file in the destination's folder
   (`.<name>.wipemark-<pid>.tmp`, `File::create_new`), `write_all`, the
   input file's permissions copied onto it when the input was a file, then
   `std::fs::rename` over the destination; on any error remove the temporary
   file and report `cli-write-failed`, exit **3** (the work did not complete;
   never 0 or 1 for a result that does not exist). An existing
   `note.cleaned.md` from an earlier run is replaced — rename never writes
   into an existing inode, so even a hard link to the input is safe. The input
   file is never opened for writing.
5. **Destination stdout**: `cleaned.text` as UTF-8 bytes (without `--json`).
6. Report (§4.4.6), exit (§4.4.7). The result is always written, findings
   or not — a script expects the file to be there.

#### 4.4.6 Where each output goes

Standard output carries exactly **one** product: the JSON, or the cleaned
text, or the human report. The human report moves to stderr only when stdout
carries the text.

| command | `--json` | destination | stdout | stderr |
|---|---|---|---|---|
| `inspect` | no | — | human report | note, refusals |
| `inspect` | yes | — | `InspectReport::to_json()` + `\n` | note, refusals |
| `clean` | no | a file | human report (with "The result is in …") | note, refusals |
| `clean` | no | stdout | the cleaned text | human report, note, refusals |
| `clean` | yes | a file | `{"report":<§7.1>,"written":"<path>"}` + `\n` | note, refusals |
| `clean` | yes | stdout | `{"report":<§7.1>,"text":"<cleaned>"}` + `\n` | note, refusals |

On a refusal or a failure stdout stays empty: a parser that gets nothing
looks at the exit code, and the exit code says why.

#### 4.4.7 Exit codes

| code | when |
|---|---|
| `0` | the input was read in full and is not `suspicious`; for `clean`, the result was written |
| `1` | the input was read in full and is `suspicious` — for `clean` too, **even though the result no longer carries it** (A §7.3: a pre-commit hook wants to know what was there); for `clean`, the result was written |
| `2` | a usage error or a refusal: clap's own (unchanged), a path that does not exist, a folder, `-o` naming a folder or the input; and every invocation of `rewrite`, `models`, `audit` (unchanged) |
| `3` | inconclusive: exists but cannot be read, not text, an 8-bit encoding this version does not name, an invalid sequence, the result could not be written, stdout could not be written |

Rewrite the module header `main.rs:8-20` to say exactly this table, and
`README.md:267-274` likewise (`1  findings  the input carries something that
looks like a mark — for clean as well, after removing it`). Rewrite
`:35-39` ("Skeleton status") as "Status: `inspect` and `clean` run Layer A;
`rewrite`, `models` and `audit` refuse with 2 until their epics land".

#### 4.4.8 The human report (`report.rs`, `Rendering::PlainText`)

Every word comes from the catalogue; code points, Unicode names, format and
encoding names, paths and numbers are formats. Lines, in order:

1. **Summary**, one of three (`$source` is the path as typed, or
   `cli-report-stdin`; `$count` is the sum of `count` over `findings` and
   `kept`, passed as a number for plural selection):
   `cli-report-none` when both lists are empty; `cli-report-suspicious` when
   `suspicious`; `cli-report-noted` otherwise. Never a sentence that calls
   the text clean (A §7.5: `!suspicious` is not "clean" — the third shelf
   says what was not looked for).
2. **Rows, under headings**, rows in core's order within each heading:
   `findings` rows whose `options.action_for(class)` is `Remove` under
   `cli-report-would-remove` (inspect) / `cli-report-removed` (clean); `Replace`
   under `…-would-replace` / `…-replaced`; all `kept` rows under
   `…-would-keep` / `…-kept`. A heading with no rows is not printed. Each row
   is `"  "` + `cli-report-row` with `$character` = `"U+200B ZERO WIDTH
   SPACE"` (`format!("U+{:04X}", c as u32)`, a space, `name_of(c)`; the
   space and name omitted if `name_of` is `None`), `$class` =
   `t(class_label(row.class))`, `$confidence` = `t(confidence_label(row.confidence))`,
   `$count` = `row.count` (a number), `$positions` = the first **10**
   offsets joined by `", "`, or `cli-report-more` (`$shown`, `$more` as
   strings) when there are more.
3. `cli-report-offsets` (`$encoding` = `Encoding::name()`) when the input
   was not UTF-8.
4. `clean` only: `cli-clean-nfkc` when `--nfkc`; `cli-clean-written` (`$path`)
   when the destination is a file; `cli-clean-untouched` (`$source`) when the
   input was a file.
5. `cli-report-unicode` (`$version` = `report.unicode_version`, a string).
6. `report-not-established-title`, then one `"  - "` line per entry of
   `not_established::ALL`, in its order, each from
   `report-not-established-<id>`. The mapping id → `Message` is a `match` on
   the three ids whose fallback arm prints the canonical English beside the id
   — **an entry is never dropped**.

```rust
/// How a line is put into words. `t_args` in production; a per-language
/// `wipemark_i18n::Localizer::for_languages(…, Rendering::PlainText)` in the
/// test that renders every language — never `wipemark_i18n::init`, which is
/// process-wide and would race the other tests.
pub(crate) type Say<'a> = &'a dyn Fn(Message, &FluentArgs) -> String;

pub(crate) fn inspect_lines(say: Say, source: &str, report: &InspectReport,
                            options: &Options, encoding: Encoding) -> Vec<String>;
pub(crate) fn clean_lines(say: Say, source: &str, report: &CleanReport,
                          options: &Options, encoding: Encoding,
                          written: Option<&str>, untouched: bool) -> Vec<String>;
fn class_label(class: UnicodeClass) -> Message      // exhaustive match: ZeroWidth => Message::UnicodeClassZeroWidth, …
fn confidence_label(confidence: Confidence) -> Message   // exhaustive: Confirmed => Message::ConfidenceConfirmed, …
fn shelf_line(say: Say, id: &str, canonical: &str) -> String
```

Exhaustive matches, so a twelfth class fails to compile here as well as in
the i18n gate (§4.5). (E7 will need the same two mappings in the app; it
copies or lifts them then — a library cannot host them, CLAUDE.md "Only
applications localize".)

What `inspect note.md` prints for a file with three ZWSPs and one NBSP
(English catalogue):

```
note.md: 4 characters were found, and at least one of them looks like a mark.
Would be removed:
  U+200B ZERO WIDTH SPACE · zero-width character · confirmed · 3 times, at bytes 5, 19, 40
Would be kept:
  U+00A0 NO-BREAK SPACE · unusual space · for information · once, at byte 61
Checked against Unicode 18.0.0.
Not established:
  - evasion of a vendor's own detector — not tested, no oracle exists here
  - human authorship — not established by any check in this tool
  - marks in schemes this build does not implement — not searched for
```

and `clean note.md` the same with `Removed:`/`Kept:` and, before the Unicode
line, `The result is in note.cleaned.md.` and `note.md itself was not
changed.`

#### 4.4.9 `--json` (D12)

- `inspect --json`: `report.to_json()` verbatim, then `\n`.
- `clean --json`, destination a file: `format!(r#"{{"report":{},"written":{}}}"#,
  report_json, serde_json::to_string(&path_lossy))` — `"written"` is the
  destination as the CLI built it (relative if the input was relative),
  `to_string_lossy()`.
- `clean --json`, destination stdout: `format!(r#"{{"report":{},"text":{}}}"#,
  report_json, serde_json::to_string(&cleaned.text))`.
- Built by `format!` with core's report verbatim (key order stable, §4.2.2);
  `serde_json` only escapes the one string. The whole stdout is one JSON
  value — `jq` reads it.

#### 4.4.10 Log lines

The `tracing::info!(command, epic, "not implemented")` of `:533-537` moves
into `refuse()` and stays for `rewrite`, `models`, `audit` — `epic()` still
feeds it (CLAUDE.md: epic ids stay in the CLI's log line). `epic()`'s
`Inspect | Clean` arm stays so the match is total; make it return `"E1"`
with a comment that those two are never refused since E1-6. For `inspect`
and `clean`, one line per run, never the text and never the path in clear:

```rust
tracing::info!(
    command = "clean",
    input = %Elided::from(source_label),      // the path is a name the user did not choose to share
    encoding = read.encoding.name(),
    bytes = read.text.len(),
    findings = report.findings.len(),
    kept = report.kept.len(),
    suspicious, aggressive, nfkc,
    to = "beside" | "out" | "stdout",
    exit = exit as u8,
    "done"
);
```

A refusal or failure: `tracing::warn!(command, input = %Elided::from(…),
reason = "missing" | "folder" | "unreadable" | "not text" | "8-bit" |
"invalid" | "write failed" | "stdout", error = %io_error_kind_if_any, exit,
"not done")` — an `io::ErrorKind`, never the OS message (it can carry a
path). The stderr mirror stays off unless `WIPEMARK_LOG` is set (`:479`,
unchanged).

#### 4.4.11 What still refuses

`rewrite`, `models list|pull|verify|rm` and `audit` go through `refuse()`
exactly as today: the log line with `epic`, `cli-not-implemented` with
`summary()` on stderr, exit 2. `summary()` and `render_out()` stay (the
refusal still echoes every flag; `summary_echoes_every_flag` stays).

#### 4.4.12 Help

`command()`: give `clean`'s `path` the `CliArgPathOrStdin` help as
`inspect`'s has (`:380-383`), and rewrite the comment at `:240-247` (both
commands take `-`; `rewrite` does not yet). `cli-arg-out` and
`cli-arg-aggressive` change text (§4.5). Nothing else in the surface moves:
`arg_surface_is_well_formed` and `every_argument_and_subcommand_has_help`
must stay green untouched.

### 4.5 Catalogue

All three files get the same keys. Rules the texts below already keep:
no epic number; "not in this version (yet)" for anything absent; never
"undetectable", "guarantee", "100%" or their de/ru forms; no U+00A0 or other
exotic space, no soft hyphen; the terms `{ -layer-a }`/`{ -layer-b }` never
start a sentence (they are lower-case in en and inflect in ru); the same
`$variables` in every language. Numbers that select a plural form are passed
as numbers (`$count`); every other number (`$offset`, `$shown`, `$more`,
`$version`) as a string. Russian uses an exact `[1]` variant before `[one]`
where "one" would also catch 21, 31, … (`fluent-bundle` 0.16 tries the variants in order and takes the first
whose key matches — a number literal by value, an identifier by plural
category — `resolver/expression.rs:30-41` in the registry source; en-US
already uses `[0]` at `:710`).

**Delete** `settings-mcp-tools-pending` (en-US `:851`, de `:475`, ru `:488`).

**Change** `cli-arg-out` and `cli-arg-aggressive` (texts below).

**Add**, en-US — the CLI keys go after `cli-arg-language` (`:1001`), with
this group comment; the class and confidence keys after the third shelf
(`:932`); the MCP key where the deleted one was:

```ftl
## The reports `inspect` and `clean` print, and what they say when they
## cannot run.
##
## Read in a terminal, piped into a file, shown by a pre-commit hook — so
## plain text (`Rendering::PlainText`). What a machine reads is not here:
## `--json`, the code point and the character's Unicode name (an
## identifier of the standard, never translated — see
## docs/architecture/i18n.md), format and encoding names, and paths.
## $source is a path exactly as typed, or `cli-report-stdin`.

cli-report-stdin = standard input
# The first line of every report: nothing at all, things noted that are
# not likely marks, or at least one likely mark. Never "clean": what was
# not looked for is the third shelf at the bottom.
cli-report-none = { $source }: none of the characters this version looks for were found.
cli-report-noted = { $source }: { $count ->
        [one] one character was found, and it is not a likely mark.
       *[other] { $count } characters were found, and none of them is a likely mark.
    }
cli-report-suspicious = { $source }: { $count ->
        [one] one character was found, and it looks like a mark.
       *[other] { $count } characters were found, and at least one of them looks like a mark.
    }
# Headings over the rows. `inspect` says what would happen; `clean` what did.
cli-report-would-remove = Would be removed:
cli-report-would-replace = Would be replaced:
cli-report-would-keep = Would be kept:
cli-report-removed = Removed:
cli-report-replaced = Replaced:
cli-report-kept = Kept:
# One row. $character is the code point and its Unicode name
# ("U+200B ZERO WIDTH SPACE"), never translated; $class and $confidence
# are the `unicode-class-*` and `confidence-*` lines; $positions is a
# list of byte offsets, already spelled.
cli-report-row = { $character } · { $class } · { $confidence } · { $count ->
        [one] once, at byte { $positions }
       *[other] { $count } times, at bytes { $positions }
    }
# When a row has more offsets than are shown. $shown is the list shown.
cli-report-more = { $shown } and { $more } more
# Only when the input was not UTF-8. $encoding is UTF-16LE and the like.
cli-report-offsets = Byte offsets count the text as UTF-8; the input was { $encoding }.
cli-report-unicode = Checked against Unicode { $version }.
# $path is where the result went, as the operating system spells it.
cli-clean-written = The result is in { $path }.
cli-clean-untouched = { $source } itself was not changed.
cli-clean-nfkc = NFKC normalisation was applied as well; whatever it uncovered was cleaned in further passes, and --json counts those without positions.

## Why `inspect` or `clean` did not run, or did not finish. Each ends the
## run: the ones about arguments with exit 2, the rest with exit 3 —
## "not read is not clean". $reason is the operating system's own words,
## never translated.

cli-no-such-file = { $path } does not exist.
cli-is-a-folder = { $path } is a folder. inspect and clean read one file, or standard input; walking a folder is not in this version yet.
cli-out-is-a-folder = --out names a folder, { $path }. It takes the name of a file.
cli-out-is-input = --out names the file being read, { $path }. Writing over the input needs a flag of its own, and this version does not have one yet.
cli-unreadable = { $path } could not be read: { $reason }. Not read is not clean.
# $format is a format name such as PDF or PNG, never translated.
cli-not-text = { $path }: the contents are { $format }, not text, so there is nothing for { -layer-a } to read. Not read is not clean.
cli-not-text-unknown = { $path } is not text in any encoding this version reads. Not read is not clean.
cli-unnamed-encoding = { $path } is text in an 8-bit encoding this version does not name. Save it as UTF-8 and run again; until then it is not read, and not read is not clean.
# $encoding is UTF-8, UTF-16LE and the like; $offset a byte offset.
cli-invalid-encoding = { $path } is not valid { $encoding } at byte { $offset }. Not read is not clean.
# A note, not a failure: the file is read by what it contains.
cli-name-disagrees = { $path }: named as { $named }, and the contents are { $found }; it was read by its contents.
cli-write-failed = { $path } could not be written: { $reason }. The result was not saved.

## What a finding is, and how sure — the eleven classes of
## `wipemark_core::UnicodeClass` and the four confidences, keyed by their
## stable ids (`UnicodeClass::as_str`, `Confidence::as_str`). The ids are
## formats; these are the words. A character's own name is not here — it
## is an identifier of the Unicode standard and is never translated.

unicode-class-zero-width = zero-width character
unicode-class-zwj = zero-width joiner
unicode-class-bidi-control = bidirectional control
unicode-class-tag-character = tag character
unicode-class-variation-selector = variation selector
unicode-class-soft-hyphen = soft hyphen
unicode-class-exotic-space = unusual space
unicode-class-noncharacter = noncharacter
unicode-class-private-use = private-use character
unicode-class-default-ignorable = ignorable format character
unicode-class-homoglyph = letter from another script

confidence-confirmed = confirmed
confidence-probable = probable
confidence-informational = for information
confidence-likely-false-positive = likely not a mark

# The MCP banner's last line, in every state of the server: what the two
# tools do, and that nothing rewrites.
settings-mcp-tools-layer-a = Both tools run: inspect lists what { -layer-a } would change in a text, and clean makes those changes and reports each one with its position. Nothing is rewritten: { -layer-b } is not in this version yet.
```

Changed en-US lines:

```ftl
cli-arg-out = Output file, or `-` for standard output. Defaults to `<name>.cleaned.<ext>` beside the input, and to standard output when the input is standard input; in-place needs an explicit flag, never a default.
cli-arg-aggressive = Also replace a letter borrowed from another script inside a word (a homoglyph). Such letters are reported either way; higher false-positive rate, hence opt-in.
```

**de** (proposed; a German reviewer may polish wording, not facts):

```ftl
cli-report-stdin = Standardeingabe
cli-report-none = { $source }: Keines der Zeichen, nach denen diese Version sucht, wurde gefunden.
cli-report-noted = { $source }: { $count ->
        [one] ein Zeichen gefunden, und es ist wahrscheinlich keine Markierung.
       *[other] { $count } Zeichen gefunden, und keines davon ist wahrscheinlich eine Markierung.
    }
cli-report-suspicious = { $source }: { $count ->
        [one] ein Zeichen gefunden, und es sieht nach einer Markierung aus.
       *[other] { $count } Zeichen gefunden, und mindestens eines davon sieht nach einer Markierung aus.
    }
cli-report-would-remove = Würde entfernt:
cli-report-would-replace = Würde ersetzt:
cli-report-would-keep = Würde behalten:
cli-report-removed = Entfernt:
cli-report-replaced = Ersetzt:
cli-report-kept = Behalten:
cli-report-row = { $character } · { $class } · { $confidence } · { $count ->
        [one] einmal, bei Byte { $positions }
       *[other] { $count }-mal, bei den Bytes { $positions }
    }
cli-report-more = { $shown } und { $more } weitere
cli-report-offsets = Die Byte-Positionen zählen den Text als UTF-8; die Eingabe war { $encoding }.
cli-report-unicode = Geprüft gegen Unicode { $version }.
cli-clean-written = Das Ergebnis steht in { $path }.
cli-clean-untouched = { $source } selbst wurde nicht verändert.
cli-clean-nfkc = Zusätzlich wurde die NFKC-Normalisierung angewendet; was sie freigelegt hat, wurde in weiteren Durchgängen bereinigt, und --json zählt es ohne Positionen.
cli-no-such-file = { $path } existiert nicht.
cli-is-a-folder = { $path } ist ein Ordner. inspect und clean lesen eine Datei oder die Standardeingabe; einen Ordner zu durchlaufen gibt es in dieser Version noch nicht.
cli-out-is-a-folder = --out nennt einen Ordner, { $path }. Erwartet wird der Name einer Datei.
cli-out-is-input = --out nennt die Datei, die gelesen wird, { $path }. Die Eingabe zu überschreiben braucht ein eigenes Flag, und diese Version hat noch keines.
cli-unreadable = { $path } konnte nicht gelesen werden: { $reason }. Nicht gelesen heißt nicht sauber.
cli-not-text = { $path }: Der Inhalt ist { $format }, kein Text, also hat die { -layer-a } hier nichts zu lesen. Nicht gelesen heißt nicht sauber.
cli-not-text-unknown = { $path } ist in keiner Kodierung Text, die diese Version liest. Nicht gelesen heißt nicht sauber.
cli-unnamed-encoding = { $path } ist Text in einer 8-Bit-Kodierung, die diese Version nicht benennt. Als UTF-8 speichern und erneut ausführen; bis dahin ist die Datei nicht gelesen, und nicht gelesen heißt nicht sauber.
cli-invalid-encoding = { $path } ist bei Byte { $offset } kein gültiges { $encoding }. Nicht gelesen heißt nicht sauber.
cli-name-disagrees = { $path }: Der Name verspricht { $named }, der Inhalt ist { $found }; gelesen wurde nach dem Inhalt.
cli-write-failed = { $path } konnte nicht geschrieben werden: { $reason }. Das Ergebnis wurde nicht gespeichert.

unicode-class-zero-width = breitenloses Zeichen
unicode-class-zwj = breitenloser Verbinder
unicode-class-bidi-control = Steuerzeichen der Schreibrichtung
unicode-class-tag-character = Tag-Zeichen
unicode-class-variation-selector = Variantenselektor
unicode-class-soft-hyphen = bedingter Trennstrich
unicode-class-exotic-space = ungewöhnliches Leerzeichen
unicode-class-noncharacter = Nichtzeichen
unicode-class-private-use = Zeichen für private Nutzung
unicode-class-default-ignorable = ignorierbares Formatzeichen
unicode-class-homoglyph = Buchstabe aus einer anderen Schrift

confidence-confirmed = bestätigt
confidence-probable = wahrscheinlich
confidence-informational = zur Information
confidence-likely-false-positive = wahrscheinlich keine Markierung

settings-mcp-tools-layer-a = Beide Werkzeuge arbeiten: inspect zeigt, was die { -layer-a } an einem Text ändern würde, und clean nimmt diese Änderungen vor und meldet jede mit ihrer Position. Umgeschrieben wird nichts: { -layer-b } gibt es in dieser Version noch nicht.

cli-arg-out = Ausgabedatei, oder `-` für die Standardausgabe. Standard ist `<name>.cleaned.<ext>` neben der Eingabe, und die Standardausgabe, wenn die Eingabe die Standardeingabe ist; direktes Überschreiben braucht ein ausdrückliches Flag und ist nie der Standard.
cli-arg-aggressive = Auch einen Buchstaben aus einer anderen Schrift innerhalb eines Wortes ersetzen (ein Homoglyph). Gemeldet werden solche Buchstaben in jedem Fall; höhere Falsch-positiv-Rate, deshalb nur auf Wunsch.
```

**ru** (proposed):

```ftl
cli-report-stdin = стандартный ввод
cli-report-none = { $source }: ни одного из символов, которые ищет эта версия, не найдено.
cli-report-noted = { $source }: { $count ->
        [1] найден один символ, и он, скорее всего, не метка.
        [one] найден { $count } символ, и ни один из них, скорее всего, не метка.
        [few] найдено { $count } символа, и ни один из них, скорее всего, не метка.
       *[other] найдено { $count } символов, и ни один из них, скорее всего, не метка.
    }
cli-report-suspicious = { $source }: { $count ->
        [1] найден один символ, и он похож на метку.
        [one] найден { $count } символ, и хотя бы один из них похож на метку.
        [few] найдено { $count } символа, и хотя бы один из них похож на метку.
       *[other] найдено { $count } символов, и хотя бы один из них похож на метку.
    }
cli-report-would-remove = Будет удалено:
cli-report-would-replace = Будет заменено:
cli-report-would-keep = Будет оставлено:
cli-report-removed = Удалено:
cli-report-replaced = Заменено:
cli-report-kept = Оставлено:
cli-report-row = { $character } · { $class } · { $confidence } · { $count ->
        [1] один раз, на байте { $positions }
        [one] { $count } раз, на байтах { $positions }
        [few] { $count } раза, на байтах { $positions }
       *[other] { $count } раз, на байтах { $positions }
    }
cli-report-more = { $shown } и ещё { $more }
cli-report-offsets = Смещения в байтах считаются по тексту в UTF-8; на входе был { $encoding }.
cli-report-unicode = Проверено по Unicode { $version }.
cli-clean-written = Результат — в { $path }.
cli-clean-untouched = Сам файл { $source } не изменён.
cli-clean-nfkc = Применена и нормализация NFKC; всё, что она обнажила, вычищено следующими проходами, и --json считает это без позиций.
cli-no-such-file = { $path } не существует.
cli-is-a-folder = { $path } — это папка. inspect и clean читают один файл или стандартный ввод; обхода папок в этой версии пока нет.
cli-out-is-a-folder = --out указывает на папку, { $path }. Нужно имя файла.
cli-out-is-input = --out указывает на читаемый файл, { $path }. Запись поверх исходника требует отдельного флага, а его в этой версии пока нет.
cli-unreadable = { $path } не удалось прочитать: { $reason }. Не прочитано — не значит чисто.
cli-not-text = { $path }: внутри { $format }, а не текст, — { -layer-a } здесь неприменима. Не прочитано — не значит чисто.
cli-not-text-unknown = { $path } — не текст ни в одной из кодировок, которые читает эта версия. Не прочитано — не значит чисто.
cli-unnamed-encoding = { $path } — текст в 8-битной кодировке, которую эта версия не называет. Сохраните его в UTF-8 и запустите снова; до тех пор он не прочитан, а не прочитано — не значит чисто.
cli-invalid-encoding = { $path }: на байте { $offset } — недопустимая последовательность { $encoding }. Не прочитано — не значит чисто.
cli-name-disagrees = { $path }: имя обещает { $named }, а внутри — { $found }; прочитано по содержимому.
cli-write-failed = { $path } не удалось записать: { $reason }. Результат не сохранён.

unicode-class-zero-width = символ нулевой ширины
unicode-class-zwj = соединитель нулевой ширины
unicode-class-bidi-control = управляющий символ направления письма
unicode-class-tag-character = символ-тег
unicode-class-variation-selector = селектор варианта
unicode-class-soft-hyphen = мягкий перенос
unicode-class-exotic-space = нетипичный пробел
unicode-class-noncharacter = несимвол
unicode-class-private-use = символ для частного использования
unicode-class-default-ignorable = игнорируемый символ форматирования
unicode-class-homoglyph = буква из другой письменности

confidence-confirmed = подтверждено
confidence-probable = вероятно
confidence-informational = для сведения
confidence-likely-false-positive = скорее всего, не метка

settings-mcp-tools-layer-a = Оба инструмента работают: inspect показывает, что { -layer-a } изменила бы в тексте, а clean вносит эти изменения и сообщает о каждом вместе с позицией. Ничего не переписывается: { -layer-b } в этой версии ещё не появилось.

cli-arg-out = Файл вывода или `-` для стандартного вывода. По умолчанию `<name>.cleaned.<ext>` рядом с исходным, а при чтении из стандартного ввода — стандартный вывод; запись поверх исходника требует явного флага и никогда не делается по умолчанию.
cli-arg-aggressive = Заменять также букву из другой письменности внутри слова (омоглиф). О таких буквах сообщается в любом случае; выше доля ложных срабатываний, поэтому только по требованию.
```

Key count: **+44** (28 CLI, 11 class, 4 confidence, 1 MCP), **−1**; 421 →
464 per language.

**The i18n gate.** Add to `crates/wipemark-i18n/src/tests.rs`, in "Product
rules", beside `the_third_shelf_is_never_empty_in_any_language` (it has
`wipemark-core` as a dev-dependency for exactly this):
`every_unicode_class_and_confidence_reads_in_every_language` — for every
`UnicodeClass::ALL` member, `unicode-class-{as_str}` is a `Message` and every
shipped language defines it; the same for `confidence-{as_str}` over the four
confidences (listed through an exhaustive `match` helper, as `class.rs:173-187`
does for classes, so a fifth confidence fails to compile there).

### 4.6 Docs

1. **`docs/architecture/i18n.md`** (D16). After "Which strings are
   localized" (`:71-87`) add a subsection *"Character names: the one exception
   that reads like prose"*: `ZERO WIDTH SPACE` is what
   `wipemark_core::name_of` returns for U+200B — the character's `Name`
   property in the Unicode Character Database, an identifier of the standard
   like the code point itself and like `Vendor::as_str()` — so it is printed
   beside its `U+XXXX`, in monospace where a window has monospace, never
   translated and never put in the catalogue (a translated name is one no
   reference, search engine or other tool knows). Code points without a name
   use the UCD code point labels (`<private-use-E000>`,
   `<noncharacter-FDD0>`, `<reserved-E0080>`), formats too. What is localized
   is everything around it: the class (`unicode-class-<id>`) and the
   confidence (`confidence-<id>`). `--json` and MCP carry the name as `name`
   and never a class label. Basis: A §5.5, owner question Q-A6, plan decision
   D16; if the owner vetoes, windows show only `U+XXXX` and the class, and
   `name_of` stays for `--json` and MCP. Add rows to the gates table
   (`:170-186`) for `every_unicode_class_and_confidence_reads_in_every_language`,
   and to the paragraph after it (`:188-191`) for the CLI's
   `the_human_report_carries_no_character_layer_a_would_strip`.
2. **`CLAUDE.md`**, exactly these four places and nothing else:
   - "Every string a person reads comes from the catalogue" (`:220-225`) —
     append the D16 sentence: *"One thing reads like prose and is not: a
     character's Unicode name (`ZERO WIDTH SPACE`, from
     `wipemark_core::name_of`) is an identifier of the standard, shown beside
     its `U+XXXX` and never translated — the class and the confidence beside it
     are what the catalogue localizes (`unicode-class-*`, `confidence-*`); see
     `docs/architecture/i18n.md`."*
   - "The MCP server answers; its tools refuse" (`:640-651`) — this
     document makes it false; replace it with a rule titled *"The MCP server
     answers, and its tools run Layer A"*: `tools/call` runs
     `wipemark_core::inspect`/`clean` and answers with the §7.1 report as
     `content[0].text` and `structuredContent` (`clean`: `{"text","report"}`);
     a call it cannot run is refused as a *result* with `isError: true` that
     names the argument, never answered with an empty report —
     `a_tool_that_cannot_run_refuses_rather_than_reporting_nothing` is the
     gate, for the same reason as before; every report carries the third shelf;
     the text limit is the transport's 1 MiB `413`, never a truncation; a
     string the client sent is said back spelled (`U+XXXX`), except the `id`
     and the kept characters of a cleaned text.
   - "No epic number leaves this repository" (`:209-215`) — the list of
     pending surfaces loses "the MCP refusal" (there is none now for the work
     itself); "the MCP tools pane" stays (it says rewriting is not in this
     version) and "the CLI's" stays (`rewrite`, `models`, `audit`).
   - The sentence about `apps/wipemark-cli/src/main.rs` (`:183-185`) — add
     the three new modules: `input.rs` (reading and decoding through
     `wipemark-intake`), `report.rs` (the human report), `run.rs` (the two
     flows and their exit codes).
3. **`docs/architecture/layer-a.md`** — add the section **"Surfaces"**
   (§0.7 item 3), written for someone working on this code next year:
   the two callers and where they live; the MCP argument table, result shapes
   and refusals (§4.2.1–4.2.3), the invisible-character rule and its two
   exceptions (§4.2.4), the limit (D13); the CLI reading order and why the
   head is read first (§4.4.2), the encodings and the BOM mechanism
   (§4.4.3), where each output goes (§4.4.6), the exit codes with the
   "input decides" rule for `clean` (§4.4.7), the JSON shapes (D12 and the
   `-o -` generalisation); positions (UTF-8 byte offsets; file offsets for
   UTF-8 input; not UTF-16 code units for MCP clients); the third shelf in
   both; character names (D16, pointing at i18n.md); what still refuses.
4. **`docs/architecture/retention.md`**, rule 1 (`:59-67`): one sentence —
   the CLI writes beside by the same `with_infix`, which lives in
   `wipemark_intake::name` so the two applications cannot spell a result
   differently; the CLI reads none of the Retention rows (rule 3, unchanged).
5. **`README.md`** (root), the exit-code table (`:267-274`) per §4.4.7. The
   rest of the README (`:20-27` "What this build is", `:112-130` the MCP
   section) is E1-7's.
6. **`docs/plan/README.md`** — your status row only (§0.7 item 4).

### 4.7 `Cargo.lock` moves, once, on purpose

§0.5 says "`Cargo.lock` must not move — `wipemark-core` gains no dependency".
That sentence is about `wipemark-core`; **this** document adds
`wipemark-intake` and `serde_json` to `wipemark-cli`, and `Cargo.lock` records
each workspace member's dependency list, so `--locked` fails until the lock
is updated. Do it once and deliberately: after editing
`apps/wipemark-cli/Cargo.toml`, run `cargo check -p wipemark-cli` (without
`--locked`), then `git diff Cargo.lock` must show **only** the
`wipemark-cli` entry's `dependencies` list gaining `"serde_json"` and
`"wipemark-intake"` — no version moved, no package added. Anything else:
stop and report. Commit the lock with the code; then every §0.5 command
runs with `--locked` and passes.

### 4.8 Report

`docs/plan/reports/E1-6-<YYYY-MM-DD>.md` per §0.7 item 5, including: the
mutation table of §5; the `Cargo.lock` diff; the test count before and after;
anything in the E1-3 JSON that differs from §3.2 (D28's `suspicious` and
`stats` in the clean form, D29's ASCII, the key order); every deviation from
this document; and **the release-binary size delta**.

The size delta was planned for E1-1's checkpoint (`docs/plan/README.md`
§3.1 item 4, §6 "The binary size budget"), but it means nothing until a
binary calls the tables — and this is the document in which one first does.
Measure it here, on two states of the tree: **before your first edit** (the
checkout is then the commit before yours — E1-4's, or the merge of E1-5 if
that came later; record its SHA) and **after your last edit**.

```sh
# 1. before any edit — `git status` clean apart from ` m vendor/gpui-component`
git rev-parse --short HEAD
cargo build --release -p wipemark-cli -p wipemark-app --locked
ls -l target/release/wipemark-cli target/release/wipemark        # record both sizes in bytes
# 2. after the last edit, with the gates green
cargo build --release -p wipemark-cli -p wipemark-app --locked
ls -l target/release/wipemark-cli target/release/wipemark
```

Record both pairs and the deltas in the report beside OV §8's budget (< 60
MB without models). The app's release build compiles GPUI with `lto =
"thin"` and `codegen-units = 1` and takes long; run it in the background
while you read, and never launch the result.

## §5 Tests

RED first. "Mutation" is the change you apply locally to prove the test can
fail; record each in the report and restore the code. Tests with no mutation
listed guard shape rather than a protection; they still must fail when their
subject is deleted.

| test | file | input → expected | mutation that must turn it red |
|---|---|---|---|
| `a_result_is_named_after_its_file` (moved) | `crates/wipemark-intake/src/name.rs` | unchanged assertions (`report.docx`→`report.cleaned.docx`, `archive.tar.gz`→`archive.tar.cleaned.gz`, `README`→`README.cleaned`, `.bashrc`→`.bashrc.cleaned`, `photo.original.png`→`photo.original.original.png`) | put the infix after the last extension |
| `the_infixes_are_formats_and_stay_ascii` (moved) | same | unchanged | — |
| `with_infix_never_returns_the_name_it_was_given` | same | for `x.cleaned.md`, `x.md`, `x`, `.x`, `x.`, `a.b.c`, `""`, `x.cleaned`, and a Cyrillic stem: `with_infix(n, RESULT_INFIX) != n` and the same for `ORIGINAL_INFIX` | collapse an infix the name already carries (return `name` when `stem` ends in `.cleaned`) |
| `a_result_beside_a_file_is_never_the_file_itself` (kept) | `apps/wipemark-app/src/retention.rs` | unchanged — now through the re-export | — |
| `a_tool_that_cannot_run_refuses_rather_than_reporting_nothing` (**rewritten, D14**) | `apps/wipemark-app/src/mcp/protocol.rs` | for each tool, `tools/call` with `"arguments":{}` and with no `arguments` key → `result.isError == true`; `content[0].text` contains the tool name and `` `text` ``; `result` has **no** `structuredContent` | treat a missing `text` as `""` (the tool then answers with an empty report) |
| `a_refusal_reaches_the_model_rather_than_the_client` (kept) | same | unchanged (`clean` with `{}` is a result, not an `error`) | answer a missing argument with `-32602` |
| `an_argument_of_the_wrong_type_is_refused_by_name` | same | `{"text":5}`, `{"text":null}`, `{"text":"a","aggressive":"yes"}`, `{"text":"a","aggressive":null}`, clean `{"text":"a","nfkc":1}` → `isError`, the message names the argument (`` `text` ``, `` `aggressive` ``, `` `nfkc` ``) | coerce: `as_bool().unwrap_or(false)` for flags |
| `an_argument_the_tool_does_not_take_is_refused_by_name` | same | inspect `{"text":"a","nfkc":true}`, clean `{"text":"a","aggresive":true}` → `isError`, names `` `nfkc` `` / `` `aggresive` ``, lists the tool's arguments; `tools/list` schemas carry `"additionalProperties": false` | ignore unknown keys |
| `arguments_that_are_not_an_object_are_a_protocol_error` | same | `"arguments":"text"` and `"arguments":[1]` → `error.code == -32602`, no `result` | treat a non-object as `{}` |
| `a_clean_result_carries_the_cleaned_text_and_its_report` | same | clean `"a\u{200B}b"` → `structuredContent.text == "ab"`, `report.removed == {"zero-width":1}`, `report.findings[0]` has `codepoint "U+200B"`, `name "ZERO WIDTH SPACE"`, `positions [1]`, `isError == false` | return the input text unchanged |
| `an_mcp_report_is_json_a_client_can_parse` | same | for both tools over `"a\u{200B}b"`, `""`, `"p\u{0430}y"` × `aggressive` ∈ {false, true} (× `nfkc` for clean): `content[0].text` parses and **equals** `structuredContent`; `structuredContent` is an object; inspect has `unicode_version == wipemark_core::UNICODE_VERSION`, `suspicious`, `findings`, `kept`, `stats`; clean has exactly the keys `text` and `report`, and `report` has `suspicious`, `stats`, `findings`, `kept`, `removed`, `normalized`, `output_len`, `unicode_version` (D28); inspect's `content[0].text` is ASCII (D29) | put the report into `structuredContent` as a *string* (unparsed) |
| `every_layer_a_answer_carries_the_third_shelf` (MCP) | same | the same inputs: inspect's `structuredContent.not_established` and clean's `structuredContent.report.not_established` equal `not_established::ALL` ids, in order | strip `not_established` from the parsed value before answering; separately, delete the field in E1-3's `json.rs` (both red) |
| `inspect_does_not_change_anything` | same | inspect result has no `text` anywhere in `structuredContent` and no `removed`/`normalized`/`output_len`; for the same text and `aggressive`, inspect's `findings`/`kept` arrays equal clean's `report.findings`/`report.kept` | answer inspect with clean's `{"text","report"}` |
| `nothing_the_server_says_carries_an_invisible_character` (**extended**) | same | the ten existing requests (the two "no arguments" tool calls now hit the missing-argument refusal) **plus**: inspect and clean (each `aggressive` false/true, clean `nfkc` false/true) over `"a\u{200B}b\u{202E}c\u{E0041}d"`; a method `"ping\u{200B}"`; a tool `"cl\u{200B}ean"`; an argument `"te\u{200B}xt"` → no forbidden character anywhere in the body. **Second half**, the declared exceptions: clean over `"\u{1F469}\u{200D}\u{1F469}\u{200D}\u{1F467}\u{200D}\u{1F466} a\u{200B}b"` → the body's forbidden characters are exactly U+200D, **6** times (3 kept × the text block and `structuredContent`), and the same response's `report.kept` lists `U+200D` with `count 3`; clean over `"\u{FEFF}a\u{200B}b"` → exactly U+FEFF, twice | echo the unknown tool name unspelled (`:278`); separately, remove the ZWJ rule in E1-2 (the family loses its ZWJs and `kept` is empty: red) |
| `the_tool_listing_says_what_it_takes_and_promises_nothing_more` | same | the whole `tools/list` body `is_ascii()`; each description contains `"1 MB"` and `"byte offset"`; none contains `"undetect"` (any case); inspect's schema has no `nfkc` | put the old `"homoglyphs and exotic spaces"` description back with its em dash |
| `every_tool_is_listed_with_something_an_agent_can_act_on`, `no_two_tools_read_the_same`, `a_tool_this_build_does_not_have_is_refused_at_the_protocol_level`, `the_id_comes_back_exactly_as_it_was_sent` (kept) | same | unchanged | — |
| `a_body_over_the_limit_is_refused_whole` | `apps/wipemark-app/src/mcp/server.rs` | a running server (`start` on `a_port_with_a_free_neighbour()`), a `POST /mcp` whose head says `Content-Length: 1048577` and which sends no body → the answer starts `HTTP/1.1 413` and arrives without the server waiting for the body; a `tools/call inspect` of exactly `LARGEST_BODY` bytes → `200` and a report. D13 makes this the text limit and the new tool descriptions promise it; at `497eafa` nothing tests it | delete the `head.length > LARGEST_BODY` check (`server.rs:380`): the first half goes red; make it `>=`: the second half goes red |
| `the_mcp_banner_always_says_what_the_tools_do` | `apps/wipemark-app/src/settings.rs` | for `Off`, `Starting`, `Listening` (port as wanted), `Listening` (moved), `Failed("Address already in use")`: last line `reads_as(…, Message::SettingsMcpToolsLayerA)`; line counts 2, 2, 2, 3, 2; tones `Quiet, Quiet, Good, Good, Bad`; no empty line | drop the last `push` |
| `every_unicode_class_and_confidence_reads_in_every_language` | `crates/wipemark-i18n/src/tests.rs` | every class id and confidence id has its key, defined in en-US, de, ru | rename `unicode-class-zwj` to `unicode-class-joiner` in all three catalogues and in the CLI's `match` (compiles; this test is red) |
| `every_class_and_confidence_label_is_its_own_key` | `apps/wipemark-cli/src/report.rs` | `class_label(c).id() == format!("unicode-class-{}", c.as_str())` for every class; the same for confidences | swap two arms of `class_label` |
| `every_third_shelf_item_is_rendered_from_the_catalogue` | same | for every id of `not_established::ALL`, `shelf_line` returns `t(report-not-established-<id>)`, not the canonical English | delete one arm (falls back to English) |
| `the_human_report_carries_no_character_layer_a_would_strip` | same | `inspect_lines`/`clean_lines` over a text with **1234** ZWSPs, one NBSP and `"p\u{0430}y"` (aggressive), source `note.md`, with `say` = each shipped language's `Localizer::for_languages(…, PlainText).format_args` → no character of the set at `main.rs:655-657` plus U+00A0, U+202F, U+3000 in any line (the 1234 guards against a future digit-grouping formatter) | build the test's `Localizer` — or production's — with `Rendering::Ui` |
| `every_encoding_round_trips` | `apps/wipemark-cli/src/input.rs` | for every encoding except `Other` and texts `""`, `"abc"`, `"\u{FEFF}x"`, `"при\u{1F600}вет"`: `decode(encode(s)) == Ok(s)`; and for valid byte sequences `encode(decode(b)) == b` | encode UTF-16 always little-endian |
| `an_invalid_sequence_is_found_where_it_is` | same | UTF-8 `b"ab\xffcd"` → `Err(2)`; UTF-8 cut `b"a\xe2\x80"` → `Err(1)`; UTF-16LE odd length 5 → `Err(4)`; UTF-16LE lone high surrogate as 3rd unit → `Err(4)`; UTF-32LE `0x110000` as 2nd unit → `Err(4)` | lossy decoding (`from_utf8_lossy`) |
| `stdin_is_addressable_as_dash` (extended) | `apps/wipemark-cli/src/main.rs` | also `clean - --json` and `clean note.md -o -` parse | — |
| existing CLI tests (`arg_surface_is_well_formed`, `every_argument_and_subcommand_has_help`, `no_help_string_is_a_catalogue_key`, `the_help_headings_are_the_catalogue_ones`, `help_carries_no_character_layer_a_would_strip`, `exit_codes_are_pinned`, `summary_echoes_every_flag`, …) | same | unchanged, green | — |
| **`a_file_with_a_zero_width_space_exits_one`** (A §8) | `apps/wipemark-cli/tests/cli.rs` (subprocess) | `note.md` = `Hello` U+200B `world\n` → `inspect` exits **1**; `clean` exits **1**, `note.cleaned.md` = `Helloworld\n`; `inspect note.cleaned.md` exits **0** | decide `clean`'s exit by inspecting the *output* |
| **`an_unreadable_encoding_exits_three_not_zero`** (A §8) | same | KOI8-R `b"\xf0\xd2\xc9\xd7\xc5\xd4 hello"` (from `text.rs:292`) → **3**; a PNG named `x.txt` → **3**; 5000 × `a` then `0xFF` → **3**, stderr names byte 5000; UTF-16LE with BOM and odd length → **3**; in every case stdout is empty | decode `Encoding::Other` as Latin-1 (`b as char`) |
| **`clean_writes_beside_the_file_and_never_over_it`** (A §8) | same | `clean note.md` → `note.cleaned.md` exists, `note.md`'s bytes and permissions unchanged; a second run replaces `note.cleaned.md` (no `note.cleaned.cleaned.md`); `clean note.cleaned.md` → `note.cleaned.cleaned.md`; `-o note.md`, `-o ./note.md`, `-o <symlink to note.md>` → exit **2**, `note.md` unchanged; `-o <a folder>` → **2** | remove the `same_file` check |
| `a_missing_path_or_a_folder_is_a_usage_error` | same | `inspect nope.md` → **2** (`cli-no-such-file` on stderr), `inspect <dir>` → **2**, `clean <dir>` → **2**; stdout empty | map `NotFound` to 3 |
| `a_file_keeps_its_encoding_and_its_bom` | same | UTF-8+BOM, UTF-16LE+BOM, UTF-16BE+BOM, UTF-32LE+BOM, UTF-16LE without BOM (ASCII) each holding `a` U+200B `b`: the result's bytes are the input's encoding with the same mark (or none), U+200B gone | write every result as UTF-8 |
| `standard_output_is_utf8_whatever_the_input_was` | same | UTF-16LE+BOM on stdin, `clean -` → stdout is `EF BB BF 61 62` | write stdout in the input's encoding |
| `the_json_is_the_report_and_nothing_else` | same | `inspect --json` stdout parses as **one** JSON object with `unicode_version` and is ASCII (D29); `clean --json note.md` → keys exactly `report`, `written` (`written` ends in `note.cleaned.md`); `clean - --json` and `clean note.md -o - --json` → keys exactly `report`, `text`; nothing else on stdout | print the human report as well |
| `every_layer_a_answer_carries_the_third_shelf` (CLI) | same | inspect and clean, `--json` and not, over a marked and an unmarked file: every JSON's `not_established` (clean: `report.not_established`) equals the ids; every human report ends with the title line and the three item lines of the process's language (`WIPEMARK_LANG=en-US` in the subprocess) | delete the shelf loop in `report.rs` |
| `a_name_that_lies_is_read_by_its_contents_and_said_so` | same | `chart.png` holding `a` U+200B `b` → exit **1**, stderr carries `cli-name-disagrees` with `PNG` and `Text` | refuse a `Disagreed` file as not text |
| `positions_are_byte_offsets_into_a_utf8_file` | same | UTF-8 `é` U+200B (`C3 A9 E2 80 8B`) → `findings[0].positions == [2]`; with a BOM in front → `[5]` | strip the BOM before Layer A |
| `inspect_never_writes_anything` | same | the folder's listing and every file's bytes are identical before and after `inspect` and `inspect --json` | — |
| `an_empty_input_is_read_and_has_nothing_in_it` | same | a 0-byte file and empty stdin → `inspect` **0**, `cli-report-none`; `clean` writes an empty `x.cleaned.txt`, **0** | treat empty as `NotText` |
| `clean_exits_as_inspect_does_on_every_fixture` | same | for every file in `fixtures/text/` except `README.md` (walked at run time from `env!("CARGO_MANIFEST_DIR")/../../fixtures/text`; assert at least 22 were read — E1-3 creates 22, and E1-2/E1-4 may add theirs): `clean`'s exit == `inspect`'s exit | `clean` exits 0 when `findings` is empty |
| `the_commands_that_are_not_here_yet_still_refuse_at_two` | same | `rewrite x.md`, `models list`, `audit .` → **2**, stderr non-empty, stdout empty | route `rewrite` to `run::clean` |
| `nothing_reaches_the_log_but_the_shape` | same | run `clean` on `Secret-name.md` holding `Hello` U+200B `world` with `WIPEMARK_DATA_DIR` scratch; every file under `<scratch>/logs` contains `"<elided"` and neither `Hello` nor `Secret-name` | log `source_label` instead of `Elided::from(source_label)` |

Where the tests live: `wipemark-cli` is a binary crate with no library
target, so nothing under `apps/wipemark-cli/tests/` can call its items —
the integration tests there drive the built binary as a subprocess, and
everything that tests a function (`decode`, `class_label`, `inspect_lines`,
the `run::` flows with in-memory streams) sits in a `#[cfg(test)] mod tests`
in its own module. The same split holds in the app: the MCP and banner tests
are module tests. (A crate's `tests/` directory reaches only its public API;
`pub(crate)` items are tested in the module.)

Subprocess tests run `env!("CARGO_BIN_EXE_wipemark-cli")` with
`WIPEMARK_DATA_DIR=<scratch>` (logs and `configured_language` stay inside
it), `WIPEMARK_LANG=en-US`, and `WIPEMARK_LOG`/`RUST_LOG` removed (no stderr
mirror). Scratch directories follow `Scratch` in
`crates/wipemark-intake/src/lib.rs:377-401` (pid + thread id, removed on
drop). No test touches the network; no test opens a window.

## §6 Acceptance criteria

- [ ] All six §0.5 commands green with `--locked`; `Cargo.lock`'s diff is
      exactly the two added names in `wipemark-cli`'s list (§4.7).
- [ ] `with_infix`, `RESULT_INFIX`, `ORIGINAL_INFIX` are defined once, in
      `wipemark_intake::name`; `retention.rs` re-exports them; the moved and
      kept tests are green.
- [ ] MCP `inspect` and `clean` answer with `content[0].text` and
      `structuredContent` as in §4.2.2; `clean`'s is `{text, report}`.
- [ ] Every refusal is an `isError` result naming the argument; a
      non-object `arguments` is `-32602`; no client-supplied string except
      the `id` comes back unspelled.
- [ ] `nothing_the_server_says_carries_an_invisible_character` covers
      inspect, clean, and the three echo paths, and permits exactly the kept
      characters and a leading U+FEFF, counted.
- [ ] The tool listing is ASCII, says "up to about 1 MB", says "byte
      offsets", and has `additionalProperties: false`.
- [ ] The MCP banner ends with `settings-mcp-tools-layer-a` in every state,
      gated; `settings-mcp-tools-pending` exists in no catalogue and no code.
- [ ] `wipemark-cli inspect` and `clean` read a path or `-`, decode
      UTF-8/16/32 with and without BOM as §4.4.3, refuse with 2 or 3 as
      §4.4.7, write beside / `-o` / stdout as §4.4.5–4.4.6, keep the input's
      encoding and BOM, never write over the input.
- [ ] `clean` exits 1 for a suspicious input even though the result is
      clean; the module header and `README.md` say so.
- [ ] `--json` output is one JSON value per D12 (plus `-o -`).
- [ ] Every JSON and every human report carries the third shelf, gated in
      both applications.
- [ ] `rewrite`, `models`, `audit` refuse at 2 with `cli-not-implemented`;
      their log line still carries `epic`.
- [ ] No log line carries document text or a path in clear (gated).
- [ ] 44 keys added and 1 deleted in each of en-US, de, ru; the i18n suite
      green, including the new class/confidence gate.
- [ ] `docs/architecture/i18n.md` carries the D16 exception;
      `docs/architecture/layer-a.md` has "Surfaces"; CLAUDE.md changed in the
      four places of §4.6 and nowhere else; `retention.md` and `README.md`
      updated as §4.6.
- [ ] The release sizes of `wipemark-cli` and `wipemark` before and after
      this document, and their deltas, are in the report (§4.8).
- [ ] Every mutation in §5 performed and recorded; status row *done*;
      report written.

## §7 Out of scope

- **E5**: `rewrite`, `audit` (folders, `--sarif`, exit 3 for unreadable
  files in a tree), `models list|pull|verify|rm`, an in-place flag
  (`--in-place`, a "no original" flag — never a preference,
  `docs/architecture/retention.md` "What is left open"), CLI flags for
  `normalize_spaces` and `keep_soft_hyphen`.
- **E7** (A §7.4): the Compare window's result becoming
  `clean(original, &Options::default())`, the panel's findings count, the
  queue footer and the "Clean" action, the Inspector and "jump to position",
  and the window sentences that still say cleaning is not in this version
  (E1-7 rewords them; E7 makes the windows clean).
- **Q-A1**: preference rows for the four `Options` knobs and per-class
  overrides — no row may be added without a widget
  (`every_persisted_preference_has_a_row`).
- **Q-D2** folders and archives, **Q-D6** UTF-16 without BOM beyond the
  ASCII pattern intake already infers.
- An MCP tool that takes a path (`protocol.rs:62-69` says why not), an
  `outputSchema`, UTF-16 code-unit offsets for JavaScript clients, a second
  text limit inside the tool (D13).
- Launching the application: the live check of this work is E1-7's.

## §8 Basis and references

- **E1 spec** `wipemark-core-layer-a-2026-09-21` (A): §1 (S1.8 added to
  E1 — "a library nobody calls has no live gate"); §2 (UTF-16 decoded by the
  caller with `char::decode_utf16`; `Encoding::Other` refused: CLI 3, MCP
  `isError`); §4.1 (U+FEFF at byte 0 is a BOM, not a finding, kept in the
  output); §5.2 (inspect and clean are one decision); §5.3 (second-pass
  counts without positions); §5.5 (catalogue keys `unicode-class-<id>` ×11 and
  `confidence-<id>` ×4 in three catalogues; the third shelf is the surface's
  duty; names are the exception, Q-A6); §7.1 (the JSON — "no word from the
  catalogue: a machine reads this"; the clean-result rule for the invisible
  character test); §7.2 (MCP: `content[0].text` as a JSON string plus
  `structuredContent`; `clean` → `{text, report}`; `isError` for missing or
  non-string `text`; the D14 test "stays — it is about the missing
  argument"; the banner); §7.3 (CLI: `inspect <path|-> [--json]`, `clean
  <path|-> [-o out] [--nfkc] [--aggressive] [--json]`; intake; exit 3 "not read
  is not clean"; exit by the input for `clean`; beside per Retention; no
  `--in-place`; PlainText; `rewrite`/`audit`/`models` stay at 2;
  `Elided` in the log line); §7.4 (window seams, E7); §7.5 (what must not be
  done with Layer A); §8 rows *третья полка*, *MCP*, *CLI*; §9 Q-A4, Q-A6.
- **Overview** `heretic-unmark-overview-decomposition-2026-09-07`: §0.1
  rule 3 (three shelves), §7 (the CLI surface — `clean <path|->`; exit codes
  "as the reference").
- **Plan** `docs/plan/README.md` §3.3 (E1-6 provides), §4 **D9** (core writes
  the JSON with the third shelf; MCP parses it back), **D10** (`with_infix`
  move), **D11** (encodings, BOM, stdout UTF-8, missing 2 / unreadable 3),
  **D12** (`--json` shapes), **D13** (the transport's 413 is the limit),
  **D14** (the test keeps its name), **D15** (the banner key), **D16**
  (names exception), **D26/D27** (NFKC in rounds; the `(Nfkc, n)` counter),
  **D28** (`CleanReport` carries `suspicious` and `stats` over the source —
  the CLI's `clean` exit code reads it), **D29** (the report JSON is ASCII).
- **CLAUDE.md** rules quoted by name: "The third shelf is never empty"; "No
  epic number leaves this repository"; "Every string a person reads comes
  from the catalogue; nothing a machine reads does"; "Only applications
  localize"; "`Rendering::PlainText` for anything that is not a window"; "The
  bytes decide what a thing is; the name may only refine it"; "Diagnostics go
  to a file, and the document never does"; "The MCP server answers; its tools
  refuse"; "Nothing the MCP server says comes from the catalogue"; "A result
  goes beside the file, the file is never touched …"; "Exit codes are the
  CLI's interface, and there are four"; "Tests must be able to fail".
- **Model Context Protocol**, revision 2025-06-18 (`PROTOCOL_VERSION`,
  `protocol.rs:46`), *Tools*: structured content "SHOULD also return the
  serialized JSON in a TextContent block"; output schema optional; error
  handling — <https://modelcontextprotocol.io/specification/2025-06-18/server/tools>.
  Revision 2025-11-25, *Tools › Error Handling* — input validation errors as
  tool execution errors "that language models can use to self-correct":
  <https://modelcontextprotocol.io/specification/2025-11-25/server/tools>.
- **JSON-RPC 2.0** (`-32602` invalid params, `-32603` internal error, the
  `id` echoed exactly): <https://www.jsonrpc.org/specification>.
- **Project Fluent** (selectors, plural categories, numeric variant keys):
  <https://projectfluent.org/fluent/guide/selectors.html>,
  <https://projectfluent.org/>.
- **Unicode**: UAX #44 (the `Name` property and code point labels —
  the basis of D16): <https://www.unicode.org/reports/tr44/>; The Unicode
  Standard ch. 23.8 (U+FEFF as byte order mark / ZWNBSP).
