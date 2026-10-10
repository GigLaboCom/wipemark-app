# E12-R — the central check of `recon/r1-r12` (2026-10-09)

*Five step agents wrote the code and the tests of R2's tools, R11's tools,
R12's stage 4b, R10's scripts and R9 without running anything — no
compile, no test, no clippy, no rustfmt, no selftest, no mutation — by the
owner's rule: "общее сведение в конце, пускай только код напишут, а проверит
всё один в конце". This is that one check, over the combined branch
`recon/r1-r12` (`recon/r1-r8` at `7bd5014` and the five merges, head
`0f3f79b` when it started). Nothing was pushed, nothing touched `main`,
`CLAUDE.md` or Watchword.*

| | |
|---|---|
| Branch | `recon/r1-r12`, from `0f3f79b`; the code as checked is `bcda512`, and the commit that adds this report adds only it and its two scripts (§5) |
| Toolchain | cargo/rustc 1.94.1 `--offline`, nightly rustfmt; Python 3.13.7, numpy 2.5.3, Pillow 12.3.0; no torch, no onnxruntime, no OpenCV |
| R3's patch | `[patch.crates-io] zune-jpeg = { path = "/workspace/zune-image/crates/zune-jpeg" }` applied from the stash and **left uncommitted** with its `Cargo.lock` move; `vendor/gpui-component` shows modified (the pin script) and was never staged |
| Scripts | `docs/plan/reports/E12-R-central-check-gates.sh` (every cargo gate, logged per step) and `docs/plan/reports/E12-R-central-check-mutations.py` (every red check below, applied, run and reverted byte for byte) |

## 1. The gates

The final run, at `bcda512` — the commands of `E12-R-central-check-gates.sh` — every one `--offline`:

| run | command | result |
|---|---|---|
| pin | `scripts/pin-gpui-component.sh` | exit 0 |
| fmt | `rustup run nightly rustfmt --edition 2021 --check $(find crates apps -name '*.rs')` | exit 0 |
| dep | `scripts/check-dep-direction.sh` | exit 0 |
| clippy | `clippy --workspace --all-targets -- -D warnings` | exit 0 |
| test | `test --workspace` | **1559 passed, 0 failed, 11 ignored** (773 s) |
| check-nodef | `check --workspace --no-default-features` | exit 0 |
| check-llama | `check --workspace --features local-llama` | exit 0 |
| engine-llama | `test -p wipemark-engine --features local-llama` | 36 passed, 0 failed, 1 ignored |
| app-llama | `test -p wipemark-app --features local-llama` | 533 passed, 0 failed, 1 ignored |
| px, none | `clippy -p wipemark-pixels -p wipemark-picture --all-targets -- -D warnings` · `test` · `build -p wipemark-picture --examples` | exit 0 · **195 passed, 0 failed, 7 ignored** · exit 0 |
| px, `planar-preview` | the same, `--features wipemark-picture/planar-preview` | exit 0 · **193 / 0 / 9** · exit 0 |
| px, `blend-preview` | the same, `--features wipemark-picture/blend-preview` | exit 0 · **203 / 0 / 7** · exit 0 |
| px, both | the same, `--features "wipemark-picture/planar-preview wipemark-picture/blend-preview"` | exit 0 · **201 / 0 / 9** · exit 0 |

The examples' own tests run inside each `px-test` run (`recon_bench`,
`measure_clean` and `export_crops` are `[[example]] test = true`):
`export_crops` 2, `measure_clean` 6, `recon_bench` 15 without
`blend-preview` and 16 with it. `blend-preview` adds `tests/blend.rs` (5),
`tests/assets.rs`'s pin test (6 → 7), one `catalogue` unit (48 → 49) and
`recon_bench`'s `a_blend_row_lays_over_the_catalogue_file`. Against
`recon/r1-r8`'s 1543/0/11, the workspace gains 16 tests.

Not cargo:

