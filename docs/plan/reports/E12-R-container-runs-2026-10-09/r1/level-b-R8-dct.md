# Regression — route detect, lossy, model: **attention**

Before: `4b5ba175fd3d2d054057e65484e695dbced9969f` (baseline, 2026-10-09, wipemark-cli 0.1.0). After: `37a301e4388522515f43d8d2ac4fae9524f7658c` (run, 2026-10-09, wipemark-cli 0.1.0). Pillow 12.3.0, libjpeg 6.2, Python 3.13.7.

Corpus gates: **G2** ok (exit3_before 285, exit3_after 195); **G1** ok (files 0, failed 0); **L4/D3** ok (files 0, verified_after 0)
* M3: no gemini-midtone in the corpus — not checked
* M4: level A is R5's bench (A5, the matrix) — not checked here
* new field `consistency_dct` (--new-fields): added on 162 of 413 files
* new field `consistency_excluded` (--new-fields): added on 272 of 413 files
* new field `consistency_px` (--new-fields): added on 272 of 413 files
* new field `interval` (--new-fields): added on 162 of 413 files
* new field `planar` (--new-fields): added on 194 of 413 files
* new field `smoothed` (--new-fields): added on 0 of 413 files — none carried it: is the CLI under test the change's?
* Out-of-range refusals lifted (L3): 02_skeptical_look-c1025-q90-420 (exit 3), 02_skeptical_look-c1040-q90-420 (exit 3), 02_skeptical_look-q85-420 (exit 3), 02_skeptical_look-q90-420 (exit 3), 05_torch_and_cross-c1025-q90-420 (exit 3), 05_torch_and_cross-c1040-q90-420 (exit 3), 05_torch_and_cross-q85-420 (exit 3), 05_torch_and_cross-q90-420 (exit 3), 06_thumbs_up-c1025-q90-420 (exit 3), 06_thumbs_up-c1040-q90-420 (exit 3), 06_thumbs_up-q85-420 (exit 3), 06_thumbs_up-q90-420 (exit 3), 07_shock-c1025-q90-420 (exit 3), 07_shock-c1040-q90-420 (exit 3), 07_shock-q85-420 (exit 3), 07_shock-q90-420 (exit 3), 08_facepalm-c1025-q90-420 (exit 3), 08_facepalm-q85-420 (exit 3), 09_thinking-c1025-q90-420 (exit 3), 09_thinking-q85-420 (exit 3), 10_this_is_fine-c1025-q90-420 (exit 3), 10_this_is_fine-c1040-q90-420 (exit 3), 10_this_is_fine-q85-420 (exit 3), 10_this_is_fine-q90-420 (exit 3), 10_this_is_fine_alternative-c1025-q90-420 (exit 3), 10_this_is_fine_alternative-c1025-q95-420 (exit 1), 10_this_is_fine_alternative-c1040-q90-420 (exit 3), 10_this_is_fine_alternative-q85-420 (exit 3), 10_this_is_fine_alternative-q90-420 (exit 3), 11_crying-c1025-q90-420 (exit 3), 11_crying-c1040-q90-420 (exit 3), 11_crying-q75-420 (exit 3), 11_crying-q85-420 (exit 1), 11_crying-q90-420 (exit 3), 13_angry-c1025-q90-420 (exit 3), 13_angry-q85-420 (exit 3), 14_sleeping-q85-420 (exit 3), 15_coffee-c1025-q90-420 (exit 3), 15_coffee-c1040-q90-420 (exit 3), 15_coffee-q85-420 (exit 3), 15_coffee-q90-420 (exit 3), 16_laptop-c1025-q90-420 (exit 3), 16_laptop-q85-420 (exit 3), 17_magnifying_glass-c1025-q90-420 (exit 3), 18_stamp-c1025-q90-420 (exit 3), 18_stamp-q85-420 (exit 3), 19_victory-c1025-q90-420 (exit 3), 19_victory-c1025-q95-420 (exit 1), 19_victory-c1040-q90-420 (exit 3), 19_victory-q85-420 (exit 3), 19_victory-q90-420 (exit 3), 20_waving-c1025-q90-420 (exit 3), 20_waving-c1040-q90-420 (exit 3), 20_waving-q85-420 (exit 3), 20_waving-q90-420 (exit 3), victory-1025-q95-420 (exit 1)

