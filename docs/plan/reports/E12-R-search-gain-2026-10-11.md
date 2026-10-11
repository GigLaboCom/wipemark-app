# D530 — the search's gain on a resized picture (2026-10-11)

**For a person whose picture was resized.** If you scale an AI picture up
or down before handing it to Wipemark, its visible mark is scaled with it,
and Wipemark finds it by searching the corner rather than by looking at the
exact place the vendor stamps it. Until now that search would also accept a
mark that was *fainter* than the vendor's own — the faint mark of another
opacity, which is not the mark Wipemark knows — and "remove" it at the
wrong strength, leaving an outline 15–25 levels off, more than half the time
without saying a mark was left. Now, on a picture where the mark is large
(64 pixels or more — every resized 2048-pixel Gemini output), Wipemark
removes a resized mark only when its strength matches the vendor's, and
otherwise says the mark is there and leaves it. Every real resized sticker
in the test corpus (63 of 63) is still found and removed exactly as before;
nothing changes for a picture that was not resized. For small marks (under
64 pixels — a 1024-pixel output resized) nothing changes either, because
there no setting tells a faint mark from a real one without refusing real
ones too. The setting is a value in the mark catalogue, so the owner can
change it, or switch it off, without a code change.

Branch `recon/search-gain-resampled`, from `feat/e0-e6-shell` at `918e314`.
The coordinator's task of 2026-10-11, from the owner's question: "do it the
better way, and if possible under a parameter, so we don't have to rewrite
the code every time." Decision **D530**.

## 1. What the search sees (the measurements)

`crates/wipemark-picture/examples/search_gain.rs` runs the product's search
on each picture and verifies its proposal at three places: **base** (NCC's
whole-pixel place, the search map drawn by the area integral), **refined**
(the place the product proves at — the refinement by the residual at
`k = 1`, D236) and **free** (a refinement by the residual at the gain the
picture shows, `k` 0.80–1.20). The hook is
`wipemark_pixels::trace_search`, `#[doc(hidden)]`.
`scripts/analytics/search_gain.py` reads its lines. The full outputs:
[`E12-R-search-gain-2026-10-11/gain-resampled.txt`](E12-R-search-gain-2026-10-11/gain-resampled.txt)
(the bench, 2 880 files of the three resampled slices) and
[`gain-real.txt`](E12-R-search-gain-2026-10-11/gain-real.txt) (the 413-file
corpus).

`k*` of the search's proofs at the refined place — the product's — before
D530 (the bench's R5 composites, 60 backgrounds, `encoded`; slices
`resize-0.9`, `resize-1.1`, `jpeg420-q90+resize-0.9`):

| group | proofs | `k*` at the refined place | `k*` at NCC's place |
|---|---:|---|---|
| (a) `k = 0.93`, V1 96 (GWT map) at 2048 | 65 | 0.96 ×36, 0.98 ×29 | 0.90–0.94 (0.94 ×60) |
| (a) `k = 0.93`, V2 96 at 2048 | 50 | 0.96 ×50 | 0.90–0.96 |
| (a) `k = 0.93`, V2 36 at 1024 | 50 | 0.96 ×26, 0.98 ×14, 1.00 ×9, 1.02 ×1 | 0.86–0.94 |
| (a) `k = 0.93`, V1 48 at 1024 | 0 | — | — |
| (b) `k = 1`, V1 96 (GWT map) | 100 | 1.02 ×5, 1.04 ×95 | 0.94–1.02 |
| (b) `k = 1`, V1 96 measured | 99 | 1.02 ×1, 1.04 ×37, 1.06 ×61 | 0.94–1.02 |
| (b) `k = 1`, V2 96 | 69 | 1.00 ×5, 1.02 ×42, 1.04 ×22 | 0.90–1.04 |
| (b) `k = 1`, V1 48 | 85 | 0.96 ×1, 0.98 ×29, 1.00 ×55 | 0.84–1.00 |
| (b) `k = 1`, V2 36 | 87 | 0.98 ×11, 1.00 ×55, 1.02 ×21 | 0.90–1.00 |
| (b) `k = 1`, V2 96 resampled to 48 | 84 | 0.96 ×4, 0.98 ×16, 1.00 ×64 | 0.92–0.96 |
| (c) the corpus, `recon-resized:x0.9-png` | 21 | 1.06 ×21 | 1.02 ×20, 1.04 ×1 |
| (c) the corpus, `recon-resized:x0.9-q90-420` | 21 | 1.04 ×20, 1.06 ×1 | 1.02 ×21 |
| (c) the corpus, `recon-resized:x1.1-png` | 21 | 1.06 ×21 | 1.02 ×20, 1.04 ×1 |