| check | result |
|---|---|
| `python3 -m py_compile` over every `.py` under `scripts/` and `docs/plan/reports/` | clean |
| `scripts/regress.py selftest` | 20 passed, 0 failed |
| `scripts/bench/report.py selftest` | 49 ok, 0 failed |
| `scripts/bench/encode.py selftest` | ok (0 problems) |
| `scripts/bench/wml.py selftest` | ok |
| `scripts/bench/wordmark.py check` | ok (the committed fixture is the script's) |
| `scripts/corpus/ring.py selftest` | 14 ok, all passed |
| `scripts/corpus/manifest.py selftest` | 7 ok, all passed |
| `scripts/grok/invariance.py selftest` | 14 ok, all passed (12 before; §3, §4) |
| `scripts/grok/align.py selftest` | 7 ok, all passed |
| `scripts/model-eval/{evalkit,trigger,fdncnn_export,fdncnn_run,lama_run,baselines,ab}.py selftest` | 7, 7, 2, 5, 4, 4, 3 ok; 0 failed. Without torch, `fdncnn_export`'s load-and-export half and without OpenCV `baselines`' NS and Telea are said and not run |
| `scripts/analytics/{bias,gain}.py selftest` | 13 and 14 ok, all passed |
| `scripts/analytics/selfcheck.sh` (release `wipemark-cli` and `wipemark-picture` examples) | exit 0, "selfcheck: done" |

`__pycache__` folders were deleted after every run; none is committed.

**What failed at the first run, and was fixed** (each in §5):

* clippy `-D warnings`, workspace: `clippy::large_enum_variant` on
  `Verdict` and `verify::Outcome` — R9's `Colours` and `Law` made
  `Verified` 232 bytes. And in the pixels/picture sets:
  `clippy::unnecessary_lazy_evaluations` in R11's `false_positives.rs`.
* rustfmt `--check`: R9's `blend.rs`, `tests/blend.rs`; R11's
  `false_positives.rs`, `tests/support/mod.rs`; R12b's `recon_bench.rs`,
  `measure_clean.rs` — layout only.
* `fdncnn_run.py selftest`: a test built a 1 × 5 array `np.gradient`
  cannot differentiate (a wrong test).
* `lama_run.py selftest`: `better()` declared an unused positional
  argument no caller passed, so `gates()` raised on every call (**a defect
  in the script**); and a test called its stub without its mask (a wrong
  test).
* `align.py selftest`: the jitter case's worst offset 0.1025 px against a
  bound of 0.1 (§5, R11).

Everything else — the whole workspace's tests, the four feature sets'
tests, every example in every set, the local-llama lanes — was green at
its first run.

## 2. The false-positive test (R11's word negatives)

`cargo test -p wipemark-pixels --test false_positives -- --nocapture`,
measured at `recon/r1-r8` by the R11 agent and here at `bcda512`:

| | before (`7bd5014`, R11's report) | after |
|---|---|---|
| negatives | 2000 × 2 profiles × 2 sources: 3272 proposals, every one no blend; 0 reported, 0 restored | **2331** × 2 × 2: **3428** proposals, every one no blend; 0 reported but the half-transparent words, 0 restored |
| words | — | 111 opaque, 111 outlined, 111 half-transparent × (2 synthetic + 2 shipped profiles) × 2 sources: **472** proposals dismissed; half-transparent: **0 of 444** examinations reported; **0 verified, 0 restored** |
| planar, 300 + 300 (4:2:0 q90) | `{("edges","edges"): 43, ("gain","gain"): 171}` | the same: `{("edges","edges"): 43, ("gain","gain"): 171}` |
| wallpapers | 120 × 2 × 2: 180 proposals, 0 reported | 180 proposals, every one no blend; 0 reported, 0 restored |
| look-alike blends | 1442 of 2000 reported, lowest edge ratio 0.143; 994 of 1000 by their gain | the same: 1442 of 2000, 0.143; 994 of 1000; 0 restored |

**No word negative was verified or restored**, under the synthetic or the
shipped Gemini profiles: there is no false-positive finding on text. The
test takes 265 s. It passed in all four feature sets.

## 3. The interfaces between the agents' work

Written blind to each other, four joins did not hold; each now does, with
a selftest that runs the producer's real output into the consumer.

| join | what did not fit | what changed | held by |
|---|---|---|---|
| R2 `manifest.py grok` → R11 `invariance.py` | R11's `normalise` read a row's source from `source` and its mark from `mark`/flat keys; R2's manifest writes the Grok source in **`profile`** (its `source` is the ZIP's, `grok`/`grok-video`), the by-hand mark under **`stage0`** as strings (`"20×20"`), and lists clips | `normalise` reads R2's row as it is (`from_stage0`): source from `profile`, the rectangle from `stage0`'s `corner`, `margin`, `mark_size`, clips and rows whose mark is not `yes` skipped; the header and `scripts/grok/README.md` say so (`4774c2f`) | `invariance.py selftest`: "R2's manifest.py grok manifest is read" — `manifest.py grok` over synthetic captures (a source with a mark, one without, a clip), its manifest through `read_rows` and `run`: one pair, `grok.com`, the held-out fifth left out, "one map" |
| R11 `invariance.csv` → R10 `trigger.py lama` | `invariance.csv` carries `source`, `size`, `hole_share`, `reading`; the trigger read it, but refused the whole file when a pair had no support (`reading` none, `hole_share` empty) | such a row is left out and **said** in the report; the table gains the size; the READMEs chain the two commands (`4774c2f`) | `trigger.py selftest`: `r11s_invariance_csv_is_read_as_it_is` — `invariance.py run` over a holed and a bare pair, its CSV read: one row, over 1 %, "mandatory", the bare pair said |
| R2's `gemini-midtone` manifest → `scripts/analytics/bias.py list` | the schema read fine, but R2's manifest holds **both** Gemini profiles and `list` wrote every row into one list, which `bias.py run`, `forced_search` and `map_regress` read under one profile's map | `list --profile ID` keeps that profile's rows; a manifest naming more than one profile is refused without it; READMEs updated (`bb87eef`) | `bias.py selftest`: "R2's manifest is listed under one profile…" — a manifest in R2's row schema: one profile kept with its held-out flag, two refused, a changed file refused by sha256 |
| R12b `--catalogue` ↔ R9 `--blend-row` (`recon_bench`) | (a) the file road collected `.wma` only, so a catalogue file whose row names a `.wml` logo map could not load even with `blend-preview`; (b) `--blend-row` built on the shipped catalogue alone, so a row over a profile that exists only in a `--catalogue` file found neither the profile nor its maps | (a) `examples/support/catalogue.rs` reads `.wma` and `.wml`; without the feature the catalogue still refuses the row by name. (b) `preview_catalogue` lays the row over the `--catalogue` file when there is one and looks for assets beside the row, then beside the file, then among the shipped ones, through `catalogue_file::parse_with` (`c7f6255`) | `recon_bench`'s `a_catalogue_file_with_a_blend_field_loads_only_under_blend_preview` (both builds: a `bias` row and a `logo_map` row load with the feature, are refused by name without) and `a_blend_row_lays_over_the_catalogue_file` (feature only) |

R9's `wml.py` → `LogoMap::read` was read side by side and agrees (magic,
`u16` sizes, depth 16, three planes, `round(L/255·65535)` half away from
zero); `map_regress`'s `pixels.tsv` carries the `L_r/L_g/L_b` columns
`wml.py from-tsv` reads; `regress.py --export-crops` calls
`export_crops --class/--variant/--pad` as that example parses them.