## 1. Class × variant

| class | variant | n | pass | fail | attention |
|---|---|---:|---:|---:|---:|
| frames | c1024-png | 21 | 21 | 0 | 0 |
| frames | c1024-q90-420 | 21 | 21 | 0 | 0 |
| frames | c1024-q95-420 | 21 | 21 | 0 | 0 |
| frames | c1025-png | 21 | 21 | 0 | 0 |
| frames | c1025-q90-420 | 21 | 5 | 0 | 16 |
| frames | c1025-q95-420 | 21 | 19 | 0 | 2 |
| frames | c1040-png | 21 | 21 | 0 | 0 |
| frames | c1040-q90-420 | 21 | 10 | 0 | 11 |
| frames | c1040-q95-420 | 21 | 21 | 0 | 0 |
| recon-jpeg-420 | fixture-q95 | 4 | 3 | 0 | 1 |
| recon-jpeg-420 | fixture-q98 | 1 | 1 | 0 | 0 |
| recon-jpeg-420 | q75 | 21 | 17 | 0 | 4 |
| recon-jpeg-420 | q85 | 21 | 5 | 0 | 16 |
| recon-jpeg-420 | q90 | 21 | 10 | 0 | 11 |
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
| 01_pointing_finger-c1025-q95-420 | 3→1 | 1→1 | True→True |
| 01_pointing_finger-c1040-q95-420 | 3→1 | 1→1 | True→True |
| 01_pointing_finger-q95-420 | 3→1 | 1→1 | True→True |
| 01_pointing_finger-q95-444 | 3→1 | 1→1 | True→True |
| 02_skeptical_look-c1025-q95-420 | 3→1 | 1→1 | True→True |
| 02_skeptical_look-c1040-q95-420 | 3→1 | 1→1 | True→True |
| 02_skeptical_look-q95-420 | 3→1 | 1→1 | True→True |
| 02_skeptical_look-q95-444 | 3→1 | 1→1 | True→True |
| 03_blessing-c1025-q95-420 | 3→1 | 1→1 | True→True |
| 03_blessing-c1040-q95-420 | 3→1 | 1→1 | True→True |
| 03_blessing-q95-420 | 3→1 | 1→1 | True→True |
| 03_blessing-q95-444 | 3→1 | 1→1 | True→True |
| 04_tearing_scroll-c1025-q95-420 | 3→1 | 1→1 | True→True |
| 04_tearing_scroll-c1040-q95-420 | 3→1 | 1→1 | True→True |
| 04_tearing_scroll-q95-420 | 3→1 | 1→1 | True→True |
| 04_tearing_scroll-q95-444 | 3→1 | 1→1 | True→True |
| 05_torch_and_cross-c1025-q95-420 | 3→1 | 1→1 | True→True |
| 05_torch_and_cross-c1040-q95-420 | 3→1 | 1→1 | True→True |
| 05_torch_and_cross-q95-420 | 3→1 | 1→1 | True→True |
| 05_torch_and_cross-q95-444 | 3→1 | 1→1 | True→True |
| 06_thumbs_up-c1025-q95-420 | 3→1 | 1→1 | True→True |
| 06_thumbs_up-c1040-q95-420 | 3→1 | 1→1 | True→True |
| 06_thumbs_up-q95-420 | 3→1 | 1→1 | True→True |
| 06_thumbs_up-q95-444 | 3→1 | 1→1 | True→True |
| 07_shock-c1025-q95-420 | 3→1 | 1→1 | True→True |
| 07_shock-c1040-q95-420 | 3→1 | 1→1 | True→True |
| 07_shock-q95-420 | 3→1 | 1→1 | True→True |
| 07_shock-q95-444 | 3→1 | 1→1 | True→True |
| 08_facepalm-c1025-q95-420 | 3→1 | 1→1 | True→True |
| 08_facepalm-c1040-q95-420 | 3→1 | 1→1 | True→True |
| 08_facepalm-q95-420 | 3→1 | 1→1 | True→True |
| 08_facepalm-q95-444 | 3→1 | 1→1 | True→True |
| 09_thinking-c1025-q95-420 | 3→1 | 1→1 | True→True |
| 09_thinking-c1040-q95-420 | 3→1 | 1→1 | True→True |
| 09_thinking-q95-420 | 3→1 | 1→1 | True→True |
| 09_thinking-q95-444 | 3→1 | 1→1 | True→True |
| 10_this_is_fine-c1025-q95-420 | 3→1 | 1→1 | True→True |
| 10_this_is_fine-c1040-q95-420 | 3→1 | 1→1 | True→True |
| 10_this_is_fine-q95-420 | 3→1 | 1→1 | True→True |
| 10_this_is_fine-q95-444 | 3→1 | 1→1 | True→True |
| 10_this_is_fine_alternative-c1025-q95-420 | 3→1 | 1→1 | True→True |
| 10_this_is_fine_alternative-c1040-q95-420 | 3→1 | 1→1 | True→True |
| 10_this_is_fine_alternative-q95-420 | 3→1 | 1→1 | True→True |
| 10_this_is_fine_alternative-q95-444 | 3→1 | 1→1 | True→True |
| 11_crying-q85-420 | 3→1 | 1→1 | True→True |
| 12_laughing-c1025-q95-420 | 3→1 | 1→1 | True→True |
| 12_laughing-c1040-q95-420 | 3→1 | 1→1 | True→True |
| 12_laughing-q95-420 | 3→1 | 1→1 | True→True |
| 12_laughing-q95-444 | 3→1 | 1→1 | True→True |
| 13_angry-c1025-q95-420 | 3→1 | 1→1 | True→True |
| 13_angry-c1040-q95-420 | 3→1 | 1→1 | True→True |
| 13_angry-q95-420 | 3→1 | 1→1 | True→True |
| 13_angry-q95-444 | 3→1 | 1→1 | True→True |
| 14_sleeping-c1025-q95-420 | 3→1 | 1→1 | True→True |
| 14_sleeping-c1040-q95-420 | 3→1 | 1→1 | True→True |
| 14_sleeping-q95-420 | 3→1 | 1→1 | True→True |
| 14_sleeping-q95-444 | 3→1 | 1→1 | True→True |
| 15_coffee-c1025-q95-420 | 3→1 | 1→1 | True→True |
| 15_coffee-c1040-q95-420 | 3→1 | 1→1 | True→True |
| 15_coffee-q95-420 | 3→1 | 1→1 | True→True |
| 15_coffee-q95-444 | 3→1 | 1→1 | True→True |
| 16_laptop-c1025-q95-420 | 3→1 | 1→1 | True→True |
| 16_laptop-c1040-q95-420 | 3→1 | 1→1 | True→True |
| 16_laptop-q95-420 | 3→1 | 1→1 | True→True |
| 16_laptop-q95-444 | 3→1 | 1→1 | True→True |
| 17_magnifying_glass-c1025-q95-420 | 3→1 | 1→1 | True→True |
| 17_magnifying_glass-c1040-q90-420 | 3→1 | 1→1 | True→True |
| 17_magnifying_glass-c1040-q95-420 | 3→1 | 1→1 | True→True |
| 17_magnifying_glass-q90-420 | 3→1 | 1→1 | True→True |
| 17_magnifying_glass-q90-444 | 3→1 | 1→1 | True→True |
| 17_magnifying_glass-q95-420 | 3→1 | 1→1 | True→True |
| 17_magnifying_glass-q95-444 | 3→1 | 1→1 | True→True |
| 18_stamp-c1025-q95-420 | 3→1 | 1→1 | True→True |
| 18_stamp-c1040-q95-420 | 3→1 | 1→1 | True→True |
| 18_stamp-q95-420 | 3→1 | 1→1 | True→True |
| 18_stamp-q95-444 | 3→1 | 1→1 | True→True |
| 19_victory-c1025-q95-420 | 3→1 | 1→1 | True→True |
| 19_victory-c1040-q95-420 | 3→1 | 1→1 | True→True |
| 19_victory-q95-420 | 3→1 | 1→1 | True→True |
| 19_victory-q95-444 | 3→1 | 1→1 | True→True |
| 20_waving-c1025-q95-420 | 3→1 | 1→1 | True→True |
| 20_waving-c1040-q95-420 | 3→1 | 1→1 | True→True |
| 20_waving-q95-420 | 3→1 | 1→1 | True→True |
| 20_waving-q95-444 | 3→1 | 1→1 | True→True |
| thinking-1040-q95-420 | 3→1 | 1→1 | True→True |
| torch-1025-q95-420 | 3→1 | 1→1 | True→True |
| torch-1025-q95-444 | 3→1 | 1→1 | True→True |
| victory-1025-q95-420 | 3→1 | 1→1 | True→True |
| victory-1025-q98-420 | 3→1 | 1→1 | True→True |
| victory-1040-q95-420 | 3→1 | 1→1 | True→True |