No other file of the corpus has a search proof (every other finding is a
row's). What it says:

* **Why the search accepts a `k = 0.93` resampled mark.** Two biases both
  push the search's `k*` up. The refinement goes where the residual *at
  `k = 1`* is least, and for a weaker mark that is a template a shade off
  (an eighth of a pixel, or a smaller size) where the gain reads higher: NCC's
  place reads 0.90–0.96 (mostly 0.94, which the second proof refuses — a
  hair past 0.06 in `f32`), the refined place 0.96–1.00, a drift of 0.02–0.10.
  And a picture resized bicubically sharpens a large mark against the
  area-drawn template: a `k = 1` mark at 86–106 pixels reads 1.02–1.06,
  the corpus's 1.04–1.06 (D524's edge). On V2's 36-pixel mark shrunk to 32
  the 0.93 marks that read 0.98–1.02 are the ones the search drew with the
  bilinear kernel (D238's filters, tried under 40 % of the search map): a
  smoother filter is a weaker-looking mark.
* **What separates (a) from (b)/(c).** By the template's width. From 64
  pixels up (every V1/V2 96 mark resized by 0.9 or 1.1): (a) 0.96–1.00,
  (b) 1.00–1.06 with only V2 96's five at 1.00, (c) 1.04–1.06 — a lower end
  of **1.02** separates them. Under 64 pixels: (a) 0.96–1.02, (b) 0.96–1.02
  — no window separates, and a true mark shrunk with its picture and saved as
  JPEG reads 0.96 there
  (`a_shrunk_and_compressed_mark_is_restored_within_the_outline_bound`, V2 96
  at 0.36 of its size).
* **The free refinement does not separate**: it reads the weak marks at
  0.92–1.00 and proves 134 of the 165 (against 165) — the drift is not only
  the refinement's objective. Not taken.

## 2. The rule, and why

**A resampled mark has gain bands of its own.** A profile's `search` carries
`gain`, a list of bands by the template's width. A search's proof of a
template that is **resampled** — its width more than `NATIVE_REACH`
(1.125 px, how far the refinement moves a size) from every map width of the
profile — must have `k*` inside the window of the first band whose `under`
is above the template's width (a band with no `under` takes every width
left). A template at a map's own size (a picture cropped, a mark a pixel off
its row) and every row keep the second proof as it was, `|k* − 1| ≤
verify.gain`. It extends D470 — which holds a mark at a row's place — to a
mark at no row's place, by the same reading of D154: a mark at another
opacity is another profile, never a per-picture `k`.