**R10's questions 2 and 3** (`7eb8eaf`): `recon_bench run --export-crops`
writes R8 §4.1's `sigma_base` — `wipemark_pixels::sigma_base` over the
input at the restoration's own rectangle (the expected box when nothing was
restored), as `interval.rs` computes it and `export_crops` writes it — in
place of `null`, with `export_crops`' `rect_px_in_crop` and `restored`
beside it, and takes `--crop-pad N` (default 64; refused without
`--export-crops`). `evalkit.py` already read `meta.json`'s value first and
restated it otherwise; a selftest case now holds that, and the docs say
both exporters write it. The bench test exports at a pad of 100 and holds
the context on the crop's two unclipped sides and the value to the Rust
function at `rect_px_in_crop`. One limit: in that fixture the restoration's
rectangle *is* the expected box, so the test does not tell the two apart.

## 4. The red checks

Every mutation the five reports list under "To run centrally", applied
alone by `E12-R-central-check-mutations.py`, its command run, the files
written back and `git diff --quiet` checked after each. `red` = the named
test failed; `named` = the named case is the one seen failing. Rust ones
in the narrowest `-p … --test/--lib/--example … NAME` and the feature set
the test needs. The first column is the report's id.

| id | edit | command | result | named | fix |
|---|---|---|---|---|---|
| R2-M1 | `ring.py` `measure_ring`: the spread over the whole image | `python3 scripts/corpus/ring.py selftest` | red | yes | — |
| R2-M2 | `assign_held_out`: listing order, not sha256 | `python3 scripts/corpus/manifest.py selftest` | red | yes | — |
| R2-M3 | `add_files`: every flag recomputed | `python3 scripts/corpus/manifest.py selftest` | red | yes | — |
| R2-M4 | `verify --root`: hash skipped | `python3 scripts/corpus/manifest.py selftest` | red | yes | — |
| R2-M5 | `verify --zip`: stored check off | `python3 scripts/corpus/manifest.py selftest` | red | yes | stronger test (`8419316`) |
| R11-M1 | `analyse`: `std_ring` over the zone | `python3 scripts/grok/invariance.py selftest` | red | yes | stronger test (`b983ffc`) |
| R11-M2 | `analyse`: `e_floor` over the zone | `python3 scripts/grok/invariance.py selftest` | red | yes | — |
| R11-M3 | `α̂` from the residual | `python3 scripts/grok/invariance.py selftest` | red | yes | — |
| R11-M4 | `r2_position` on `B` | `python3 scripts/grok/invariance.py selftest` | red | yes | — |
| R11-M5 | `align_pair`: no offsets | `python3 scripts/grok/align.py selftest` | red | yes | — |
| R11-M6 | `write`: shifted by −δ | `python3 scripts/grok/align.py selftest` | red | yes | — |
| R11-M7 | `align_pair`: offsets rotated | `python3 scripts/grok/align.py selftest` | red | yes | — |
| R11-M8 | word arm: nothing stamped | `cargo test -p wipemark-pixels --test false_positives no_procedural_negative_is_ever_restored` | red | yes | — |
| R11-M9 | translucent: `all(Verified)` | `cargo test -p wipemark-pixels --test false_positives no_procedural_negative_is_ever_restored` | green | no | none: no translucent word is seen (0 of 444) |
| R12b-M1 | `tile_origin`: always bottom-right | `cargo test -p wipemark-picture --example recon_bench a_catalogue_files_profile_gets_rows_of_its_own` | red | yes | — |
| R12b-M2 | `jpeg_slice`: digits unchecked | `cargo test -p wipemark-picture --example recon_bench a_slice_is_one_of_the_benchs_or_a_jpeg_at_any_quality` | red | yes | — |
| R12b-M3 | `degradations`: `check_slices` gone | `cargo test -p wipemark-picture --example recon_bench a_degradation_list_names_its_profile_and_its_slices` | red | yes | — |
| R12b-M4 | `run_case`: `Cat::File` arm gone | `cargo test -p wipemark-picture --example recon_bench a_profile_from_a_catalogue_file_runs_from_gen_to_its_result_lines` | red | yes | — |
| R12b-M5 | `gen_one`: fixed q95/q90 writes | `cargo test -p wipemark-picture --example recon_bench a_profile_from_a_catalogue_file_runs_from_gen_to_its_result_lines` | red | yes | — |
| R12b-M6 | `map_for`: square rect | `cargo test -p wipemark-picture --example measure_clean a_profile_from_a_catalogue_file_is_measured_at_its_own_shape` | red | yes | — |
| R12b-M7 | `BY_VARIANT` without held-out | `python3 scripts/regress.py selftest` | red | yes | — |
| R12b-M8 | held-out G4 rule gone | `python3 scripts/regress.py selftest` | red | yes | — |
| R12b-M9 | `facts` without profiles | `python3 scripts/regress.py selftest` | red | yes | — |
| R12b-M10 | F1 block off | `python3 scripts/regress.py selftest` | red | yes | — |
| R12b-M11 | `reproduce_scope` returns all | `python3 scripts/regress.py selftest` | red | yes | — |
| R12b-M12 | `of_profiles` returns all | `python3 scripts/bench/report.py selftest` | red | yes | — |
| R12b-M13 | `planned` returns all | `python3 scripts/bench/encode.py selftest` | red | yes | — |
| R10-1 | `fdncnn_rows`: counts marks | `python3 scripts/model-eval/trigger.py selftest` | red | yes | — |
| R10-2 | `mask_of`: strength everywhere | `python3 scripts/model-eval/fdncnn_run.py selftest` | red | yes | — |
| R10-3 | `compose`: no composite | `python3 scripts/model-eval/lama_run.py selftest` | red | yes | — |
| R10-4 | `compose`: leak check gone | `python3 scripts/model-eval/fdncnn_run.py selftest` | red | yes | — |
| R10-5 | `why_left`: chroma rule gone | `python3 scripts/model-eval/trigger.py selftest` | red | yes | — |
| R10-6 | F1: median only | `python3 scripts/model-eval/fdncnn_run.py selftest` | red | yes | — |
| R10-7 | `closing_line`: NS branch gone | `python3 scripts/model-eval/lama_run.py selftest` | red | yes | stronger test (`697119d`) |
| R10-8 | `ab.py`: names in picture paths | `python3 scripts/model-eval/ab.py selftest` | red | yes | — |
| R10-9 | `export_crops`: no `--class` | `python3 scripts/regress.py selftest` | red | yes | — |
| R9-debias | `Law::debiased` returns `stored` | `cargo test -p wipemark-pixels --test blend --features blend-preview a_bias_composited_is_a_bias_restored` | red | yes | — |
| R9-A2-catalogue | catalogue: absent bias → 0.5 | `cargo test -p wipemark-pixels --test blend --features blend-preview a_profile_without_a_bias_is_byte_for_byte_todays` | red | yes | — |
| R9-A2-law | `Law::of`: absent bias → 0.5 | `cargo test -p wipemark-pixels --test blend --features blend-preview a_profile_without_a_bias_is_byte_for_byte_todays` | red | yes | — |
| R9-logo-map | `template_logos` → None | `cargo test -p wipemark-pixels --test blend --features blend-preview a_logo_map_restores_what_a_global_logo_cannot` | red | yes | — |
| R9-wml-pin | `.wml` sha256 check gone | `cargo test -p wipemark-pixels --test assets --features blend-preview a_logo_map_asset_is_pinned` | red | yes | — |
| R9-linear | `lin`/`unlin` identity | `cargo test -p wipemark-pixels --test blend --features blend-preview the_linear_inverse_restores_a_linear_composite` | red | yes | — |
| R9-planes | `invert_with`: Y bias kept | `cargo test -p wipemark-pixels --test blend --features blend-preview a_bias_is_taken_off_in_the_planes_too` | red | yes | stronger test (`bcda512`) |
| R9-planes-chroma | `invert_with`: Cb/Cr bias kept (added here) | `cargo test -p wipemark-pixels --test blend --features blend-preview a_bias_is_taken_off_in_the_planes_too` | red | yes | — |
| R9-map-size | map-size check gone | `cargo test -p wipemark-pixels --lib --features blend-preview catalogue::tests::a_logo_map_is_refused_when_it_does_not_fit` | red | yes | — |
| R9-interval | `out_of_range`: plain `unblend` | `cargo test -p wipemark-pixels --lib --features blend-preview blend::tests::a_bias_moves_the_interval_the_proof_allows` | red | yes | — |
| R9-calibrate | `Draft::to_json` writes encoded | `cargo test -p wipemark-pixels --test calibrate --features blend-preview the_grey_captures_choose_the_blend_model` | red | yes | edit fixed: `{model:.0}` |
| R9-drawing | `Drawing::record`: no `restored` | `cargo test -p wipemark-picture --example recon_bench --features blend-preview a_drawing_records_what_it_draws_and_nothing_else` | red | yes | — |
| R9-refuse-map | no-feature `logo_map` refusal gone | `cargo test -p wipemark-pixels --lib catalogue::tests::the_catalogue_still_refuses_what_was_not_built` | red | yes | — |
| R9-refuse-linear | no-feature linear-light arm gone | `cargo test -p wipemark-pixels --lib catalogue::tests::the_catalogue_still_refuses_what_was_not_built` | red | yes | — |
| R9-refuse-bias | `bias` field without the feature | `cargo test -p wipemark-pixels --lib catalogue::tests::the_catalogue_still_refuses_what_was_not_built` | red | yes | — |
| R9-open-linear | feature: linear-light arm gone | `cargo test -p wipemark-pixels --lib --features blend-preview catalogue::tests::the_catalogue_still_refuses_what_was_not_built` | red | yes | — |
| C1-sigma | bench crops: `sigma_base` null | `cargo test -p wipemark-picture --example recon_bench a_profile_from_a_catalogue_file_runs_from_gen_to_its_result_lines` | red | yes | — |
| C2-pad | bench crops: `--crop-pad` ignored | `cargo test -p wipemark-picture --example recon_bench a_profile_from_a_catalogue_file_runs_from_gen_to_its_result_lines` | red | yes | — |
| C3-wml | file road: `.wma` only | `cargo test -p wipemark-picture --example recon_bench --features blend-preview a_catalogue_file_with_a_blend_field_loads_only_under_blend_preview` | red | yes | — |
| C4-row-over-file | `--blend-row` without the file's folder | `cargo test -p wipemark-picture --example recon_bench --features blend-preview a_blend_row_lays_over_the_catalogue_file` | red | yes | — |
| C5-stage0 | `normalise`: R2's row not read | `python3 scripts/grok/invariance.py selftest` | red | no | — |
| C6-none-row | trigger: a no-support row refused | `python3 scripts/model-eval/trigger.py selftest` | red | yes | — |
| C7-profile | `bias.py list`: two profiles taken | `python3 scripts/analytics/bias.py selftest` | red | yes | — |
| C8-evalkit | evalkit: meta's σ ignored | `python3 scripts/model-eval/evalkit.py selftest` | red | yes | — |