## 3. Attention

* **02_skeptical_look-c1025-q90-420** (frames c1025-q90-420): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **02_skeptical_look-c1040-q90-420** (frames c1040-q90-420): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **02_skeptical_look-q85-420** (recon-jpeg-420 q85): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **02_skeptical_look-q90-420** (recon-jpeg-420 q90): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **05_torch_and_cross-c1025-q90-420** (frames c1025-q90-420): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **05_torch_and_cross-c1040-q90-420** (frames c1040-q90-420): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **05_torch_and_cross-q85-420** (recon-jpeg-420 q85): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **05_torch_and_cross-q90-420** (recon-jpeg-420 q90): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **06_thumbs_up-c1025-q90-420** (frames c1025-q90-420): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **06_thumbs_up-c1040-q90-420** (frames c1040-q90-420): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **06_thumbs_up-q75-420** (recon-jpeg-420 q75): time ×1.52 (1.69→2.57 s)
* **06_thumbs_up-q85-420** (recon-jpeg-420 q85): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **06_thumbs_up-q90-420** (recon-jpeg-420 q90): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **07_shock-c1025-q90-420** (frames c1025-q90-420): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **07_shock-c1040-q90-420** (frames c1040-q90-420): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **07_shock-q85-420** (recon-jpeg-420 q85): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **07_shock-q90-420** (recon-jpeg-420 q90): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **08_facepalm-c1025-q90-420** (frames c1025-q90-420): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **08_facepalm-q75-420** (recon-jpeg-420 q75): time ×1.98 (1.71→3.39 s)
* **08_facepalm-q85-420** (recon-jpeg-420 q85): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **09_thinking-c1025-q90-420** (frames c1025-q90-420): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **09_thinking-q75-420** (recon-jpeg-420 q75): time ×1.53 (1.74→2.67 s)
* **09_thinking-q85-420** (recon-jpeg-420 q85): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **09_thinking-q90-444** (recon-jpeg-444 q90): time ×1.93 (1.78→3.44 s)
* **09_thinking-q95-420** (recon-jpeg-420 q95): G2: exit 3→1, every measure within its bound; time ×1.66 (1.83→3.02 s)
* **09_thinking-q95-444** (recon-jpeg-444 q95): G2: exit 3→1, every measure within its bound; time ×1.94 (1.83→3.54 s)
* **10_this_is_fine-c1025-q90-420** (frames c1025-q90-420): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **10_this_is_fine-c1040-q90-420** (frames c1040-q90-420): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **10_this_is_fine-q85-420** (recon-jpeg-420 q85): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **10_this_is_fine-q90-420** (recon-jpeg-420 q90): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **10_this_is_fine_alternative-c1025-q90-420** (frames c1025-q90-420): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **10_this_is_fine_alternative-c1025-q95-420** (frames c1025-q95-420): G2: exit 3→1, every measure within its bound; L3: an out-of-range refusal lifted, exit 1; D2: the verified findings changed (0→1)
* **10_this_is_fine_alternative-c1040-q90-420** (frames c1040-q90-420): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **10_this_is_fine_alternative-q85-420** (recon-jpeg-420 q85): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **10_this_is_fine_alternative-q90-420** (recon-jpeg-420 q90): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **11_crying-c1025-q90-420** (frames c1025-q90-420): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **11_crying-c1040-q90-420** (frames c1040-q90-420): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **11_crying-q75-420** (recon-jpeg-420 q75): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **11_crying-q85-420** (recon-jpeg-420 q85): G2: exit 3→1, but texture over its bound: not counted; L3: an out-of-range refusal lifted, exit 1; D2: the verified findings changed (0→1)
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
* **17_magnifying_glass-c1040-q90-420** (frames c1040-q90-420): G2: exit 3→1, but step over its bound: not counted
* **17_magnifying_glass-q90-420** (recon-jpeg-420 q90): G2: exit 3→1, but step over its bound: not counted
* **17_magnifying_glass-q90-444** (recon-jpeg-444 q90): G2: exit 3→1, but step over its bound: not counted
* **18_stamp-c1025-q90-420** (frames c1025-q90-420): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **18_stamp-q85-420** (recon-jpeg-420 q85): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **19_victory-c1025-q90-420** (frames c1025-q90-420): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **19_victory-c1025-q95-420** (frames c1025-q95-420): G2: exit 3→1, every measure within its bound; L3: an out-of-range refusal lifted, exit 1; D2: the verified findings changed (0→1)
* **19_victory-c1040-q90-420** (frames c1040-q90-420): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **19_victory-q85-420** (recon-jpeg-420 q85): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **19_victory-q90-420** (recon-jpeg-420 q90): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **20_waving-c1025-q90-420** (frames c1025-q90-420): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **20_waving-c1040-q90-420** (frames c1040-q90-420): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **20_waving-q85-420** (recon-jpeg-420 q85): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **20_waving-q90-420** (recon-jpeg-420 q90): L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement; D2: the verified findings changed (0→1)
* **victory-1025-q95-420** (recon-jpeg-420 fixture-q95): G2: exit 3→1, every measure within its bound; L3: an out-of-range refusal lifted, exit 1; D2: the verified findings changed (0→1)