Both Gemini profiles ship (D530's default):

```json
"gain": [ { "under": 64, "window": [0.96, 1.06] }, { "window": [1.02, 1.06] } ]
```

Under 64 pixels the window is a row's as the sweep reads it (`|k* − 1| ≤
0.06` accepts 0.96–1.06 on the 0.02 grid: 0.94 is a hair past 0.06 in
`f32`, as it always was) — so nothing changes there; from 64 pixels up the
search proves a resampled mark only at 1.02–1.06.

**The others, measured and not taken** (the search's proofs kept, from the
measure above; the bench's own counts in §4):

| setting | weak (a) proved, of 165 | `k = 1` (b) proved, of 524 | corpus (c), of 63 | why not |
|---|---:|---:|---:|---|
| off (before D530) | 165 | 524 | 63 | the problem |
| one window `[0.98, 1.06]` | 53 | 519 | 63 | leaves a third of the weak marks |
| one window `[1.00, 1.06]` | 10 | 463 | 63 | loses 61 true marks (V1 48 and V2's 48 shapes read 0.96–0.98), and the shrunk-mark gate |
| one window `[1.02, 1.06]` | 1 | 284 | 63 | loses 240 true marks, every small resized one |
| under 64 `[0.98, 1.06]`, else `[1.02, 1.06]` | 24 | 514 | 63 | loses a true shrunk mark (the gate reads 0.96) and 10 bench marks for 26 weak ones |
| under 64 `[0.96, 1.06]`, else `[1.00, 1.06]` | 50 | 524 | 63 | a large weak mark at 1.00 passes (on the bench none; in `tests/search_gain.rs` 3 of 8 large weak marks over noise read 1.00) |
| **under 64 `[0.96, 1.06]`, else `[1.02, 1.06]` (D530)** | **50** | **519** | **63** | — |

Also not taken: a tighter symmetric tolerance for the search (the real marks
sit at the *top* of the window, 1.04–1.06, so a tighter one loses them
first); comparing against the rows at the scaled place (it needs the
original size — a resize and a crop together have none — and NCC's place
already reads the weak marks at 0.90–0.96 against the true ones' 0.84–1.04,
which overlap); the free refinement (§1). The default keeps every real
finding, as the task asks; the weak small marks it leaves are said in §7.

## 3. The parameter

* **Name**: `search.gain` on each profile of `manifests/marks.v1.json`.
* **Values**: a list of bands `{ "under": <width in pixels>, "window": [lo,
  hi] }`; the last band may omit `under`. Absent, `null` or `[]`: no band —
  the product before D530, **byte for byte** (the value that switches the rule
  off). Checked with the catalogue, a bad value refused by name: a window is
  two gains from above 0 to 1.6, the lower first; an `under` is a width above
  0 and above the band before; a band with no `under` is the last; a band is
  an object of those two fields and nothing else
  (`a_search_gain_band_is_read_or_refused_by_name`).
* **Default**: `[ { "under": 64, "window": [0.96, 1.06] }, { "window": [1.02,
  1.06] } ]` on both Gemini profiles.
* **Changing it without code.** The product reads only the compiled-in
  catalogue (D162), so a new value ships with a rebuild — a JSON edit, no
  code. To **try** one first, copy the catalogue and edit the copy:

  ```sh
  mkdir -p /tmp/try && cp manifests/marks.v1.json /tmp/try/marks.json
  # edit both "gain" fields of /tmp/try/marks.json, e.g. to switch the rule off:
  #   ..., "alpha": "gemini-v1-96-measured" },        (the field removed)
  # or another large band:
  #   "gain": [ { "under": 64, "window": [0.96, 1.06] }, { "window": [1.00, 1.06] } ]

  # the bench, over a run generated without a catalogue file (the Gemini rows):
  target/release/examples/recon_bench run --in bench/out/<run> --config R8d \
      --slices resize-0.9,resize-1.1,jpeg420-q90+resize-0.9 \
      --catalogue /tmp/try/marks.json --out bench/out/<run>/results-try.jsonl
  # real files (a regress.py list, or any picture list):
  python3 scripts/regress.py list --out /tmp/try/corpus.tsv
  target/release/examples/search_gain --list /tmp/try/corpus.tsv \
      --catalogue /tmp/try/marks.json --out /tmp/try/gain.jsonl
  python3 scripts/analytics/search_gain.py run /tmp/try/gain.jsonl \
      --setting 'try=[{"under": 64, "window": [0.96, 1.06]}, {"window": [1.00, 1.06]}]'
  ```

  `recon_bench run --catalogue FILE` over a run generated without one is new
  (D530): the file takes the shipped catalogue's place and its GWT twin is
  made from it by the same one-row edit, read as JSON; the run's `.run.json`
  records the file, its sha256 and `"in_place_of": "shipped"`
  (`a_catalogue_file_takes_the_shipped_ones_place`). `search_gain`'s `product`
  field is the product's own verdict under the file. There is no
  `--catalogue` on `wipemark-cli` (D162: the product reads the compiled-in
  catalogue and nothing else); the task's "the CLI's existing `--catalogue`
  route" is the developer tools' road, `examples/support/catalogue.rs`.
  No environment variable, no cargo feature.

## 4. The bench before and after

`docs/plan/reports/E12-R-search-gain-2026-10-11-bench.sh`: the close-out's 60
backgrounds (30 flat, 30 text, pinned), 960 cases; R0 and R8d (the product's
road) over the three resampled slices, once per catalogue; R8d over the ten
files where the mark sits at its row, `off` and `bands`. Tables by
`E12-R-search-gain-2026-10-11-tally.py`, all of them in
[`E12-R-search-gain-2026-10-11/tally.md`](E12-R-search-gain-2026-10-11/tally.md);
the runs' records in [`bench/`](E12-R-search-gain-2026-10-11/bench/). The
subset was run rather than the whole bench (R6, R8p and R8w are R8d's on
these slices for detection; 2 h rather than 2 h 20 for five configs).

**The R-k marks (`k = 0.93`, D154)** — the close-out's G4 columns:

| catalogue | config | slices | n | refused by gain | proved by the search | not found | PSNR med of those proved | said left |
|---|---|---|---:|---:|---:|---:|---:|---:|
| off | R0 | resampled | 720 | 246 | **148** | 237 | 38.84 | 66 |
| off | R8d | resampled | 720 | 246 | **165** | 237 | 39.71 | 69 |
| `[0.98, 1.06]` | R8d | resampled | 720 | 398 | 53 | 237 | 35.45 | 34 |
| `[1.00, 1.06]` | R8d | resampled | 720 | 469 | 10 | 237 | 41.13 | 6 |
| **bands (D530)** | R0 | resampled | 720 | 418 | **44** | 237 | 41.53 | 14 |
| **bands (D530)** | R8d | resampled | 720 | 418 | **50** | 237 | 41.82 | 14 |
| off | R8d | at its row | 2400 | 2016 | 0 | 382 | — | 0 |
| **bands (D530)** | R8d | at its row | 2400 | 2016 | 0 | 382 | — | 0 |

`off` reproduces G4 (148 R0, 165 for the others). Of the weak marks the
search proved before, fewer than half were *said* to leave a mark (66 of
148): the rest were restored at `k = 1` and reported clean. Under D530 the
large ones are gone (V1 96: 65 → 0, V2 96: 50 → 0); the 50 left are all V2's
36-pixel mark (§1: no window separates there).

**The marks at the profile's own opacity (`k = 1`, `encoded`)**, R8d,
resampled slices, proved at the mark's place (of 180 each):

| row | off | `[0.98, 1.06]` | `[1.00, 1.06]` | bands (D530) |
|---|---:|---:|---:|---:|
| v1-48 | 85 | 84 | 55 | 85 |
| v1-96 | 100 | 100 | 100 | 100 |
| v1-96-measured | 99 | 99 | 99 | 99 |
| v2-36 | 87 | 87 | 76 | 87 |
| v2-96 | 69 | 69 | 69 | **64** |
| v2-96-r48 | 84 | 80 | 64 | 84 |

D530 loses 5 of 524: V2's 96 mark at 1.00 after a resize to 0.9 and a 4:2:0
JPEG at 90 — refused by its gain, a finding, nothing written. R0 the same.
At the row nothing moved (every config, every row, both `k`). The
linear-light composites (another blend than the profile's) moved under the
one-window settings (10 and 31 proved → refused) and not under D530.

The `0.98` and `1.00` runs were made with the step's intermediate build (one
window `[lo, hi]` for every search proof, no `NATIVE_REACH`): on the bench no
search proof is at a map's own width (0 of 2 880, measured), so they are
what the band format `[{"window": [lo, 1.06]}]` gives;
`E12-R-search-gain-2026-10-11-bench.sh` writes them in the band format.

## 5. The regression over the real corpus

`scripts/regress.py run --route all` against `golden/baseline/34b4c2c/`,
the release CLI at `04471c5` (the gates' run): **pass**
([`regress-summary.md`](E12-R-search-gain-2026-10-11/regress-summary.md)).
Corpus gates: G2 ok (exit 3: 195 before, 195 after), G1 ok (0 files), L4/D3
ok. **Exit or `written` changed: none. Attention: none. Fail: none.** The 63
resized findings are all searched, verified and restored as before (§1:
`k*` 1.04–1.06, inside the large band). The baseline was not regenerated. (A
first run at `598ec44`, made while the bench ran beside it, read
"attention" on time alone — ×2–3 on every file, the machine shared — with
nothing else moved.)

## 6. Tests and red checks

New tests:

* `crates/wipemark-picture/tests/search_gain.rs`:
  `a_weaker_mark_resized_with_its_picture_is_refused_by_the_search` — V1's
  measured mark drawn at 0.93 at its row on a 1025 picture over four
  backgrounds, resized by 0.9 and 1.1 (Catmull-Rom, Pillow's bicubic): proved
  by the search with the bands removed (`k*` 0.96–1.00), refused by its gain,
  nothing written and a mark said left, under the shipped catalogue — the
  parameter switches it; `a_mark_at_its_own_opacity_resized_with_its_picture_is_still_proved`
  (the same at `k = 1`, and V2's 36 mark resized to 32);
  `the_vendors_mark_resized_with_its_picture_is_still_proved_by_the_search`
  (the committed real crops `torch-1025.png` and `victory-1025.png`, resized
  by 0.9 and 1.1: searched, `k*` 1.06, restored — there is no resized real
  crop in `fixtures/`, so the test makes one from a real one);
  `the_shipped_searches_carry_d530s_bands`.
* `crates/wipemark-pixels/src/catalogue.rs`:
  `a_search_gain_band_is_read_or_refused_by_name` (absent, `null`, `[]` → no
  band; two bands read in order; six bad values refused by name; three shapes
  refused by the schema), `a_band_holds_a_resampled_template_and_never_one_at_its_maps_size`.
* `recon_bench`: `a_catalogue_file_takes_the_shipped_ones_place`.
* The existing gates hold unchanged, among them
  `a_shrunk_and_compressed_mark_is_restored_within_the_outline_bound`,
  `a_real_mark_off_its_row_is_searched_with_the_measured_map` (a crop: a
  template at its map's own size, held to no band),
  `a_real_mark_shrunk_with_its_picture_is_restored_within_the_outline_bound`
  and `a_k_of_0_93_is_refused_by_a_k_of_1_profile` (D470).

Red checks — `E12-R-search-gain-2026-10-11-red.py`, each protection deleted
once, its test run, put back
([`red.tsv`](E12-R-search-gain-2026-10-11/red.tsv)), **9 of 9 red**, the
named test failing in each:

| check | the protection deleted | red test |
|---|---|---|
| `search-held-to-no-band` | `gain_holds` asks no band for the search | `a_weaker_mark_resized_with_its_picture_is_refused_by_the_search` |
| `shipped-without-bands` | both `gain` fields removed from `marks.v1.json` | the same |
| `native-held-too` | `search_band` holds a template at a map's own width too | `a_band_holds_a_resampled_template_and_never_one_at_its_maps_size` |
| `large-band-short` | V1's large band ends at 1.04 | `the_vendors_mark_resized_with_its_picture_is_still_proved_by_the_search` |
| `small-band-raised` | V2's small band starts at 0.98 | `a_shrunk_and_compressed_mark_is_restored_within_the_outline_bound` |
| `window-reversed-read` | a window's `lo <= hi` not checked | `a_search_gain_band_is_read_or_refused_by_name` |
| `under-unordered-read` | an `under` not checked against the band before | the same |
| `open-band-not-last-read` | a band without `under` before another not refused | the same |
| `bench-file-not-in-place` | `run --catalogue` over the Gemini rows keeps the shipped catalogue | `a_catalogue_file_takes_the_shipped_ones_place` |

The first run of `search-held-to-no-band` did not compile (`None` with no
type); the edit was given its type and run again, red — the table's line is
the second run.

## 7. Left open

* **A weak small mark resized is still proved** (V2's 36-pixel mark at 0.93,
  50 of the bench's 720): under 64 pixels a true mark reads as low, and the
  bands cannot tell them apart. If the owner prefers refusing more true small
  marks, `[0.98, 1.06]` under 64 leaves 24 weak ones and refuses the shrunk
  gate's mark and 10 bench marks — a catalogue edit.
* **A weak mark at its own size off its row** (a weaker picture cropped) is
  neither D470's nor D530's: the template is at the map's own size. Not
  measured; the bench has no cropped slice.
* The real corpus holds only V1's large mark resized (2048 → 1843 and 2253).
  A resized 1024-class output (a small mark) and other resizers (Lanczos, a
  browser's) are not in it; the bench's Catmull-Rom/bicubic is the one
  measured. The large band's top, 1.06, is where the corpus reads: a
  resizer that sharpens more would be refused by gain as before D530 (D524).
* D530 changes no sentence a person reads: a refusal by gain is the finding
  it always was, in every language (no catalogue string, no epic number).

## 8. Gates

`E12-R-search-gain-2026-10-11-gates.sh`, once, at `04471c5`, toolchain
1.95.0, everything `--locked`:

| gate | result |
|---|---|
| nightly rustfmt `--check` over every `.rs` under crates/ and apps/ | exit 0 |
| clippy `--workspace --all-targets -D warnings` | exit 0 |
| test `--workspace` | **1991 passed, 0 failed, 11 ignored** |
| `check-dep-direction.sh`, `check-gpui-pin.sh`, `check-zune-pin.sh` | exit 0, exit 0, exit 0 |
| check `--no-default-features`; check `--features local-llama` | exit 0; exit 0 |
| engine tests with `local-llama` | 43 passed, 0 failed, 1 ignored |
| app tests with `local-llama` | 763 passed, 0 failed, 2 ignored |
| pipeline examples with `local-llama`: clippy; tests | exit 0; 28 passed, 0 failed |
| pixels + picture without `blend-preview`: clippy, tests, examples | exit 0; **207 passed, 0 failed, 6 ignored**; exit 0 |
| pixels + picture with `blend-preview`: clippy, tests, examples | exit 0; **215 passed, 0 failed, 6 ignored**; exit 0 |
| release CLI and examples; `calibrate` | exit 0; exit 0 |
| every Python file compiles; every selftest under scripts/ (regress, bench ×4, corpus ×2, grok ×4, model-eval ×7, analytics ×3 — `search_gain.py` new) and the report's tally | exit 0 each |
| `scripts/analytics/selfcheck.sh` | exit 0 |
| regression against `golden/baseline/34b4c2c/`, `--route all` | **pass** (§5) |

`check-gpui-pin.sh` ran once more by hand: in the gates' run it called
`cargo` by name and met this container's wrapper rather than the toolchain
(`ssh-exec: SSH_HOST is not set`, exit 2); with the toolchain first on
`PATH` it passes (`gpui pin ok … x11 fixes ok`). The script now puts
`$CARGO`'s folder first on `PATH`.

## 9. CI

CI

## 10. Wanted CLAUDE.md edits

* In "A visible mark comes off only once it is proved, and what is left is
  said", after D470's sentence: "A mark resized with its picture is held by
  the profile's search gain bands instead (D530, `search.gain` in
  `manifests/marks.v1.json`): from 64 pixels up a resampled mark is proved
  only at `k*` 1.02–1.06, under 64 as a row is; absent is the product before
  D530."
* In the `wipemark-pixels` row of the crate table: "… the search's gain
  bands by template width (`search.gain`, `Profile::search_band`, D530) …".
* In "Epic order": "The next free decision block is **D531**", and a line
  for D530 (the search's gain bands, merged when it is).
* In the Watchword table: `wipemark-recon-search-gain-report-2026-10-11`
  (FILE): this report.