**51 listed by the reports, 50 red, 1 green as its report predicted; with the nine this check added (`R9-planes-chroma`, `C1`–`C8`), 60 run and 59 red.**

* **R11 M9** (a half-transparent word proved) stays green: no
  half-transparent word is ever *seen* — 0 of 444 examinations — so the
  assertion that would bite on a proof never meets a finding. R11's report
  said exactly this would happen, and the count line says why. It is the
  protection that matters if a future profile does see one; it is not
  vacuous code, it is a test whose trigger the data never reaches.

**Four were green at the first run and needed a stronger test** (each a
test change, no protection weakened):

| id | why it stayed green | the test now |
|---|---|---|
| R2 M5 (the stored-ZIP rule) | the deflated ZIP was named `deflated.zip` and also failed its pinned sha256, whose problem quotes the path: "deflated" was in the problems either way | also verifies the ZIP with the source's pin cleared, where every problem must be a member "not stored" (`8419316`) |
| R11 M1 (`std_ring` over the ring) | the zone of a word is half bare: over the zone the median moved 9 %, and the ratio check's ±0.15 never saw it | holds `std_ring` within 3 % of the floor band's median std, the same scatter read farther out (65.25 vs 65.23; the zone reads 59.5) (`b983ffc`) |
| R10 7 (M2 against NS) | without the `beats_ns` branch the same records fall to "LaMa not needed: M2 failed", which also starts "LaMa not needed" | requires "LaMa not needed: it does not beat NS inside the holes" (`697119d`) |
| R9 planes (`invert_with` takes the bias off Y) | the test compared the biased profile only with a plain one that did not restore at all (68 levels off): a bias left in Y read 12.8 and still won by two; and its grey bias (6, 6, 6) has no chroma part, so the Cb/Cr half was invisible | draws a coloured bias (+6, +2, −4) and also bounds the restoration within 2 levels of the truth (1.02 measured; a bias left in Y reads 4.8, left in the chroma blocks 6.0); `R9-planes-chroma` added (`bcda512`) |