## 4. Fail

None.

## 5. Measures per class (min / median / p95 / max)

| class | measure | n | before | after |
|---|---|---:|---|---|
| frames | outline | 126 | 0 / 0.06013 / 0.1091 / 0.1119 | 0 / 0.02953 / 0.06587 / 0.07702 |
| frames | step | 126 | -4.193 / -0.005014 / 2.224 / 2.819 | -3.67 / 0.06898 / 2.083 / 2.498 |
| frames | chroma | 126 | 0.1103 / 7.742 / 9.109 / 9.364 | 0.1103 / 0.5441 / 1.413 / 1.541 |
| frames | texture | 126 | 1.591 / 10.34 / 12.36 / 12.75 | 1.591 / 3.183 / 4.63 / 5.432 |
| frames | out_of_range | 126 | 0 / 0.00552 / 0.01356 / 0.01724 | 0 / 0 / 0 / 0 |
| frames | holes | 126 | 0 / 0 / 0 / 0 | 0 / 0 / 0 / 0 |
| frames | clamped | 126 | 0 / 219 / 414 / 421 | 0 / 0 / 39 / 61 |
| frames | consistency_px | 126 | — | 0.2439 / 1.899 / 3.134 / 3.217 |
| recon-jpeg-420 | outline | 69 | 0.05889 / 0.07546 / 0.1451 / 0.1469 | 0.0134 / 0.06398 / 0.107 / 0.1857 |
| recon-jpeg-420 | step | 69 | -4.193 / -0.03637 / 3.639 / 4.046 | -3.66 / 1.687 / 3.761 / 6.135 |
| recon-jpeg-420 | chroma | 69 | 7.399 / 8.239 / 9.288 / 9.364 | 0.2339 / 1.258 / 1.646 / 1.769 |
| recon-jpeg-420 | texture | 69 | 9.447 / 10.74 / 14.32 / 14.55 | 1.844 / 3.765 / 8.261 / 12.85 |
| recon-jpeg-420 | out_of_range | 89 | 0.001356 / 0.01046 / 0.02247 / 0.0244 | 0 / 0 / 0 / 0 |
| recon-jpeg-420 | holes | 69 | 0 / 0 / 0 / 0 | 0 / 0 / 0 / 0 |
| recon-jpeg-420 | clamped | 69 | 193 / 334 / 421 / 423 | 0 / 15 / 118 / 263 |
| recon-jpeg-420 | consistency_px | 69 | — | 0.9422 / 2.861 / 3.027 / 3.087 |
| recon-jpeg-444 | outline | 44 | 0.02235 / 0.0767 / 0.1081 / 0.119 | 0.004881 / 0.04117 / 0.06363 / 0.07431 |
| recon-jpeg-444 | step | 44 | -3.952 / 0.1618 / 2.411 / 2.756 | -3.463 / 0.2001 / 2.093 / 2.43 |
| recon-jpeg-444 | chroma | 44 | 0.3749 / 2.872 / 4.976 / 5.017 | 0.3669 / 2.091 / 3.855 / 3.884 |
| recon-jpeg-444 | texture | 44 | 5.223 / 9.253 / 11.68 / 11.95 | 1.628 / 3.233 / 5.419 / 5.938 |
| recon-jpeg-444 | out_of_range | 44 | 0 / 0.0006295 / 0.004358 / 0.004455 | 0 / 0.0006295 / 0.004358 / 0.004455 |
| recon-jpeg-444 | holes | 44 | 0 / 0 / 0 / 0 | 0 / 0 / 0 / 0 |
| recon-jpeg-444 | clamped | 44 | 15 / 97.5 / 226 / 240 | 0 / 15 / 46 / 52 |
| recon-jpeg-444 | consistency_px | 44 | — | 2.071 / 3.674 / 4.472 / 4.514 |
| recon-png | outline | 25 | 0 / 0.001893 / 0.0398 / 0.0398 | 0 / 0.001893 / 0.0398 / 0.0398 |
| recon-png | step | 25 | -3.67 / -0.003764 / 0.304 / 13.2 | -3.67 / -0.003764 / 0.304 / 13.2 |
| recon-png | chroma | 25 | 0.1103 / 0.2222 / 0.8452 / 8.208 | 0.1103 / 0.2222 / 0.8452 / 8.208 |
| recon-png | texture | 25 | 1.591 / 1.795 / 2.05 / 10.45 | 1.591 / 1.795 / 2.05 / 10.45 |
| recon-png | out_of_range | 25 | 0 / 0 / 0 / 0 | 0 / 0 / 0 / 0 |
| recon-png | holes | 25 | 0 / 0 / 0 / 0 | 0 / 0 / 0 / 0 |
| recon-png | clamped | 25 | 0 / 0 / 37 / 180 | 0 / 0 / 37 / 180 |
| recon-png | consistency_px | 25 | — | 0.2439 / 0.2442 / 0.2451 / 0.2453 |
| recon-resized | outline | 63 | 0.1105 / 0.1194 / 0.1553 / 0.1728 | 0.07645 / 0.1138 / 0.122 / 0.1588 |
| recon-resized | step | 63 | -4.69 / -3.188 / -1.821 / -1.686 | -4.69 / -3.188 / -1.388 / -1.238 |
| recon-resized | chroma | 63 | 0.9827 / 1.395 / 7.121 / 7.201 | 0.9531 / 1.255 / 1.44 / 1.563 |
| recon-resized | texture | 63 | 4.711 / 5.599 / 11.22 / 11.59 | 3.823 / 4.883 / 5.724 / 6.398 |
| recon-resized | out_of_range | 63 | 0 / 0 / 0.005716 / 0.006859 | 0 / 0 / 0 / 0.0006933 |
| recon-resized | holes | 63 | 0 / 0 / 0 / 0 | 0 / 0 / 0 / 0 |
| recon-resized | clamped | 63 | 0 / 42 / 366 / 401 | 0 / 10 / 96 / 112 |
| recon-resized | consistency_px | 63 | — | 0.2481 / 0.2652 / 3.158 / 3.194 |
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

Before: median 1.78 s, max 6.65 s, total 900 s. After: median 2.14 s, max 4.24 s, total 965 s.
* 06_thumbs_up-q75-420: 1.69→2.57 s
* 08_facepalm-q75-420: 1.71→3.39 s
* 09_thinking-q75-420: 1.74→2.67 s
* 09_thinking-q90-444: 1.78→3.44 s
* 09_thinking-q95-420: 1.83→3.02 s
* 09_thinking-q95-444: 1.83→3.54 s
