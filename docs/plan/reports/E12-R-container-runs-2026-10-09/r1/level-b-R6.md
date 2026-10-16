# Regression — route detect, lossy, model: **fail**

Before: `4b5ba175fd3d2d054057e65484e695dbced9969f` (baseline, 2026-10-09, wipemark-cli 0.1.0). After: `37a301e4388522515f43d8d2ac4fae9524f7658c` (run, 2026-10-09, wipemark-cli 0.1.0). Pillow 12.3.0, libjpeg 6.2, Python 3.13.7.

Corpus gates: **G2** ok (exit3_before 285, exit3_after 284); **G1** ok (files 0, failed 0); **L4/D3** ok (files 0, verified_after 0)
* M3: no gemini-midtone in the corpus — not checked
* M4: level A is R5's bench (A5, the matrix) — not checked here
* new field `consistency_dct` (--new-fields): added on 0 of 413 files — none carried it: is the CLI under test the change's?
* new field `consistency_excluded` (--new-fields): added on 272 of 413 files
* new field `consistency_px` (--new-fields): added on 272 of 413 files
* new field `interval` (--new-fields): added on 0 of 413 files — none carried it: is the CLI under test the change's?
* new field `planar` (--new-fields): added on 194 of 413 files
* new field `smoothed` (--new-fields): added on 0 of 413 files — none carried it: is the CLI under test the change's?
* Out-of-range refusals lifted (L3): 02_skeptical_look-c1025-q90-420 (exit 3), 02_skeptical_look-c1040-q90-420 (exit 3), 02_skeptical_look-q85-420 (exit 3), 02_skeptical_look-q90-420 (exit 3), 05_torch_and_cross-c1025-q90-420 (exit 3), 05_torch_and_cross-c1040-q90-420 (exit 3), 05_torch_and_cross-q85-420 (exit 3), 05_torch_and_cross-q90-420 (exit 3), 06_thumbs_up-c1025-q90-420 (exit 3), 06_thumbs_up-c1040-q90-420 (exit 3), 06_thumbs_up-q85-420 (exit 3), 06_thumbs_up-q90-420 (exit 3), 07_shock-c1025-q90-420 (exit 3), 07_shock-c1040-q90-420 (exit 3), 07_shock-q85-420 (exit 3), 07_shock-q90-420 (exit 3), 08_facepalm-c1025-q90-420 (exit 3), 08_facepalm-q85-420 (exit 3), 09_thinking-c1025-q90-420 (exit 3), 09_thinking-q85-420 (exit 3), 10_this_is_fine-c1025-q90-420 (exit 3), 10_this_is_fine-c1040-q90-420 (exit 3), 10_this_is_fine-q85-420 (exit 3), 10_this_is_fine-q90-420 (exit 3), 10_this_is_fine_alternative-c1025-q90-420 (exit 3), 10_this_is_fine_alternative-c1025-q95-420 (exit 3), 10_this_is_fine_alternative-c1040-q90-420 (exit 3), 10_this_is_fine_alternative-q85-420 (exit 3), 10_this_is_fine_alternative-q90-420 (exit 3), 11_crying-c1025-q90-420 (exit 3), 11_crying-c1040-q90-420 (exit 3), 11_crying-q75-420 (exit 3), 11_crying-q85-420 (exit 3), 11_crying-q90-420 (exit 3), 13_angry-c1025-q90-420 (exit 3), 13_angry-q85-420 (exit 3), 14_sleeping-q85-420 (exit 3), 15_coffee-c1025-q90-420 (exit 3), 15_coffee-c1040-q90-420 (exit 3), 15_coffee-q85-420 (exit 3), 15_coffee-q90-420 (exit 3), 16_laptop-c1025-q90-420 (exit 3), 16_laptop-q85-420 (exit 3), 17_magnifying_glass-c1025-q90-420 (exit 3), 18_stamp-c1025-q90-420 (exit 3), 18_stamp-q85-420 (exit 3), 19_victory-c1025-q90-420 (exit 3), 19_victory-c1025-q95-420 (exit 3), 19_victory-c1040-q90-420 (exit 3), 19_victory-q85-420 (exit 3), 19_victory-q90-420 (exit 3), 20_waving-c1025-q90-420 (exit 3), 20_waving-c1040-q90-420 (exit 3), 20_waving-q85-420 (exit 3), 20_waving-q90-420 (exit 3), victory-1025-q95-420 (exit 3)