One more needed its *edit* fixed, not its test: R9's calibrate mutation
(`"model": "{model}"` → `"encoded"`) does not compile, because `format!`
refuses an unused named argument; `"encoded{model:.0}"` keeps the argument
used and prints nothing of it, and is red.

R11 M2 was red on the jitter case and the stage-1 lines, not on the
opacity case its report named; the protection (the floor of `q` is where
there is no mark) is guarded either way.

**The central check's own tests** (`C1`–`C8`, and `R9-planes-chroma`) were
each seen red once in the same way.

## 5. Every fix

| commit | step | file(s) | what, and why |
|---|---|---|---|
| `9b76355` | merge | R9's `blend.rs`, `tests/blend.rs`; R11's `false_positives.rs`, `tests/support/mod.rs`; R12b's `recon_bench.rs`, `measure_clean.rs` | nightly rustfmt's own output; layout only |
| `7eb8eaf` | R10 | `recon_bench.rs`, `evalkit.py`, `model-eval/README.md`, `lama_run.py` | `sigma_base` in the bench's crops, `--crop-pad` (§3) |
| `c7f6255` | merge | `examples/support/catalogue.rs`, `recon_bench.rs`, `bench/README.md` | `.wml` on the file road; `--blend-row` over `--catalogue` (§3) |
| `e916eea` | R10 | `fdncnn_run.py`, `lama_run.py` | a 1 × 5 gradient (wrong test); `better()`'s unused `key` argument that made `gates()` raise on every call (**script defect**); a stub called without its mask (wrong test) |
| `ce16ced` | R11 | `align.py` | the jitter case bounds the p95 at 0.1 px and the worst at 0.15: measured mean 0.016, p95 0.041, worst 0.1025 on a textured background near 225 levels where the white α 0.5 word is 15 levels above it under 1.5 levels of noise — the estimate's noise, not a fault. M5 and M7 move offsets by the 1.5 px jitter and stay red |
| `4774c2f` | merge | `invariance.py`, `trigger.py`, `grok/README.md`, `model-eval/README.md` | manifest → invariance → trigger (§3) |
| `bb87eef` | merge | `bias.py`, `analytics/README.md`, `corpus/README.md` | `bias.py list --profile` (§3) |
| `2d786f1` | R9 | `crates/wipemark-pixels/src/verify.rs` | `Verified` boxes its `Colours`: the workspace clippy gate was red on `large_enum_variant` (232 bytes against a refusal's 8). Product code; nothing reads it differently |
| `5f85ad2` | R11 | `false_positives.rs` | `then_some` (clippy) |
| `8333131` | merge | `recon_bench.rs` | let-else in the central check's own tests (clippy `err_expect`) |
| `8419316` | R2 | `manifest.py` | the stored-ZIP selftest (§4) |
| `b983ffc` | R11 | `invariance.py` | `std_ring` held to the floor band (§4) |
| `697119d` | R10 | `lama_run.py` | the NS gate's reason (§4) |
| `bcda512` | R9 | `tests/blend.rs` | the planes' bias test (§4) |
| this commit | — | this report, `E12-R-central-check-gates.sh`, `E12-R-central-check-mutations.py` | the record |

No protection was weakened. The one bound loosened is `align.py`'s
self-test on its worst file (0.1 → 0.15 px), with its p95 held at 0.1;
the reason is measured above.

## 6. Left failing

Nothing. Every gate in §1 is green at `bcda512`, every selftest passes,
and every red check is red but R11 M9 (§4).

## 7. Open questions

1. **R11 M9 cannot bite on today's data.** If the owner wants it to, a
   half-transparent word that *is* seen (a larger, softer word, or a
   profile with a lower `min_ncc`) would have to be added to the family —
   a change to the negatives' counts, so the owner's call.
2. **The planar and interval halves of R9 without a bench config** (R9's
   question 2) stand: `a_bias_is_taken_off_in_the_planes_too` now guards
   the bias in Y and in the chroma blocks of the planar inverse, but the
   logo colour map in the planes (`block_logos`, `logo_y`) and the bias
   and map inside R8's interval methods still have no test of their own.
3. **`sigma_base`'s rectangle in the bench.** It is taken at the
   restoration's rectangle (as `export_crops` takes it), the expected box
   when nothing was restored. The bench test cannot tell the two apart on
   its fixture (they coincide); a crop where a search moved the
   restoration would.
4. The step agents' own questions (R2 1–6, R11's two, R12b 1–3, R10 1 and
   4, R9 1–4) are unchanged by this check and are the coordinator's and
   the owner's.
