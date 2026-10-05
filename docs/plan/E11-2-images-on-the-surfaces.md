# E11-2 — Images on the command line and over MCP

|                  |                                                                                                                                       |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| Series           | `docs/plan/` — epic E11, images (phase 2)                                                                                              |
| Spec scopes      | OV §9 (containers, AI flags, knobs) as E11-1 built it; the CLI's exit codes (`docs/architecture/cli.md`); the MCP server's rules (`CLAUDE.md`) |
| Task             | Watchword FILE `wipemark-task-e11-2-image-surfaces-2026-10-04`                                                                         |
| Depends on       | E11-1 (`crates/wipemark-image`, branch `e11/image-metadata`); E5-1 (`inspect`/`clean`/`audit`, `--in-place`); E6 S6.3 (the MCP server) |
| Unblocks         | E7's windows (the queue and the panel showing a picture's report); an owner answer on an MCP `path` argument                           |
| Files touched    | `crates/wipemark-image/src/{lib.rs,json.rs}`, `apps/wipemark-cli/{Cargo.toml,src/{main,input,run,report,audit,image}.rs,tests/image.rs}`, `apps/wipemark-app/{Cargo.toml,src/mcp/{mod,protocol,server,image}.rs}`, the three catalogues, `Cargo.lock`, `docs/architecture/{cli,images}.md`, this document, its report |
| Size             | ~1.5 days for one agent; pure code — no GPU, no model, no window                                                                      |

## §0 Ground rules

### 0.1 Start here

Read `CLAUDE.md` in full — where it and this document disagree, `CLAUDE.md`
wins and the report says so. Branch `e11/image-surfaces`; E11-1 was not yet
in `feat/e0-e6-shell` when this started, so the branch is
`origin/e11/image-metadata` with `origin/feat/e0-e6-shell` merged into it
(E4-5, E4-6a — the CLI and MCP code this step edits). One commit,
`E11-2: Images on the command line and over MCP`; push the branch only, no
PR. Leave ` m vendor/gpui-component` unstaged.

### 0.2 Where code goes

```
core ← image ← cli, app        (applications may use any library)
       intake ← cli, app       (the bytes decide what a thing is)
```

- The library gains **formats only** — ids, names, a JSON writer, `spell`
  (I12). It still returns values and never a sentence.
- **Only applications localize.** Every word a person reads in the CLI is a
  catalogue key in en/ru/de; nothing the MCP server says comes from the
  catalogue.
- `rewrite.rs` (CLI and MCP) is another branch's (E4-7) and is not touched.

### 0.3 Rules of this repository that bind this document

- **Exit codes are the interface**: one meaning across a text and a picture.
- **The third shelf is never empty**, and a picture's report says that
  only its metadata was examined — a mark in the pixels is not looked for.
- **No epic number leaves this repository**: TIFF/HEIC/AVIF are refused
  "not in this version yet".
- **The bytes decide**: a picture is a picture by its contents, never by
  its name.
- **A result goes beside the file**; in place only by the per-run flag,
  through `wipemark_intake::inplace`, the original set aside first.
- **The MCP server answers**: a call it cannot run is an `isError` result
  naming why; the transport's 1 MiB `413`; a client string said back
  spelled; no invisible character in anything it says.
- **Tests must be able to fail**: every protection has a mutation (§6).

### 0.4 Gates

```sh
rustup run nightly rustfmt --edition 2021 --check $(find crates apps -name '*.rs')
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
scripts/check-dep-direction.sh
cargo check --workspace --no-default-features --locked
cargo check --workspace --features local-llama --locked
cargo test -p wipemark-app --features local-llama --locked
```

## §1 Goal

A person can run `wipemark-cli inspect|clean|audit` on a PNG, JPEG or WebP
and an agent can call `inspect_image`/`clean_image` over MCP: every
metadata block listed, AI provenance found by its signal and removed, the
pixels untouched, and every refusal said by name.

## §2 Read first

`CLAUDE.md`; `docs/architecture/images.md`; `docs/plan/E11-1-image-metadata.md`
and its report (the remark that `Scope::AllMetadata` takes EXIF whole,
orientation included); `docs/architecture/cli.md`; `docs/plan/E5-1-cli-without-the-pipeline.md`;
`docs/architecture/layer-a.md`; `apps/wipemark-cli/src/`; `apps/wipemark-app/src/mcp/`;
`crates/wipemark-intake` (`identify`, `inplace`).