## 1. Class × variant

| class | variant | n | pass | fail | attention |
|---|---|---:|---:|---:|---:|
| frames | c1024-png | 21 | 21 | 0 | 0 |
| frames | c1024-q90-420 | 21 | 21 | 0 | 0 |
| frames | c1024-q95-420 | 21 | 21 | 0 | 0 |
| frames | c1025-png | 21 | 21 | 0 | 0 |
| frames | c1025-q90-420 | 21 | 1 | 4 | 16 |
| frames | c1025-q95-420 | 21 | 19 | 0 | 2 |
| frames | c1040-png | 21 | 21 | 0 | 0 |
| frames | c1040-q90-420 | 21 | 11 | 0 | 10 |
| frames | c1040-q95-420 | 21 | 21 | 0 | 0 |
| recon-jpeg-420 | fixture-q95 | 4 | 3 | 0 | 1 |
| recon-jpeg-420 | fixture-q98 | 1 | 1 | 0 | 0 |
| recon-jpeg-420 | q75 | 21 | 17 | 0 | 4 |
| recon-jpeg-420 | q85 | 21 | 0 | 5 | 16 |
| recon-jpeg-420 | q90 | 21 | 11 | 0 | 10 |
| recon-jpeg-420 | q95 | 21 | 20 | 0 | 1 |
| recon-jpeg-444 | fixture-q95 | 1 | 1 | 0 | 0 |
| recon-jpeg-444 | fixture-q98 | 1 | 1 | 0 | 0 |
| recon-jpeg-444 | q90 | 21 | 19 | 0 | 2 |
| recon-jpeg-444 | q95 | 21 | 20 | 0 | 1 |
| recon-png | fixture-png | 4 | 4 | 0 | 0 |
| recon-png | png | 21 | 21 | 0 | 0 |
| recon-resized | x0.9-png | 21 | 21 | 0 | 0 |
| recon-resized | x0.9-q90-420 | 21 | 21 | 0 | 0 |
| recon-resized | x1.1-png | 21 | 21 | 0 | 0 |
| recon-webp | fixture-q90 | 1 | 1 | 0 | 0 |
| transparent | fixture-png | 1 | 1 | 0 | 0 |
| transparent | fixture-webp-lossy | 1 | 1 | 0 | 0 |

## 2. Exit or `written` changed, by name

| file | clean exit | inspect exit | written |
|---|---|---|---|
| victory-1025-q98-420 | 3→1 | 1→1 | True→True |

## 3. Attention

