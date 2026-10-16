# Images — the fifth round: test gaps and docs (2026-10-05)

*The report of Watchword FILE `wipemark-task-images-followups-5-2026-10-05`
(V1–V5), uploaded as `wipemark-images-followups-5-report-2026-10-05`.
Branch `images/series-v3`, fast-forwarded from `ebf421a` to `c313eee`
(`origin/feat/e0-e6-shell`, which carries `6ff18bf`); the work is
`2cc421d`, this report after it. Nothing was merged into
`feat/e0-e6-shell` or `main`. No behaviour changed, and no decision was
taken: D254 onwards stay free.*

This round's mutations are `5V1`–`5V5` in the script (V4 is docs and has
none).

## What was done

| | requirement | done | commit | the test | the mutation that goes red |
|---|---|---|---|---|---|
| **V1** | the texture sentence's figure is guarded | yes | `2cc421d` | `a_texture_left_on_a_jpeg_is_said_and_exits_three` reads both figures out of the sentence, as 4L1 does: after "lie " over 8 (torch 4:4:4 95: 9.0), after "against " under 4 (3.3) | 5V1/M1 (`fixed(restored.texture_around, 1)` as the first figure) |
| **V2** | a lossy WebP is held as lossy | yes | `2cc421d` | `a_lossy_webp_is_held_as_lossy` (CLI) over the new `scroll-1040-q90.webp` — `04_tearing_scroll` cut to 1040, `lossless=False`, q90, 55 KB: exit 3, "pixels restored", "stored with loss", the texture sentence; in the JSON `lossy`, `texture_left`, not `exact`, `marks_left`, encoding `webp-lossless` `from_lossy` | 5V2/M1 (`decode.rs`: `fidelity: if false {`, a lossy WebP held as lossless) |
| **V3** | `TEXTURE_LEVELS` from below on real data; `TEXTURE_RATIO` pinned | yes | `2cc421d` | `a_texture_an_eye_barely_finds_is_not_said` over the new `fine-1040-q98-444.jpg` (`10_this_is_fine`, 4:4:4 98, the roughest of the 22 at 5.22 against 1.99): restored, lossy, not said, nothing left, and the ratio (2.62) does not decide it. `a_texture_the_picture_has_around_the_mark_is_not_said` moves to 13 against 7 (1.86, not said) and 15 against 7 (2.14, said). `a_grain_the_picture_has_is_not_said_under_a_real_mark`: `anchor-green-1025.png` at JPEG 4:4:4 95 by the `image` crate, 11.40 against 10.19 (1.12) — over `TEXTURE_LEVELS` twice over, not said | 5V3/M1 (`TEXTURE_LEVELS` 5.0), 5V3/M2 (`TEXTURE_RATIO` 1.8), 5V3/M3 (2.2), 5V3/M4 (1.1: anchor's grain said) |
| **V4** | stale docs | yes | `2cc421d` | — (docs: the CLI's exit table names a texture left; `cli.md`'s `restored` keys are `Restored`'s nineteen, and `kernel` is among `found`'s; `visible-marks.md`, D250's bullet: the two pixel sets as they are, the 5 % margin, the ratio's one job) | — |
| **V5** | `models list` says "4.7 GB" in ru/de | yes | `2cc421d` | `models_sizes_are_spelled_in_the_languages_decimals` (CLI, `models list` in en-US, de, ru: "7.4 GB", "7,4 GB", "7,4 ГБ", and never the other mark); the unit test keeps the rounding, mark-blind | 5V5/M1 (`format!("{:.1}", …)` back) |

## V3 — the measurements behind the fixtures

Every one of the 22 2048 originals cut to 1040 (from 1008, so the codec's
blocks fall as in the 2048 file, D252), saved by Pillow 12.3.0 (libjpeg
6.2, libwebp 1.6.0) and cleaned by this branch's CLI:

| saved as | texture | around | ratio | said |
|---|---|---|---|---|
| 4:4:4 q98 | 4.88–5.22 | 1.97–2.39 | 2.09–2.62 | 0 of 21; `11_crying` 5.60, said with its outline |
| lossy WebP q90 | 9.61–10.26 | 1.82–1.99 | 4.96–5.64 | 12 restored, all said with the fringe (chroma 6.4–7.3); 9 refused out of range |
| lossy WebP q95 | 9.01–10.16 | 1.59–1.85 | 5.03–6.06 | 16 restored, all said with the fringe; 5 refused out of range |

The q98 figures are the host verifier's to the hundredth. Both lossy
WebP outcomes — restored and said, or refused — are the ones the
verifier found correct; the fixture is a restored one, and the smallest.
`anchor.png` cut the same way has no finding (exit 0) and is not in the
rows; `anchor-green-1025.png` is the alternative, whose corner is the
grainy green.

**`TEXTURE_RATIO` and real data.** No sticker at 97 or 98 is decided by
the ratio — 2.39–2.69 and 2.09–2.62 are both over 2.0 — so it is pinned
twice: synthetically on both sides within 7 %, and by the one real
picture whose texture the ratio alone keeps unsaid, `anchor` at 4:4:4 95.

**The margin.** `TEXTURE_LEVELS` 5.5 is 5 % over the roughest q98
(5.22), not the ±10 % the fourth round's report gave; it is 11 % under
the smoothest q97 (6.12). `visible-marks.md` says so; the fourth round's
report is left as it was written.

## V4 — the texture's two pixel sets

Written into D250's bullet as the host verifier measured them, the
behaviour unchanged: the pixels a restoration *changed* are measured as
`α` in [0.002, 0.95), 3 868 on V1's large row against the 3 439–3 441 a
restoration reports; *around* includes the square's own pixels under
`α` 0.002, which read 2.8–3.4 on the 4:4:4 95 stickers against 1.0–1.3 on
a ring 8–36 pixels out. That makes the ratio cautious — it says less, not
more.

## Gates

In this container, on `2cc421d`:

| gate | result |
|---|---|
| `rustfmt --check` (nightly) over `crates` and `apps` | clean |
| `cargo clippy --workspace --exclude wipemark-app --all-targets -- -D warnings` | clean |
| `scripts/check-dep-direction.sh` | ok |
| `cargo test --workspace --exclude wipemark-app --locked` | **927 / 0 / 5** |
| `wipemark-app` (clippy, tests) | **not run here**: `yeslogic-fontconfig-sys` does not build in this container (fontconfig, xcb missing, as in round 4). GitHub runs it |
| mutations, this round (`5V1`–`5V5`, 7) | **7 red** |
| mutations, U2 re-run (its tests were edited) | **9 of 9 red** |
| `images-followups-mutate.py --check` | 83 entries, each text there once |

GitHub, `gate`, `macos` and `native` all green on `2cc421d`:
https://github.com/GigLaboCom/wipemark-app/actions/runs/37348299995 —
`cargo test --workspace --locked` **1379 / 0 / 6** (1375 at `ebf421a`,
and the four new tests: `a_lossy_webp_is_held_as_lossy`,
`a_texture_an_eye_barely_finds_is_not_said`,
`a_grain_the_picture_has_is_not_said_under_a_real_mark`,
`models_sizes_are_spelled_in_the_languages_decimals`); `local-llama`
36 / 0 / 1 and 452 / 0 / 1; the app's `mcp::image` 5 / 5.