## §3 What is true today (at `98f16b6`)

- `wipemark_image::inspect` and `strip` exist and no surface calls them —
  `crates/wipemark-image/src/lib.rs:411`, `:436`; the crate's status says
  so at `:28`.
- The CLI refuses a picture as **not text**, exit 3:
  `apps/wipemark-cli/src/input.rs:173` (`encoding_of` → `Unread::NotText`),
  `apps/wipemark-cli/src/run.rs:515`. `audit` skips one as `not-text`:
  `apps/wipemark-cli/src/audit.rs:299`.
- `clean`'s signature is positional flags (`run.rs:159`); the destination is
  decided before a byte is read (`run.rs:339`).
- The MCP server has three tools (`apps/wipemark-app/src/mcp/protocol.rs:130`,
  `:151`), each requiring `text` (`:354`); the body limit is
  `LARGEST_BODY` (`server.rs:87`). The banner says "Three tools run"
  (`crates/wipemark-i18n/i18n/en-US/wipemark.ftl:984`).
- `ImageReport`/`StripReport` have no JSON form; a PNG text keyword is
  Latin-1 and may hold a soft hyphen (`png.rs`, `latin1`).
- `base64` 0.22.1 is in `Cargo.lock` (through gpui's `usvg`).

## §4 Decisions

Numbered I1…; proposed as D130… for `docs/plan/README.md` §4.

| # | D | decision | why |
|---|---|---|---|
| I1 | D130 | A file is a picture when intake places it as PNG, JPEG, WebP, TIFF, HEIC or AVIF **by its bytes** (`Evidence::Content`, `Agreed`, or `Disagreed` with the bytes winning). A name alone (`Evidence::Name`) is not; GIF, BMP and SVG are not handed over. `input::read_any`; `input::read` (which `rewrite` uses) is unchanged. | the bytes decide; a parser handed a name-only "PNG" would refuse it anyway |
| I2 | D131 | `inspect` on a picture exits 1 when any block is AI provenance (`has_ai_metadata`, which includes every C2PA signal), 0 otherwise. Camera EXIF, XMP without an AI source type, IPTC, a comment, colour are **not** findings. | the product's subject is provenance; a hook that failed on every camera photo would be switched off |
| I3 | D132 | `clean` exits by the **input**, as a text's does (D28): 1 when it carried AI provenance (removed), 0 when it carried none. **3 when the output would still carry any** (`still_has_ai_metadata` or `still_has_c2pa`, from the second inspection) — and then **nothing is written**. | one meaning of 0/1 across text and picture; a `.cleaned` file that is not is worse than none; 3 is "not complete", as `rewrite`'s kept paragraph is |
| I4 | D133 | The flag is `--all-metadata`, on `clean` only, mapped to `Scope::AllMetadata`. Its help says it takes camera data, EXIF orientation included, and that colour profiles are kept by every scope; the report says so again when an EXIF block went. | the library's own name for the scope; `--strip-all` would promise colour too |
| I5 | D134 | A flag for the other kind is a **usage error, 2**, decided once the bytes are read and before anything is written: `--aggressive`/`--nfkc` on a picture, `--all-metadata` on a text. | silently ignoring a flag runs a different job from the one asked for |
| I6 | D135 | A picture's bytes go to standard output only when it is **not a terminal**; a terminal is refused (2). `--json` with the picture on standard output is refused (2): stdout carries one product. The terminal check is a field (`run::Clean::stdout_is_terminal`) so a test can drive it. | binary on a terminal is garbage at best; base64 in the CLI's JSON would be a second format for the same thing |
| I7 | D136 | Refusals: `NotYet` (TIFF/HEIC/AVIF), `UnknownContainer` and `Unsupported` (MPF) exit **2** by name ("not in this version yet"); **`Malformed` exits 3** with the defect from the catalogue. | 2 is "a refusal, said what did not run" (the stub rule); a file that could not be read has not been shown clean — exactly an invalid encoding's 3, and `audit` counts it the same way. **Deviation**: the task asked 2 for malformed |
| I8 | D137 | `ImageReport::to_json` and `StripReport::to_json` live in `wipemark-image` (`json.rs`, std only, as core's writer is D9): one line, ASCII, keys fixed, the third shelf written from core's constant; every string read out of the file (`chunk`, `key`, `field`) goes through `wipemark_image::spell` — printable ASCII as itself, anything else `U+XXXX`. | one writer for the CLI, `audit` and MCP; a keyword's soft hyphen must not reach a terminal or a model |
| I9 | D138 | The human report: the summary (never "clean"), blocks under "Would be removed/kept" (`inspect`) or "Removed/Kept" (`clean`), each with its signals; the colour note; the `--all-metadata` note; where the result went; **the pixels line**, then the third shelf — and no Unicode line. Words are `cli-image-*`, `image-kind-*`, `image-signal-*`, `image-defect-*`; formats (chunk, key, generator id, IPTC code, offsets) as themselves. | the text report's shape, with what a picture actually has |
| I10 | D139 | `audit` reads pictures: scanned with the default scope; a finding is `has_ai_metadata`; TIFF/HEIC/AVIF are **skipped** (`image-not-yet`); a malformed picture is **unreadable** (`malformed-image`) and counts toward 3-beats-1. `--json` keeps its shape and `version: 1` — the entry's `report` is the picture's own JSON (`container` where a text's has `unicode_version`). SARIF: rules `image-<signal id>`, one result per signal of each AI block, `level: error`, region `{byteOffset, byteLength}` and no line. | a binary artifact's region is bytes (SARIF 2.1.0 `region`); one rule per signal is what a dashboard can group |
| I11 | D140 | MCP `inspect_image { data }` and `clean_image { data, scope? }`; `data` is base64 (standard alphabet, padded, strict — `base64` 0.22 from the lock, no hand-written codec); `scope` is `ai-provenance` (default) or `all-metadata`. Refusals are `isError` results: bad base64, not a picture (naming what intake found), `NotYet`, MPF, malformed, and a result that would still carry provenance (no image comes back). No `path` argument. | the task, and `CLAUDE.md`'s MCP rules |
| I12 | D141 | The library's public surface grows by formats only: `ImageContainer::{ALL, id, name}`, `MetadataKind::{ALL, id}`, `Signal::id`, `Defect::{IDS, id}`, `Scope::{ALL, id}`, `spell`, the two `to_json`. No behaviour changes. | the surfaces key their words and their JSON off stable ids |
| I13 | D142 | `rewrite` on a picture is unchanged: refused as not text (3). | out of scope; `rewrite.rs` is another branch's |

## §5 Deliverables

1. **CLI** — `input::read_any`/`Content`/`Picture`/`picture_of`;
   `image.rs` (`inspect`, `clean`, `clean_exit`, `inspect_exit`, refusals,
   the lines); `run.rs` dispatching to it, `run::Clean` (the flags as one
   value), `say_named`; `report::{shelf, written_lines}`; `audit.rs` with
   `Status::Image`/`ImageUnreadable`, `Skip::ImageNotYet`, the image line,
   the JSON entry and SARIF rules/results; `--all-metadata` in `main.rs`.
2. **MCP** — `mcp/image.rs` (decode/encode, `Refusal`, `inspect`, `clean`);
   `protocol.rs`: `Tool::{InspectImage, CleanImage}`, `required()`,
   `image_schema`, `read_image`, `image_answer`, `image_refused`.
3. **Library** — `json.rs`, the ids and names (I12).
4. **Catalogue** — `cli-arg-all-metadata`, `cli-audit-image`, `cli-image-*`
   (19), `image-kind-*` (8), `image-signal-*` (5), `image-defect-*` (11), in
   en/ru/de; `cli-command-{inspect,clean,audit}`, `cli-arg-in-place` and
   `settings-mcp-tools` reworded in all three.
5. **Docs** — `docs/architecture/cli.md` (image rows, the flag, `audit`),
   `docs/architecture/images.md` ("Surfaces"), the MCP paragraph there; the
   report.

## §6 Tests (RED first; a mutation per protection)

| protection | test | mutation that must turn it red |
|---|---|---|
| `still_has_*` never exits 0 or 1 | `image::tests::an_output_that_still_carries_provenance_never_exits_zero` | `clean_exit`: map `still_has_*` to `Exit::Clean` (or drop either half of the `||`) |
| inspect exits by AI provenance only | `tests/image.rs::inspect_exits_one_on_each_ai_signal_and_zero_on_a_camera` | `inspect_exit` on `!findings.is_empty()` |
| the bytes decide | `input::tests::a_picture_is_decided_by_its_bytes` | drop the `Evidence::Name` exclusion in `picture_of` |
| `rewrite` keeps refusing a picture | `input::tests::only_read_any_hands_over_a_picture` | `read` passes `pictures: true` |
| the input is never touched | `clean_writes_beside_the_file_and_leaves_it_untouched` | `Destination::Beside` → the input path |
| pixels unchanged through the surface | `the_pixels_of_a_cleaned_image_are_the_pixels_it_had` | in `image::clean`, flip a byte of `bytes` before writing |
| `--in-place` sets aside, never over one | `in_place_sets_the_original_aside_and_never_overwrites_one` | `image::clean` writes with `write_atomically` instead of `inplace::replace` |
| `--all-metadata` reaches the library | `all_metadata_removes_exif_and_keeps_colour` | `run::clean` always passes `Scope::AiProvenance` |
| a flag for the other kind is refused | `a_flag_for_the_other_kind_is_a_usage_error`, `run::tests::all_metadata_on_a_text_is_a_usage_error` | delete the `text_flag` refusal / the `Content::Text(_) if all_metadata` arm |
| never image bytes to a terminal, never beside `--json` | `image::tests::image_bytes_are_never_written_to_a_terminal` | delete either refusal at the top of `image::clean` |
| TIFF by name, 2 | `a_tiff_is_refused_by_name` | `ImageError::NotYet` → `Exit::Partial` |
| malformed is 3 | `a_truncated_jpeg_is_not_read_and_not_clean` | `Malformed` → `Exit::Usage` |
| `audit` 3 beats 1 for a picture | `an_unreadable_picture_makes_the_audit_inconclusive` | `Status::unreadable` without `ImageUnreadable` |
| `audit` lists pictures in all three outputs | `audit_lists_pictures_in_every_output` | `Status::with_findings` without `Image`; SARIF `region` with `startLine` |
| the JSON writer: exact, shelf, spelled | `json::tests::*` | `tail` empty; `spell` → identity |
| the pixels line in every language | `image::tests::every_image_report_says_the_pixels_were_not_examined` | drop `CliImagePixels` from `footer` |
| MCP: third shelf on every answer | `protocol::tests::every_image_answer_carries_the_third_shelf` | `image::inspect` returns `{}` |
| MCP: every refusal `isError` | `every_image_refusal_is_an_error_result_naming_why` | `image_answer` answers a refusal with `answered("{}")` |
| MCP: base64 byte-identical | `the_base64_round_trip_is_byte_identical`, `image::tests::base64_reads_the_rfc_vectors_and_nothing_looser` | `encode` drops the last byte; `decode` with a forgiving engine |
| MCP: no invisible character | `nothing_the_server_says_carries_an_invisible_character` (extended), `no_image_answer_carries_a_character_it_read_out_of_the_file` | `spell` → identity |
| MCP: 413 whole | `server::tests::an_image_over_the_limit_is_refused_whole` | raise `LARGEST_BODY` |
| catalogue complete, no promise | `wipemark-i18n`'s suite (`every_message_renders_in_every_language`, `variables_match_the_fallback`, `no_language_promises_more_than_the_product_does`) and `image::tests::every_label_is_its_own_key` | delete a key from `ru` |

## §7 Acceptance

All gates of §0.4 green on a machine that builds GPUI; every mutation of
§6 red; the report lists any gate not run, and why.

## §8 Out of scope

TIFF, HEIC/AVIF; any window (E7); `rewrite` on a picture; the pixel
domain; an MCP `path` argument (an owner question); editing inside XMP or
EXIF.

## §9 Basis

The task; `CLAUDE.md`; E11-1's plan, report and `docs/architecture/images.md`;
`docs/architecture/cli.md`; MCP 2025-06-18 (Tools, structured content);
RFC 4648 §4 and §10; SARIF 2.1.0 (`region.byteOffset`, `region.byteLength`).