* **02_skeptical_look-c1025-q90-420** (frames c1025-q90-420): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **02_skeptical_look-c1040-q90-420** (frames c1040-q90-420): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **02_skeptical_look-q75-420** (recon-jpeg-420 q75): time ×1.52 (1.79→2.72 s)
* **02_skeptical_look-q85-420** (recon-jpeg-420 q85): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **02_skeptical_look-q90-420** (recon-jpeg-420 q90): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **05_torch_and_cross-c1025-q90-420** (frames c1025-q90-420): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **05_torch_and_cross-c1040-q90-420** (frames c1040-q90-420): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **05_torch_and_cross-q85-420** (recon-jpeg-420 q85): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **05_torch_and_cross-q90-420** (recon-jpeg-420 q90): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **06_thumbs_up-c1025-q90-420** (frames c1025-q90-420): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **06_thumbs_up-c1040-q90-420** (frames c1040-q90-420): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **06_thumbs_up-q75-420** (recon-jpeg-420 q75): time ×1.56 (1.69→2.64 s)
* **06_thumbs_up-q85-420** (recon-jpeg-420 q85): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **06_thumbs_up-q90-420** (recon-jpeg-420 q90): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **07_shock-c1025-q90-420** (frames c1025-q90-420): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **07_shock-c1040-q90-420** (frames c1040-q90-420): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **07_shock-q85-420** (recon-jpeg-420 q85): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **07_shock-q90-420** (recon-jpeg-420 q90): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **08_facepalm-c1025-q90-420** (frames c1025-q90-420): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **08_facepalm-q75-420** (recon-jpeg-420 q75): time ×1.88 (1.71→3.23 s)
* **08_facepalm-q85-420** (recon-jpeg-420 q85): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **08_facepalm-q90-444** (recon-jpeg-444 q90): time ×1.50 (1.78→2.67 s)
* **09_thinking-c1025-q90-420** (frames c1025-q90-420): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **09_thinking-q85-420** (recon-jpeg-420 q85): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **09_thinking-q90-444** (recon-jpeg-444 q90): time ×1.88 (1.78→3.35 s)
* **09_thinking-q95-420** (recon-jpeg-420 q95): time ×1.78 (1.83→3.26 s)
* **09_thinking-q95-444** (recon-jpeg-444 q95): time ×1.87 (1.83→3.41 s)
* **10_this_is_fine-c1025-q90-420** (frames c1025-q90-420): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **10_this_is_fine-c1040-q90-420** (frames c1040-q90-420): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **10_this_is_fine-q85-420** (recon-jpeg-420 q85): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **10_this_is_fine-q90-420** (recon-jpeg-420 q90): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **10_this_is_fine_alternative-c1025-q90-420** (frames c1025-q90-420): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **10_this_is_fine_alternative-c1025-q95-420** (frames c1025-q95-420): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **10_this_is_fine_alternative-c1040-q90-420** (frames c1040-q90-420): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **10_this_is_fine_alternative-q85-420** (recon-jpeg-420 q85): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **10_this_is_fine_alternative-q90-420** (recon-jpeg-420 q90): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **11_crying-c1025-q90-420** (frames c1025-q90-420): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **11_crying-c1040-q90-420** (frames c1040-q90-420): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **11_crying-q75-420** (recon-jpeg-420 q75): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **11_crying-q85-420** (recon-jpeg-420 q85): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **11_crying-q90-420** (recon-jpeg-420 q90): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **13_angry-c1025-q90-420** (frames c1025-q90-420): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **13_angry-q85-420** (recon-jpeg-420 q85): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **14_sleeping-q85-420** (recon-jpeg-420 q85): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **15_coffee-c1025-q90-420** (frames c1025-q90-420): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **15_coffee-c1040-q90-420** (frames c1040-q90-420): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **15_coffee-q85-420** (recon-jpeg-420 q85): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **15_coffee-q90-420** (recon-jpeg-420 q90): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **16_laptop-c1025-q90-420** (frames c1025-q90-420): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **16_laptop-q85-420** (recon-jpeg-420 q85): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **17_magnifying_glass-c1025-q90-420** (frames c1025-q90-420): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **18_stamp-c1025-q90-420** (frames c1025-q90-420): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **18_stamp-q85-420** (recon-jpeg-420 q85): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **19_victory-c1025-q90-420** (frames c1025-q90-420): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **19_victory-c1025-q95-420** (frames c1025-q95-420): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **19_victory-c1040-q90-420** (frames c1040-q90-420): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **19_victory-q85-420** (recon-jpeg-420 q85): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **19_victory-q90-420** (recon-jpeg-420 q90): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **20_waving-c1025-q90-420** (frames c1025-q90-420): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **20_waving-c1040-q90-420** (frames c1040-q90-420): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **20_waving-q85-420** (recon-jpeg-420 q85): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **20_waving-q90-420** (recon-jpeg-420 q90): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **victory-1025-q95-420** (recon-jpeg-420 fixture-q95): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)

## 4. Fail

* **01_pointing_finger-c1025-q90-420** (frames c1025-q90-420): L2: outline worse 0.09901878→0.11235566 (past max(0.01, 5%))
* **01_pointing_finger-q85-420** (recon-jpeg-420 q85): L2: outline worse 0.14130431→0.1552186 (past max(0.01, 5%))
* **03_blessing-c1025-q90-420** (frames c1025-q90-420): L2: outline worse 0.101934046→0.11640462 (past max(0.01, 5%))
* **03_blessing-q85-420** (recon-jpeg-420 q85): L2: outline worse 0.14686118→0.15973638 (past max(0.01, 5%))
* **04_tearing_scroll-c1025-q90-420** (frames c1025-q90-420): L2: outline worse 0.09716942→0.10719314 (past max(0.01, 5%))
* **04_tearing_scroll-q85-420** (recon-jpeg-420 q85): L2: outline worse 0.14511198→0.15815392 (past max(0.01, 5%))
* **12_laughing-c1025-q90-420** (frames c1025-q90-420): L2: outline worse 0.10102868→0.113964915 (past max(0.01, 5%))
* **12_laughing-q85-420** (recon-jpeg-420 q85): L2: outline worse 0.14597207→0.15904142 (past max(0.01, 5%))
* **17_magnifying_glass-q85-420** (recon-jpeg-420 q85): L2: outline worse 0.14110021→0.15421654 (past max(0.01, 5%))

