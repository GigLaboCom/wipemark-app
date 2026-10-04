# E12-5 — Surfaces: the CLI and MCP carry visible marks

|                  |                                                                                                                         |
| ---------------- | ----------------------------------------------------------------------------------------------------------------------- |
| Series           | `docs/plan/E12-visible-marks.md` — epic E12; step 6 of the images series                                                  |
| Spec scopes      | `docs/sdd/visible-marks.md` §6, §8.1, §8.2; D156, D160 (amended), D161; the owner's answers Q-V1, Q-V5, Q-V9 (2026-10-04)  |
| Task             | Watchword FILE `wipemark-task-images-series-2026-10-04`, step 6                                                           |
| Depends on       | E12-3, E12-4 (`wipemark-picture`), E11-2 (the image surfaces)                                                             |
| Files touched    | `apps/wipemark-cli/{Cargo.toml,src/{image,audit,report}.rs,tests/{image,visible}.rs}`, `apps/wipemark-app/{Cargo.toml,src/mcp/{image,protocol}.rs}`, `crates/wipemark-picture/src/lib.rs`, `crates/wipemark-i18n/{Cargo.toml,src/tests.rs,i18n/*/wipemark.ftl}`, `scripts/check-dep-direction.sh`, `docs/plan/reports/E11-2-mutate.py`, `docs/architecture/{cli,images,visible-marks}.md`, this document, its report |
| Size             | ~2 days                                                                                                                  |

## §0 Ground rules

`CLAUDE.md` wins. One commit, `E12-5: Surfaces — the CLI and MCP carry
visible marks`. Only applications localize; nothing the MCP server says
comes from the catalogue; no epic number leaves the repository; the
product never says "undetectable". Tests compiled, not run.

**The owner's answers (2026-10-04), binding.** Q-V1: no flag — `inspect`
reports, `clean` removes every verified mark; there is no `--keep-visible`
(D160 amended). Q-V5: the 1 MiB body stays, a larger one is refused with
the limit named, no path argument. Q-V9: a vendor is an identifier in a
finding, never inside a sentence.

## §1 Goal

Every picture that reaches `inspect`, `clean`, `audit`, `inspect_image`
or `clean_image` goes through `wipemark_picture` — the metadata pass and
the visible pass, one writer — and every report says what it found in the
pixels, what it removed, what it left and why, and that invisible marks
remain.

## §4 Deliverables

1. **CLI `inspect`**: a "Visible marks" section; exit 1 on any finding,
   proved or not; 3 when the pixels should have been examined and were not
   (a catalogue that did not load, pixels that do not decode) — 3 beats 1.
   An animation is said and changes nothing.
2. **CLI `clean`**: verified marks removed with no flag; the report says
   how the picture was written back; the exit by the input (1 when it
   carried a mark or provenance); a mark **left** → the result written,
   exit 3; a result still carrying provenance metadata → not written, 3 (as
   E11-2); a restored picture that could not be written back or failed its
   own check → nothing written, 3.
3. **CLI `audit`**: a visible mark is a finding; an unexamined picture is a
   hole; SARIF `visible-<profile>` results, `properties.rect`, no region;
   `run.properties.not_established` leads with `invisible-pixel-marks` when
   a picture was scanned.
4. **MCP**: `inspect_image` and `clean_image` carry the pass; refusals for
   a picture that could not be written back or failed its proof; a mark left
   is in the report, not a refusal; the banner reworded in en/ru/de.
5. **JSON**: E11's keys where they were; `visible`, `encoding`,
   `marks_left`; the picture's shelf.
6. **Catalogue**: every new sentence in en/ru/de; the picture shelf's
   translation; a gate on the picture shelf; a gate that no catalogue
   string names a mark's vendor or product.

## §5 Tests

| test | protects | mutation |
|---|---|---|
| `a_visible_mark_left_behind_never_exits_0` (CLI unit) | a mark left, or pixels not examined, is 3 | map it to the metadata's exit |
| `inspect_reports_a_visible_mark_and_exits_one` | inspect exits 1 on a mark; the words; the JSON | inspect ignores marks |
| `clean_removes_a_proved_mark_with_no_flag` | Q-V1; the result inspects at 0 | — |
| `a_mark_that_cannot_be_proved_is_left_and_exits_three` | written, said, 3 | — |
| `audit_puts_a_visible_mark_in_sarif_properties` | SARIF | a byte region |
| `the_picture_shelf_is_never_empty_in_any_language` (i18n) | the shelf's translation | delete the `ru` key |
| `no_catalogue_string_names_a_mark_vendor` (i18n) | Q-V9 | put "Gemini" in a sentence |
| `every_image_answer_carries_the_third_shelf` (MCP, extended) | the picture's shelf and the pass on every answer | — |
| `nothing_the_server_says_carries_an_invisible_character` | unchanged, covers the new refusals | — |

## §7 Out of scope

The windows (E12-8, with E7); a `path` argument or a larger limit (Q-V5);
other vendors (E12-6).
