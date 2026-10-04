# Images — real Gemini marks in the tests (2026-10-04)

After `images-followups-2026-10-04.md`. The owner asked for three things
on 2026-10-04: the stickers in `heretic-videos/images/stickers/` that
carry Gemini's mark as a stored (uncompressed) archive in Watchword; the
composites in the tests replaced by real files, because they were not
good enough; and the work on `images/series-v3`.

## The archive

Watchword FILE **`wipemark-gemini-stickers-2026-10-04`** (ttl 0): a ZIP,
every entry `stored`, 42 files, 231 329 437 bytes, sha256
`5a54435bbd600b43113ade474ad08f6f8b83dc10cdb637eca84847515b991f04`
(read back and compared). Paths kept, under `stickers/`.

Which files: every one in which `wipemark-cli inspect` saw the
`gemini-sparkle-v1` mark, checked by eye where it was not proved —

* the 21 originals (`01_…` to `20_…`, `10_this_is_fine_alternative`):
  2048 × 2048, the mark proved at the large row (1888, 96 px), and every
  one but `11_crying` also carries C2PA and AI metadata;
* `good-alt/Gemini_Generated_Image_…(1).png`: proved;
* `alt-anch/anchor-alternative.png`: the sparkle is there (`k*` 1.00) but
  refused out of range — the corner was edited after the mark;
* `transparent/*` (19 of 22): the logo is still in the colour channels
  but under **alpha 0** — invisible to anyone who honours the alpha, seen
  and refused as `Transparent`.

Not in it: `anchor.png`, `icon_100.png`, the grids (no mark);
`transparent/05`, `/14` and `/anchor.png` (no mark under them);
`telegram_hq/*` and the thumbnails (the background, and the mark with
it, cut away — the one "finding" there, on `telegram_hq/19` and two
thumbnails, was confetti: the defect below).

## The fixtures

`fixtures/image/gemini/` — four of them, each a 2048 picture cut to its
bottom-right **1025 × 1025** (the vendor's mark then sits exactly at the
large row, margin 64), so a row is tested on the pixels the vendor
stamped at a twentieth of the bytes; origin, bytes and sha256 in
`fixtures/image/README.md`. Q-V8 ("no vendor file is committed") is
superseded for these four by the owner's request.

| file | bytes | role |
|---|---:|---|
| `crying-1025.png` | 558 672 | the vendor's mark at its row |
| `torch-1025.png` | 934 572 | the same, another picture |
| `anchor-edited-1025.png` | 1 053 895 | an edited corner: seen, not proved |
| `cut-out-confetti-256.webp` | 18 422 | a cut-out sticker, confetti in its transparent corner: no mark |

## What the real files showed

| case | result |
|---|---|
| the mark at its row, PNG | proved, restored, outline **0.043**; **not exact** — 61 of 9 013 pixels clamp: GWT's maps are 8-bit captures of the vendor's α, which the composites (drawn *with* those maps) could never show |
| as JPEG 90 / 95 | restored; outline 0.120 / 0.084 (crying), 0.106 / 0.065 (torch) |
| shrunk with its picture to 373 px (0.364) by Lanczos or bilinear, JPEG 90 / 95 — the host verifier's case, on real marks | proved in **6 of 8**, the filter recognised (Lanczos for Lanczos, bilinear for bilinear), outline **0.066–0.109**, under the 0.20 bound |
| the edited corner | `k*` 1.00, refused out of range, left, `clean` exits 3 |
| **the cut-out sticker** | **was reported as a mark** — fixed |

**The defect.** `Refusal::Transparent` was returned before the proposal
was measured, so a proposal under a transparent corner never reached the
no-blend test (D235) and was always a finding: confetti on a cut-out
sticker, a mark that is not there, `inspect` exiting 1. Transparency is
now asked only once the proposal is a blend (`verify.rs`): the confetti is
no blend and no finding; the `transparent/` copies, a real blend under
alpha 0, are still seen and refused as `Transparent`.

## Tests

* `crates/wipemark-picture/tests/real.rs` (five):
  `a_real_mark_is_proved_at_its_row_and_restored`,
  `a_real_mark_saved_as_jpeg_is_restored_within_the_outline_bound`,
  `a_real_mark_shrunk_with_its_picture_is_restored_within_the_outline_bound`,
  `an_edited_real_mark_is_seen_and_left`,
  `a_cut_out_sticker_is_not_a_finding`.
* The CLI's `tests/visible.rs` runs on the real files: the proved mark
  (inspect, clean, audit/SARIF — the rectangle is now 96 wide), the
  edited corner as the mark that cannot be proved, and
  `a_cut_out_sticker_is_clean`.
* The MCP server: `a_real_gemini_mark_comes_off_over_mcp` (`clean_image`
  on the real PNG, 745 KB of base64: the image comes back with the mark
  removed, and inspecting it finds nothing).
* `blocks_tile_the_file` walks `fixtures/image/gemini/` too.
* What stays synthetic, on purpose: `wipemark-pixels`' algorithm suites
  (exactness, the false-positive gate, the second pass, calibration) —
  there the original under the mark is known; a real file has no ground
  truth to measure a restoration against.

Mutations (`images-followups-mutate.py`, `REAL/*`), all **red**:

| # | protection | tests |
|---|---|---|
| REAL/M1 | a transparent corner is asked about only once the proposal is a blend | `a_cut_out_sticker_is_not_a_finding`, `a_cut_out_sticker_is_clean` |
| REAL/M2 | the vendor's own mark is proved at its row (the large row's margin moved by one) | `a_real_mark_is_proved_at_its_row_and_restored` |
| REAL/M3 | a shrunk real mark is matched to its filter | `a_real_mark_shrunk_with_its_picture_is_restored_within_the_outline_bound` |

## The branch and CI

`images/series-v3`: the host merged `feat/e0-e6-shell` into it
(`5f7899d`), which brought the GitHub workflow and E2-5. **The gate ran
on that merge — everything R0–R11 changed — and is green**:
<https://github.com/GigLaboCom/wipemark-app/actions/runs/37217039807>
(`gate`: fmt, clippy, the whole workspace's tests, deps, features;
`native`; `macos`: all success; `llama-source` success too). This work is
`85c5ece`, on top of it; its run is **green** too: <https://github.com/GigLaboCom/wipemark-app/actions/runs/37219175707> (`gate`, `native`, `macos`: success).

Locally on `85c5ece` (the merged tree): clippy over the workspace clean;
the image tests **252 passed, 0 failed** (libraries 191, the CLI's image
and visible 25, audit 7, unit 16, MCP 13).