## 5. Measures per class (min / median / p95 / max)

| class | measure | n | before | after |
|---|---|---:|---|---|
| frames | outline | 126 | 0 / 0.06013 / 0.1091 / 0.1119 | 0 / 0.06102 / 0.1151 / 0.127 |
| frames | step | 126 | -4.193 / -0.005014 / 2.224 / 2.819 | -4.008 / 0.04654 / 2.363 / 2.795 |
| frames | chroma | 126 | 0.1103 / 7.742 / 9.109 / 9.364 | 0.02769 / 0.3699 / 0.8563 / 1.137 |
| frames | texture | 126 | 1.591 / 10.34 / 12.36 / 12.75 | 1.591 / 6.901 / 10.52 / 10.92 |
| frames | out_of_range | 126 | 0 / 0.00552 / 0.01356 / 0.01724 | 0 / 0 / 0 / 0 |
| frames | holes | 126 | 0 / 0 / 0 / 0 | 0 / 0 / 0 / 0 |
| frames | clamped | 126 | 0 / 219 / 414 / 421 | 0 / 36.5 / 288 / 392 |
| frames | consistency_px | 126 | — | 0.1986 / 0.244 / 0.2946 / 0.3101 |
| recon-jpeg-420 | outline | 69 | 0.05889 / 0.07546 / 0.1451 / 0.1469 | 0.02493 / 0.1129 / 0.1582 / 0.2236 |
| recon-jpeg-420 | step | 69 | -4.193 / -0.03637 / 3.639 / 4.046 | -4.008 / 1.93 / 4.164 / 6.67 |
| recon-jpeg-420 | chroma | 69 | 7.399 / 8.239 / 9.288 / 9.364 | 0.2103 / 0.5618 / 0.7976 / 0.8563 |
| recon-jpeg-420 | texture | 69 | 9.447 / 10.74 / 14.32 / 14.55 | 3.375 / 8.959 / 13.14 / 16.66 |
| recon-jpeg-420 | out_of_range | 89 | 0.001356 / 0.01046 / 0.02247 / 0.0244 | 0 / 0 / 0 / 0 |
| recon-jpeg-420 | holes | 69 | 0 / 0 / 0 / 0 | 0 / 0 / 0 / 0 |
| recon-jpeg-420 | clamped | 69 | 193 / 334 / 421 / 423 | 3 / 151 / 423 / 495 |
| recon-jpeg-420 | consistency_px | 69 | — | 0.1913 / 0.2146 / 0.2946 / 0.2999 |
| recon-jpeg-444 | outline | 44 | 0.02235 / 0.0767 / 0.1081 / 0.119 | 0.02235 / 0.0767 / 0.1081 / 0.119 |
| recon-jpeg-444 | step | 44 | -3.952 / 0.1618 / 2.411 / 2.756 | -3.952 / 0.1618 / 2.411 / 2.756 |
| recon-jpeg-444 | chroma | 44 | 0.3749 / 2.872 / 4.976 / 5.017 | 0.3749 / 2.872 / 4.976 / 5.017 |
| recon-jpeg-444 | texture | 44 | 5.223 / 9.253 / 11.68 / 11.95 | 5.223 / 9.253 / 11.68 / 11.95 |
| recon-jpeg-444 | out_of_range | 44 | 0 / 0.0006295 / 0.004358 / 0.004455 | 0 / 0.0006295 / 0.004358 / 0.004455 |
| recon-jpeg-444 | holes | 44 | 0 / 0 / 0 / 0 | 0 / 0 / 0 / 0 |
| recon-jpeg-444 | clamped | 44 | 15 / 97.5 / 226 / 240 | 15 / 97.5 / 226 / 240 |
| recon-jpeg-444 | consistency_px | 44 | — | 0.2437 / 0.2442 / 0.2451 / 0.2452 |
| recon-png | outline | 25 | 0 / 0.001893 / 0.0398 / 0.0398 | 0 / 0.001893 / 0.0398 / 0.0398 |
| recon-png | step | 25 | -3.67 / -0.003764 / 0.304 / 13.2 | -3.67 / -0.003764 / 0.304 / 13.2 |
| recon-png | chroma | 25 | 0.1103 / 0.2222 / 0.8452 / 8.208 | 0.1103 / 0.2222 / 0.8452 / 8.208 |
| recon-png | texture | 25 | 1.591 / 1.795 / 2.05 / 10.45 | 1.591 / 1.795 / 2.05 / 10.45 |
| recon-png | out_of_range | 25 | 0 / 0 / 0 / 0 | 0 / 0 / 0 / 0 |
| recon-png | holes | 25 | 0 / 0 / 0 / 0 | 0 / 0 / 0 / 0 |
| recon-png | clamped | 25 | 0 / 0 / 37 / 180 | 0 / 0 / 37 / 180 |
| recon-png | consistency_px | 25 | — | 0.2439 / 0.2442 / 0.2451 / 0.2453 |
| recon-resized | outline | 63 | 0.1105 / 0.1194 / 0.1553 / 0.1728 | 0.1105 / 0.1194 / 0.1472 / 0.1638 |
| recon-resized | step | 63 | -4.69 / -3.188 / -1.821 / -1.686 | -4.69 / -3.188 / -1.73 / -1.678 |
| recon-resized | chroma | 63 | 0.9827 / 1.395 / 7.121 / 7.201 | 0.5428 / 1.176 / 1.436 / 1.563 |
| recon-resized | texture | 63 | 4.711 / 5.599 / 11.22 / 11.59 | 4.711 / 5.599 / 9.427 / 9.689 |
| recon-resized | out_of_range | 63 | 0 / 0 / 0.005716 / 0.006859 | 0 / 0 / 0 / 0.0006933 |
| recon-resized | holes | 63 | 0 / 0 / 0 / 0 | 0 / 0 / 0 / 0 |
| recon-resized | clamped | 63 | 0 / 42 / 366 / 401 | 0 / 39 / 207 / 272 |
| recon-resized | consistency_px | 63 | — | 0.2175 / 0.2585 / 0.2755 / 0.2761 |
| recon-webp | outline | 1 | 0.07835 / 0.07835 / 0.07835 / 0.07835 | 0.07835 / 0.07835 / 0.07835 / 0.07835 |
| recon-webp | step | 1 | 0.8926 / 0.8926 / 0.8926 / 0.8926 | 0.8926 / 0.8926 / 0.8926 / 0.8926 |
| recon-webp | chroma | 1 | 6.427 / 6.427 / 6.427 / 6.427 | 6.427 / 6.427 / 6.427 / 6.427 |
| recon-webp | texture | 1 | 10.11 / 10.11 / 10.11 / 10.11 | 10.11 / 10.11 / 10.11 / 10.11 |
| recon-webp | out_of_range | 1 | 0.004455 / 0.004455 / 0.004455 / 0.004455 | 0.004455 / 0.004455 / 0.004455 / 0.004455 |
| recon-webp | holes | 1 | 0 / 0 / 0 / 0 | 0 / 0 / 0 / 0 |
| recon-webp | clamped | 1 | 271 / 271 / 271 / 271 | 271 / 271 / 271 / 271 |
| recon-webp | consistency_px | 1 | — | 0.2447 / 0.2447 / 0.2447 / 0.2447 |
| transparent | out_of_range | 1 | 0 / 0 / 0 / 0 | 0 / 0 / 0 / 0 |

## 6. Time per file (slower than ×1.5 from 0.25 s is attention)

Before: median 1.78 s, max 6.65 s, total 900 s. After: median 2.14 s, max 4.23 s, total 962 s.
* 02_skeptical_look-q75-420: 1.79→2.72 s
* 06_thumbs_up-q75-420: 1.69→2.64 s
* 08_facepalm-q75-420: 1.71→3.23 s
* 08_facepalm-q90-444: 1.78→2.67 s
* 09_thinking-q90-444: 1.78→3.35 s
* 09_thinking-q95-420: 1.83→3.26 s
* 09_thinking-q95-444: 1.83→3.41 s
