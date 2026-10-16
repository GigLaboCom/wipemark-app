# Regression — route detect, lossy, model: **attention**

Before: `4b5ba175fd3d2d054057e65484e695dbced9969f` (baseline, 2026-10-09, wipemark-cli 0.1.0). After: `4b5ba175fd3d2d054057e65484e695dbced9969f` (run, 2026-10-09, wipemark-cli 0.1.0). Pillow 12.3.0, libjpeg 6.2, Python 3.13.7.

Corpus gates: **G2** ok (exit3_before 285, exit3_after 285); **G1** ok (files 0, failed 0); **L4/D3** ok (files 0, verified_after 0)
* M3: no gemini-midtone in the corpus — not checked
* M4: level A is R5's bench (A5, the matrix) — not checked here

## 1. Class × variant

| class | variant | n | pass | fail | attention |
|---|---|---:|---:|---:|---:|
| frames | c1024-png | 21 | 1 | 0 | 20 |
| frames | c1024-q90-420 | 21 | 1 | 0 | 20 |
| frames | c1024-q95-420 | 21 | 0 | 0 | 21 |
| frames | c1025-png | 21 | 1 | 0 | 20 |
| frames | c1025-q90-420 | 21 | 1 | 0 | 20 |
| frames | c1025-q95-420 | 21 | 1 | 0 | 20 |
| frames | c1040-png | 21 | 1 | 0 | 20 |
| frames | c1040-q90-420 | 21 | 1 | 0 | 20 |
| frames | c1040-q95-420 | 21 | 0 | 0 | 21 |
| recon-jpeg-420 | fixture-q95 | 4 | 1 | 0 | 3 |
| recon-jpeg-420 | fixture-q98 | 1 | 0 | 0 | 1 |
| recon-jpeg-420 | q75 | 21 | 2 | 0 | 19 |
| recon-jpeg-420 | q85 | 21 | 0 | 0 | 21 |
| recon-jpeg-420 | q90 | 21 | 0 | 0 | 21 |
| recon-jpeg-420 | q95 | 21 | 3 | 0 | 18 |
| recon-jpeg-444 | fixture-q95 | 1 | 0 | 0 | 1 |
| recon-jpeg-444 | fixture-q98 | 1 | 1 | 0 | 0 |
| recon-jpeg-444 | q90 | 21 | 1 | 0 | 20 |
| recon-jpeg-444 | q95 | 21 | 0 | 0 | 21 |
| recon-png | fixture-png | 4 | 2 | 0 | 2 |
| recon-png | png | 21 | 4 | 0 | 17 |
| recon-resized | x0.9-png | 21 | 17 | 0 | 4 |
| recon-resized | x0.9-q90-420 | 21 | 19 | 0 | 2 |
| recon-resized | x1.1-png | 21 | 19 | 0 | 2 |
| recon-webp | fixture-q90 | 1 | 1 | 0 | 0 |
| transparent | fixture-png | 1 | 1 | 0 | 0 |
| transparent | fixture-webp-lossy | 1 | 1 | 0 | 0 |

## 2. Exit or `written` changed, by name

None.

## 3. Attention

* **01_pointing_finger-c1024** (frames c1024-png): time ×1.73 (1.66→2.87 s)
* **01_pointing_finger-c1024-q90-420** (frames c1024-q90-420): time ×1.86 (1.63→3.04 s)
* **01_pointing_finger-c1024-q95-420** (frames c1024-q95-420): time ×1.83 (1.72→3.14 s)
* **01_pointing_finger-c1025** (frames c1025-png): time ×1.84 (1.79→3.29 s)
* **01_pointing_finger-c1025-q90-420** (frames c1025-q90-420): time ×1.93 (1.60→3.08 s)
* **01_pointing_finger-c1025-q95-420** (frames c1025-q95-420): time ×1.76 (1.66→2.93 s)
* **01_pointing_finger-c1040** (frames c1040-png): time ×1.85 (1.74→3.22 s)
* **01_pointing_finger-c1040-q90-420** (frames c1040-q90-420): time ×1.89 (1.57→2.96 s)
* **01_pointing_finger-c1040-q95-420** (frames c1040-q95-420): time ×1.64 (1.64→2.68 s)
* **01_pointing_finger-q85-420** (recon-jpeg-420 q85): time ×1.72 (1.87→3.21 s)
* **01_pointing_finger-q90-420** (recon-jpeg-420 q90): time ×1.74 (1.98→3.45 s)
* **01_pointing_finger-q90-444** (recon-jpeg-444 q90): time ×1.69 (1.93→3.26 s)
* **01_pointing_finger-q95-420** (recon-jpeg-420 q95): time ×2.08 (1.87→3.90 s)
* **01_pointing_finger-q95-444** (recon-jpeg-444 q95): time ×1.78 (1.84→3.27 s)
* **01_pointing_finger-x0.9** (recon-resized x0.9-png): time ×1.55 (3.03→4.71 s)
* **01_pointing_finger-x0.9-q90-420** (recon-resized x0.9-q90-420): time ×1.87 (2.27→4.25 s)
* **01_pointing_finger-x1.1** (recon-resized x1.1-png): time ×1.59 (3.62→5.75 s)
* **02_skeptical_look-c1024** (frames c1024-png): time ×1.90 (1.72→3.28 s)
* **02_skeptical_look-c1024-q90-420** (frames c1024-q90-420): time ×1.66 (1.71→2.84 s)
* **02_skeptical_look-c1024-q95-420** (frames c1024-q95-420): time ×1.83 (1.70→3.12 s)
* **02_skeptical_look-c1025** (frames c1025-png): time ×1.80 (1.79→3.22 s)
* **02_skeptical_look-c1025-q90-420** (frames c1025-q90-420): time ×1.64 (1.78→2.92 s)
* **02_skeptical_look-c1025-q95-420** (frames c1025-q95-420): time ×1.89 (1.60→3.01 s)
* **02_skeptical_look-c1040** (frames c1040-png): time ×1.65 (1.86→3.07 s)
* **02_skeptical_look-c1040-q90-420** (frames c1040-q90-420): time ×1.64 (1.65→2.70 s)
* **02_skeptical_look-c1040-q95-420** (frames c1040-q95-420): time ×1.70 (1.62→2.75 s)
* **02_skeptical_look-q75-420** (recon-jpeg-420 q75): time ×1.75 (1.79→3.12 s)
* **02_skeptical_look-q85-420** (recon-jpeg-420 q85): time ×1.70 (1.78→3.03 s)
* **02_skeptical_look-q90-420** (recon-jpeg-420 q90): time ×1.69 (1.87→3.15 s)
* **02_skeptical_look-q95-444** (recon-jpeg-444 q95): time ×1.56 (2.06→3.20 s)
* **02_skeptical_look-x0.9** (recon-resized x0.9-png): time ×1.58 (3.20→5.05 s)
* **03_blessing** (recon-png png): time ×1.71 (2.50→4.27 s)
* **03_blessing-c1024** (frames c1024-png): time ×2.31 (1.63→3.76 s)
* **03_blessing-c1024-q90-420** (frames c1024-q90-420): time ×1.78 (1.61→2.87 s)
* **03_blessing-c1024-q95-420** (frames c1024-q95-420): time ×1.82 (1.62→2.96 s)
* **03_blessing-c1025** (frames c1025-png): time ×1.62 (1.79→2.90 s)
* **03_blessing-c1025-q90-420** (frames c1025-q90-420): time ×1.95 (1.61→3.14 s)
* **03_blessing-c1025-q95-420** (frames c1025-q95-420): time ×1.86 (1.66→3.09 s)
* **03_blessing-c1040** (frames c1040-png): time ×1.81 (1.80→3.25 s)
* **03_blessing-c1040-q90-420** (frames c1040-q90-420): time ×2.14 (1.76→3.77 s)
* **03_blessing-c1040-q95-420** (frames c1040-q95-420): time ×1.87 (1.60→2.99 s)
* **03_blessing-q75-420** (recon-jpeg-420 q75): time ×1.76 (1.72→3.01 s)
* **03_blessing-q85-420** (recon-jpeg-420 q85): time ×1.79 (1.81→3.23 s)
* **03_blessing-q90-420** (recon-jpeg-420 q90): time ×1.76 (2.05→3.61 s)
* **03_blessing-q90-444** (recon-jpeg-444 q90): time ×1.59 (2.14→3.40 s)
* **03_blessing-q95-420** (recon-jpeg-420 q95): time ×1.74 (2.00→3.48 s)
* **03_blessing-q95-444** (recon-jpeg-444 q95): time ×1.71 (2.04→3.49 s)
* **04_tearing_scroll** (recon-png png): time ×1.55 (2.68→4.16 s)
* **04_tearing_scroll-c1024** (frames c1024-png): time ×1.88 (1.64→3.08 s)
* **04_tearing_scroll-c1024-q90-420** (frames c1024-q90-420): time ×1.83 (1.62→2.96 s)
* **04_tearing_scroll-c1024-q95-420** (frames c1024-q95-420): time ×1.78 (1.63→2.89 s)
* **04_tearing_scroll-c1025** (frames c1025-png): time ×1.76 (1.81→3.19 s)
* **04_tearing_scroll-c1025-q90-420** (frames c1025-q90-420): time ×1.93 (1.61→3.12 s)
* **04_tearing_scroll-c1025-q95-420** (frames c1025-q95-420): time ×1.78 (1.61→2.87 s)
* **04_tearing_scroll-c1040** (frames c1040-png): time ×1.93 (1.75→3.36 s)
* **04_tearing_scroll-c1040-q90-420** (frames c1040-q90-420): time ×2.21 (1.54→3.41 s)
* **04_tearing_scroll-c1040-q95-420** (frames c1040-q95-420): time ×1.81 (1.59→2.87 s)
* **04_tearing_scroll-q75-420** (recon-jpeg-420 q75): time ×1.60 (1.81→2.89 s)
* **04_tearing_scroll-q85-420** (recon-jpeg-420 q85): time ×1.79 (1.77→3.17 s)
* **04_tearing_scroll-q90-420** (recon-jpeg-420 q90): time ×1.65 (1.91→3.15 s)
* **04_tearing_scroll-q90-444** (recon-jpeg-444 q90): time ×1.76 (1.96→3.46 s)
* **04_tearing_scroll-q95-444** (recon-jpeg-444 q95): time ×1.81 (1.92→3.47 s)
* **05_torch_and_cross** (recon-png png): time ×1.55 (2.78→4.33 s)
* **05_torch_and_cross-c1024** (frames c1024-png): time ×1.85 (1.68→3.10 s)
* **05_torch_and_cross-c1024-q90-420** (frames c1024-q90-420): time ×1.63 (1.75→2.86 s)
* **05_torch_and_cross-c1024-q95-420** (frames c1024-q95-420): time ×1.84 (1.67→3.06 s)
* **05_torch_and_cross-c1025** (frames c1025-png): time ×2.14 (1.72→3.69 s)
* **05_torch_and_cross-c1025-q90-420** (frames c1025-q90-420): time ×1.87 (1.75→3.28 s)
* **05_torch_and_cross-c1025-q95-420** (frames c1025-q95-420): time ×1.90 (1.63→3.10 s)
* **05_torch_and_cross-c1040** (frames c1040-png): time ×2.06 (1.71→3.52 s)
* **05_torch_and_cross-c1040-q90-420** (frames c1040-q90-420): time ×1.78 (1.66→2.95 s)
* **05_torch_and_cross-c1040-q95-420** (frames c1040-q95-420): time ×1.86 (1.65→3.06 s)
* **05_torch_and_cross-q75-420** (recon-jpeg-420 q75): time ×1.71 (1.80→3.07 s)
* **05_torch_and_cross-q85-420** (recon-jpeg-420 q85): time ×1.62 (1.77→2.87 s)
* **05_torch_and_cross-q90-420** (recon-jpeg-420 q90): time ×1.76 (1.76→3.09 s)
* **05_torch_and_cross-q90-444** (recon-jpeg-444 q90): time ×1.56 (2.03→3.16 s)
* **05_torch_and_cross-q95-420** (recon-jpeg-420 q95): time ×1.77 (1.85→3.27 s)
* **05_torch_and_cross-q95-444** (recon-jpeg-444 q95): time ×1.84 (1.95→3.60 s)
* **06_thumbs_up** (recon-png png): time ×1.56 (2.77→4.33 s)
* **06_thumbs_up-c1024** (frames c1024-png): time ×1.87 (1.69→3.18 s)
* **06_thumbs_up-c1024-q90-420** (frames c1024-q90-420): time ×1.91 (1.68→3.21 s)
* **06_thumbs_up-c1024-q95-420** (frames c1024-q95-420): time ×2.32 (1.72→3.99 s)
* **06_thumbs_up-c1025** (frames c1025-png): time ×1.81 (1.86→3.36 s)
* **06_thumbs_up-c1025-q90-420** (frames c1025-q90-420): time ×1.75 (1.90→3.32 s)
* **06_thumbs_up-c1025-q95-420** (frames c1025-q95-420): time ×1.88 (1.66→3.12 s)
* **06_thumbs_up-c1040** (frames c1040-png): time ×1.65 (1.86→3.08 s)
* **06_thumbs_up-c1040-q90-420** (frames c1040-q90-420): time ×1.69 (1.63→2.77 s)
* **06_thumbs_up-c1040-q95-420** (frames c1040-q95-420): time ×2.05 (1.65→3.37 s)
* **06_thumbs_up-q75-420** (recon-jpeg-420 q75): time ×1.75 (1.69→2.96 s)
* **06_thumbs_up-q85-420** (recon-jpeg-420 q85): time ×1.75 (1.72→3.00 s)
* **06_thumbs_up-q90-420** (recon-jpeg-420 q90): time ×1.86 (1.76→3.28 s)
* **06_thumbs_up-q90-444** (recon-jpeg-444 q90): time ×2.07 (1.83→3.79 s)
* **06_thumbs_up-q95-420** (recon-jpeg-420 q95): time ×1.81 (1.94→3.50 s)
* **06_thumbs_up-q95-444** (recon-jpeg-444 q95): time ×1.96 (1.91→3.73 s)
* **06_thumbs_up-x0.9-q90-420** (recon-resized x0.9-q90-420): time ×1.72 (2.22→3.81 s)
* **06_thumbs_up-x1.1** (recon-resized x1.1-png): time ×1.62 (3.57→5.76 s)
* **07_shock** (recon-png png): time ×1.64 (2.84→4.65 s)
* **07_shock-c1024** (frames c1024-png): time ×1.62 (1.68→2.74 s)
* **07_shock-c1024-q90-420** (frames c1024-q90-420): time ×1.94 (1.68→3.26 s)
* **07_shock-c1024-q95-420** (frames c1024-q95-420): time ×1.58 (1.77→2.81 s)
* **07_shock-c1025** (frames c1025-png): time ×1.69 (1.83→3.10 s)
* **07_shock-c1025-q90-420** (frames c1025-q90-420): time ×1.81 (1.68→3.04 s)
* **07_shock-c1025-q95-420** (frames c1025-q95-420): time ×1.87 (1.65→3.10 s)
* **07_shock-c1040** (frames c1040-png): time ×1.64 (1.86→3.06 s)
* **07_shock-c1040-q90-420** (frames c1040-q90-420): time ×1.65 (1.63→2.69 s)
* **07_shock-c1040-q95-420** (frames c1040-q95-420): time ×1.66 (1.61→2.66 s)
* **07_shock-q75-420** (recon-jpeg-420 q75): time ×1.91 (1.71→3.26 s)
* **07_shock-q85-420** (recon-jpeg-420 q85): time ×1.68 (1.73→2.89 s)
* **07_shock-q90-420** (recon-jpeg-420 q90): time ×2.01 (1.72→3.46 s)
* **07_shock-q90-444** (recon-jpeg-444 q90): time ×1.74 (1.82→3.17 s)
* **07_shock-q95-420** (recon-jpeg-420 q95): time ×1.84 (1.81→3.34 s)
* **07_shock-q95-444** (recon-jpeg-444 q95): time ×1.76 (1.85→3.25 s)
* **07_shock-x0.9** (recon-resized x0.9-png): time ×1.68 (2.77→4.65 s)
* **08_facepalm-c1024** (frames c1024-png): time ×1.66 (1.74→2.89 s)
* **08_facepalm-c1024-q90-420** (frames c1024-q90-420): time ×1.68 (1.75→2.95 s)
* **08_facepalm-c1024-q95-420** (frames c1024-q95-420): time ×1.77 (1.72→3.05 s)
* **08_facepalm-c1025** (frames c1025-png): time ×1.78 (1.81→3.23 s)
* **08_facepalm-c1025-q90-420** (frames c1025-q90-420): time ×1.76 (1.65→2.90 s)
* **08_facepalm-c1025-q95-420** (frames c1025-q95-420): time ×1.68 (1.64→2.77 s)
* **08_facepalm-c1040** (frames c1040-png): time ×1.78 (1.72→3.06 s)
* **08_facepalm-c1040-q90-420** (frames c1040-q90-420): time ×1.75 (1.56→2.73 s)
* **08_facepalm-c1040-q95-420** (frames c1040-q95-420): time ×1.94 (1.55→3.01 s)
* **08_facepalm-q75-420** (recon-jpeg-420 q75): time ×1.83 (1.71→3.13 s)
* **08_facepalm-q85-420** (recon-jpeg-420 q85): time ×1.85 (1.72→3.18 s)
* **08_facepalm-q90-420** (recon-jpeg-420 q90): time ×1.94 (1.78→3.45 s)
* **08_facepalm-q90-444** (recon-jpeg-444 q90): time ×1.77 (1.78→3.16 s)
* **08_facepalm-q95-420** (recon-jpeg-420 q95): time ×1.92 (1.79→3.44 s)
* **08_facepalm-q95-444** (recon-jpeg-444 q95): time ×1.73 (1.84→3.18 s)
* **09_thinking** (recon-png png): time ×1.72 (2.67→4.59 s)
* **09_thinking-c1024** (frames c1024-png): time ×1.93 (1.66→3.20 s)
* **09_thinking-c1024-q90-420** (frames c1024-q90-420): time ×1.92 (1.62→3.10 s)
* **09_thinking-c1024-q95-420** (frames c1024-q95-420): time ×1.94 (1.62→3.15 s)
* **09_thinking-c1025** (frames c1025-png): time ×1.78 (1.84→3.28 s)
* **09_thinking-c1025-q90-420** (frames c1025-q90-420): time ×1.67 (1.69→2.84 s)
* **09_thinking-c1025-q95-420** (frames c1025-q95-420): time ×1.57 (1.78→2.80 s)
* **09_thinking-c1040** (frames c1040-png): time ×1.69 (1.75→2.97 s)
* **09_thinking-c1040-q90-420** (frames c1040-q90-420): time ×1.74 (1.54→2.67 s)
* **09_thinking-c1040-q95-420** (frames c1040-q95-420): time ×1.79 (1.59→2.84 s)
* **09_thinking-q75-420** (recon-jpeg-420 q75): time ×1.79 (1.74→3.12 s)
* **09_thinking-q85-420** (recon-jpeg-420 q85): time ×1.82 (1.73→3.15 s)
* **09_thinking-q90-420** (recon-jpeg-420 q90): time ×1.65 (1.81→2.99 s)
* **09_thinking-q90-444** (recon-jpeg-444 q90): time ×1.94 (1.78→3.45 s)
* **09_thinking-q95-420** (recon-jpeg-420 q95): time ×1.95 (1.83→3.57 s)
* **09_thinking-q95-444** (recon-jpeg-444 q95): time ×1.76 (1.83→3.21 s)
* **10_this_is_fine** (recon-png png): time ×1.80 (2.74→4.93 s)
* **10_this_is_fine-c1024** (frames c1024-png): time ×1.86 (1.77→3.29 s)
* **10_this_is_fine-c1024-q90-420** (frames c1024-q90-420): time ×1.78 (1.67→2.98 s)
* **10_this_is_fine-c1024-q95-420** (frames c1024-q95-420): time ×1.99 (1.66→3.30 s)
* **10_this_is_fine-c1025** (frames c1025-png): time ×1.95 (1.75→3.42 s)
* **10_this_is_fine-c1025-q90-420** (frames c1025-q90-420): time ×1.97 (1.61→3.17 s)
* **10_this_is_fine-c1025-q95-420** (frames c1025-q95-420): time ×1.98 (1.59→3.16 s)
* **10_this_is_fine-c1040** (frames c1040-png): time ×1.94 (1.71→3.32 s)
* **10_this_is_fine-c1040-q90-420** (frames c1040-q90-420): time ×1.93 (1.60→3.09 s)
* **10_this_is_fine-c1040-q95-420** (frames c1040-q95-420): time ×1.99 (1.54→3.07 s)
* **10_this_is_fine-q75-420** (recon-jpeg-420 q75): time ×1.63 (1.69→2.75 s)
* **10_this_is_fine-q85-420** (recon-jpeg-420 q85): time ×1.88 (1.71→3.22 s)
* **10_this_is_fine-q90-420** (recon-jpeg-420 q90): time ×1.69 (1.73→2.93 s)
* **10_this_is_fine-q90-444** (recon-jpeg-444 q90): time ×1.84 (1.77→3.26 s)
* **10_this_is_fine-q95-420** (recon-jpeg-420 q95): time ×1.81 (1.82→3.30 s)
* **10_this_is_fine-q95-444** (recon-jpeg-444 q95): time ×1.66 (1.82→3.02 s)
* **10_this_is_fine_alternative** (recon-png png): time ×1.62 (2.83→4.58 s)
* **10_this_is_fine_alternative-c1024** (frames c1024-png): time ×1.75 (1.71→2.99 s)
* **10_this_is_fine_alternative-c1024-q90-420** (frames c1024-q90-420): time ×1.76 (1.66→2.92 s)
* **10_this_is_fine_alternative-c1024-q95-420** (frames c1024-q95-420): time ×1.75 (1.74→3.04 s)
* **10_this_is_fine_alternative-c1025** (frames c1025-png): time ×1.66 (1.87→3.10 s)
* **10_this_is_fine_alternative-c1025-q90-420** (frames c1025-q90-420): time ×1.69 (1.80→3.05 s)
* **10_this_is_fine_alternative-c1025-q95-420** (frames c1025-q95-420): time ×1.63 (1.75→2.85 s)
* **10_this_is_fine_alternative-c1040** (frames c1040-png): time ×1.57 (1.99→3.12 s)
* **10_this_is_fine_alternative-c1040-q90-420** (frames c1040-q90-420): time ×2.27 (1.66→3.78 s)
* **10_this_is_fine_alternative-c1040-q95-420** (frames c1040-q95-420): time ×1.76 (1.64→2.89 s)
* **10_this_is_fine_alternative-q75-420** (recon-jpeg-420 q75): time ×1.73 (1.70→2.93 s)
* **10_this_is_fine_alternative-q85-420** (recon-jpeg-420 q85): time ×1.80 (1.71→3.08 s)
* **10_this_is_fine_alternative-q90-420** (recon-jpeg-420 q90): time ×1.74 (1.73→3.02 s)
* **10_this_is_fine_alternative-q90-444** (recon-jpeg-444 q90): time ×1.87 (1.78→3.33 s)
* **10_this_is_fine_alternative-q95-420** (recon-jpeg-420 q95): time ×1.68 (1.81→3.04 s)
* **10_this_is_fine_alternative-q95-444** (recon-jpeg-444 q95): time ×1.62 (1.83→2.96 s)
* **11_crying** (recon-png png): time ×1.67 (1.35→2.25 s)
* **11_crying-c1024** (frames c1024-png): time ×1.79 (1.27→2.27 s)
* **11_crying-c1024-q90-420** (frames c1024-q90-420): time ×1.69 (1.31→2.22 s)
* **11_crying-c1024-q95-420** (frames c1024-q95-420): time ×1.70 (1.44→2.44 s)
* **11_crying-c1025** (frames c1025-png): time ×2.12 (1.06→2.23 s)
* **11_crying-c1025-q90-420** (frames c1025-q90-420): time ×1.98 (1.35→2.67 s)
* **11_crying-c1025-q95-420** (frames c1025-q95-420): time ×1.98 (1.06→2.10 s)
* **11_crying-c1040** (frames c1040-png): time ×1.86 (1.04→1.94 s)
* **11_crying-c1040-q90-420** (frames c1040-q90-420): time ×1.82 (1.26→2.29 s)
* **11_crying-c1040-q95-420** (frames c1040-q95-420): time ×1.79 (1.04→1.86 s)
* **11_crying-q75-420** (recon-jpeg-420 q75): time ×1.73 (1.29→2.24 s)
* **11_crying-q85-420** (recon-jpeg-420 q85): time ×1.68 (1.31→2.19 s)
* **11_crying-q90-420** (recon-jpeg-420 q90): time ×1.95 (1.30→2.53 s)
* **11_crying-q90-444** (recon-jpeg-444 q90): time ×1.76 (1.11→1.96 s)
* **11_crying-q95-420** (recon-jpeg-420 q95): time ×1.76 (1.17→2.07 s)
* **11_crying-q95-444** (recon-jpeg-444 q95): time ×1.86 (1.12→2.09 s)
* **11_crying-x0.9** (recon-resized x0.9-png): time ×1.54 (2.36→3.64 s)
* **12_laughing-c1024** (frames c1024-png): time ×1.90 (1.67→3.17 s)
* **12_laughing-c1024-q90-420** (frames c1024-q90-420): time ×1.83 (1.63→2.99 s)
* **12_laughing-c1024-q95-420** (frames c1024-q95-420): time ×1.83 (1.63→2.99 s)
* **12_laughing-c1025** (frames c1025-png): time ×2.11 (1.79→3.78 s)
* **12_laughing-c1025-q90-420** (frames c1025-q90-420): time ×1.72 (1.69→2.90 s)
* **12_laughing-c1025-q95-420** (frames c1025-q95-420): time ×1.83 (1.64→3.00 s)
* **12_laughing-c1040** (frames c1040-png): time ×1.69 (1.80→3.04 s)
* **12_laughing-c1040-q90-420** (frames c1040-q90-420): time ×1.71 (1.69→2.89 s)
* **12_laughing-c1040-q95-420** (frames c1040-q95-420): time ×1.65 (1.65→2.72 s)
* **12_laughing-q75-420** (recon-jpeg-420 q75): time ×1.69 (1.75→2.95 s)
* **12_laughing-q85-420** (recon-jpeg-420 q85): time ×1.82 (1.77→3.23 s)
* **12_laughing-q90-420** (recon-jpeg-420 q90): time ×1.63 (1.78→2.91 s)
* **12_laughing-q90-444** (recon-jpeg-444 q90): time ×1.68 (1.77→2.99 s)
* **12_laughing-q95-420** (recon-jpeg-420 q95): time ×1.76 (1.82→3.22 s)
* **12_laughing-q95-444** (recon-jpeg-444 q95): time ×1.64 (1.83→3.00 s)
* **13_angry** (recon-png png): time ×1.52 (2.89→4.41 s)
* **13_angry-c1024** (frames c1024-png): time ×1.84 (1.70→3.14 s)
* **13_angry-c1024-q90-420** (frames c1024-q90-420): time ×1.85 (1.67→3.08 s)
* **13_angry-c1024-q95-420** (frames c1024-q95-420): time ×2.03 (1.64→3.32 s)
* **13_angry-c1025** (frames c1025-png): time ×1.78 (1.85→3.30 s)
* **13_angry-c1025-q90-420** (frames c1025-q90-420): time ×1.83 (1.67→3.05 s)
* **13_angry-c1025-q95-420** (frames c1025-q95-420): time ×1.90 (1.60→3.05 s)
* **13_angry-c1040** (frames c1040-png): time ×1.74 (1.74→3.03 s)
* **13_angry-c1040-q90-420** (frames c1040-q90-420): time ×1.70 (1.66→2.83 s)
* **13_angry-c1040-q95-420** (frames c1040-q95-420): time ×1.78 (1.55→2.77 s)
* **13_angry-q85-420** (recon-jpeg-420 q85): time ×1.60 (1.82→2.91 s)
* **13_angry-q90-420** (recon-jpeg-420 q90): time ×1.76 (1.84→3.24 s)
* **13_angry-q90-444** (recon-jpeg-444 q90): time ×1.75 (1.83→3.20 s)
* **13_angry-q95-420** (recon-jpeg-420 q95): time ×1.58 (1.87→2.97 s)
* **13_angry-q95-444** (recon-jpeg-444 q95): time ×1.90 (1.85→3.51 s)
* **14_sleeping** (recon-png png): time ×1.62 (2.64→4.29 s)
* **14_sleeping-c1024** (frames c1024-png): time ×1.78 (1.74→3.09 s)
* **14_sleeping-c1024-q90-420** (frames c1024-q90-420): time ×1.74 (1.63→2.82 s)
* **14_sleeping-c1024-q95-420** (frames c1024-q95-420): time ×1.86 (1.66→3.09 s)
* **14_sleeping-c1025** (frames c1025-png): time ×1.80 (1.78→3.20 s)
* **14_sleeping-c1025-q90-420** (frames c1025-q90-420): time ×2.20 (1.60→3.53 s)
* **14_sleeping-c1025-q95-420** (frames c1025-q95-420): time ×1.97 (1.61→3.16 s)
* **14_sleeping-c1040** (frames c1040-png): time ×1.74 (1.72→2.99 s)
* **14_sleeping-c1040-q90-420** (frames c1040-q90-420): time ×1.87 (1.56→2.92 s)
* **14_sleeping-c1040-q95-420** (frames c1040-q95-420): time ×2.06 (1.55→3.20 s)
* **14_sleeping-q75-420** (recon-jpeg-420 q75): time ×1.70 (1.72→2.93 s)
* **14_sleeping-q85-420** (recon-jpeg-420 q85): time ×1.83 (1.82→3.32 s)
* **14_sleeping-q90-420** (recon-jpeg-420 q90): time ×1.99 (1.80→3.58 s)
* **14_sleeping-q90-444** (recon-jpeg-444 q90): time ×1.66 (1.81→3.00 s)
* **14_sleeping-q95-420** (recon-jpeg-420 q95): time ×2.21 (1.86→4.13 s)
* **14_sleeping-q95-444** (recon-jpeg-444 q95): time ×1.71 (1.92→3.28 s)
* **15_coffee** (recon-png png): time ×1.71 (2.77→4.75 s)
* **15_coffee-c1024** (frames c1024-png): time ×1.92 (1.68→3.22 s)
* **15_coffee-c1024-q90-420** (frames c1024-q90-420): time ×1.92 (1.71→3.29 s)
* **15_coffee-c1024-q95-420** (frames c1024-q95-420): time ×1.73 (1.63→2.82 s)
* **15_coffee-c1025** (frames c1025-png): time ×1.93 (1.75→3.38 s)
* **15_coffee-c1025-q90-420** (frames c1025-q90-420): time ×1.65 (1.66→2.74 s)
* **15_coffee-c1025-q95-420** (frames c1025-q95-420): time ×2.05 (1.61→3.29 s)
* **15_coffee-c1040** (frames c1040-png): time ×1.87 (1.76→3.28 s)
* **15_coffee-c1040-q90-420** (frames c1040-q90-420): time ×1.87 (1.60→2.99 s)
* **15_coffee-c1040-q95-420** (frames c1040-q95-420): time ×1.93 (1.56→3.00 s)
* **15_coffee-q75-420** (recon-jpeg-420 q75): time ×1.71 (1.68→2.88 s)
* **15_coffee-q85-420** (recon-jpeg-420 q85): time ×1.65 (1.71→2.82 s)
* **15_coffee-q90-420** (recon-jpeg-420 q90): time ×1.62 (1.72→2.79 s)
* **15_coffee-q90-444** (recon-jpeg-444 q90): time ×1.58 (1.80→2.84 s)
* **15_coffee-q95-420** (recon-jpeg-420 q95): time ×1.70 (1.82→3.09 s)
* **15_coffee-q95-444** (recon-jpeg-444 q95): time ×1.69 (1.84→3.11 s)
* **16_laptop** (recon-png png): time ×1.72 (2.72→4.68 s)
* **16_laptop-c1024** (frames c1024-png): time ×1.79 (1.66→2.96 s)
* **16_laptop-c1024-q90-420** (frames c1024-q90-420): time ×2.04 (1.63→3.32 s)
* **16_laptop-c1024-q95-420** (frames c1024-q95-420): time ×2.01 (1.64→3.30 s)
* **16_laptop-c1025** (frames c1025-png): time ×1.78 (1.82→3.24 s)
* **16_laptop-c1025-q90-420** (frames c1025-q90-420): time ×2.07 (1.63→3.37 s)
* **16_laptop-c1025-q95-420** (frames c1025-q95-420): time ×1.94 (1.60→3.10 s)
* **16_laptop-c1040** (frames c1040-png): time ×2.20 (1.71→3.75 s)
* **16_laptop-c1040-q90-420** (frames c1040-q90-420): time ×1.84 (1.59→2.92 s)
* **16_laptop-c1040-q95-420** (frames c1040-q95-420): time ×1.73 (1.62→2.81 s)
* **16_laptop-q75-420** (recon-jpeg-420 q75): time ×1.71 (1.83→3.14 s)
* **16_laptop-q85-420** (recon-jpeg-420 q85): time ×1.52 (2.01→3.05 s)
* **16_laptop-q90-420** (recon-jpeg-420 q90): time ×1.78 (1.78→3.18 s)
* **16_laptop-q90-444** (recon-jpeg-444 q90): time ×1.86 (1.80→3.35 s)
* **16_laptop-q95-420** (recon-jpeg-420 q95): time ×1.90 (1.88→3.58 s)
* **16_laptop-q95-444** (recon-jpeg-444 q95): time ×1.69 (1.81→3.07 s)
* **17_magnifying_glass** (recon-png png): time ×1.71 (2.62→4.48 s)
* **17_magnifying_glass-c1024** (frames c1024-png): time ×1.93 (1.64→3.16 s)
* **17_magnifying_glass-c1024-q90-420** (frames c1024-q90-420): time ×2.00 (1.63→3.26 s)
* **17_magnifying_glass-c1024-q95-420** (frames c1024-q95-420): time ×2.38 (1.65→3.92 s)
* **17_magnifying_glass-c1025** (frames c1025-png): time ×1.91 (1.77→3.38 s)
* **17_magnifying_glass-c1025-q90-420** (frames c1025-q90-420): time ×1.68 (1.63→2.75 s)
* **17_magnifying_glass-c1025-q95-420** (frames c1025-q95-420): time ×1.76 (1.66→2.93 s)
* **17_magnifying_glass-c1040** (frames c1040-png): time ×1.80 (1.73→3.12 s)
* **17_magnifying_glass-c1040-q90-420** (frames c1040-q90-420): time ×2.29 (1.52→3.48 s)
* **17_magnifying_glass-c1040-q95-420** (frames c1040-q95-420): time ×1.93 (1.55→2.99 s)
* **17_magnifying_glass-q75-420** (recon-jpeg-420 q75): time ×1.98 (1.73→3.41 s)
* **17_magnifying_glass-q85-420** (recon-jpeg-420 q85): time ×1.86 (1.85→3.45 s)
* **17_magnifying_glass-q90-420** (recon-jpeg-420 q90): time ×1.81 (1.90→3.43 s)
* **17_magnifying_glass-q90-444** (recon-jpeg-444 q90): time ×1.63 (2.03→3.30 s)
* **17_magnifying_glass-q95-420** (recon-jpeg-420 q95): time ×1.77 (1.98→3.50 s)
* **17_magnifying_glass-q95-444** (recon-jpeg-444 q95): time ×1.61 (2.08→3.35 s)
* **18_stamp** (recon-png png): time ×1.85 (2.71→5.02 s)
* **18_stamp-c1024** (frames c1024-png): time ×1.82 (1.68→3.07 s)
* **18_stamp-c1024-q90-420** (frames c1024-q90-420): time ×1.52 (1.71→2.61 s)
* **18_stamp-c1024-q95-420** (frames c1024-q95-420): time ×1.72 (1.81→3.12 s)
* **18_stamp-c1025** (frames c1025-png): time ×1.70 (1.92→3.26 s)
* **18_stamp-c1025-q90-420** (frames c1025-q90-420): time ×1.80 (1.63→2.93 s)
* **18_stamp-c1025-q95-420** (frames c1025-q95-420): time ×1.97 (1.58→3.11 s)
* **18_stamp-c1040** (frames c1040-png): time ×1.63 (1.88→3.06 s)
* **18_stamp-c1040-q95-420** (frames c1040-q95-420): time ×1.68 (1.54→2.59 s)
* **18_stamp-q75-420** (recon-jpeg-420 q75): time ×1.91 (1.81→3.46 s)
* **18_stamp-q85-420** (recon-jpeg-420 q85): time ×1.84 (1.78→3.28 s)
* **18_stamp-q90-420** (recon-jpeg-420 q90): time ×1.71 (1.90→3.25 s)
* **18_stamp-q90-444** (recon-jpeg-444 q90): time ×1.96 (1.85→3.63 s)
* **18_stamp-q95-444** (recon-jpeg-444 q95): time ×1.92 (1.89→3.62 s)
* **19_victory** (recon-png png): time ×1.79 (2.66→4.76 s)
* **19_victory-c1024** (frames c1024-png): time ×1.60 (1.65→2.64 s)
* **19_victory-c1024-q90-420** (frames c1024-q90-420): time ×1.58 (1.91→3.03 s)
* **19_victory-c1024-q95-420** (frames c1024-q95-420): time ×1.59 (1.70→2.70 s)
* **19_victory-c1025** (frames c1025-png): time ×1.74 (1.78→3.09 s)
* **19_victory-c1025-q90-420** (frames c1025-q90-420): time ×1.66 (1.66→2.76 s)
* **19_victory-c1025-q95-420** (frames c1025-q95-420): time ×1.74 (1.71→2.98 s)
* **19_victory-c1040-q90-420** (frames c1040-q90-420): time ×1.66 (1.61→2.66 s)
* **19_victory-c1040-q95-420** (frames c1040-q95-420): time ×1.63 (1.55→2.53 s)
* **19_victory-q75-420** (recon-jpeg-420 q75): time ×1.65 (1.92→3.17 s)
* **19_victory-q85-420** (recon-jpeg-420 q85): time ×2.00 (1.78→3.57 s)
* **19_victory-q90-420** (recon-jpeg-420 q90): time ×1.92 (1.80→3.46 s)
* **19_victory-q90-444** (recon-jpeg-444 q90): time ×2.03 (1.85→3.77 s)
* **19_victory-q95-420** (recon-jpeg-420 q95): time ×1.79 (1.88→3.37 s)
* **19_victory-q95-444** (recon-jpeg-444 q95): time ×1.80 (1.91→3.43 s)
* **20_waving** (recon-png png): time ×1.80 (2.72→4.91 s)
* **20_waving-c1024-q95-420** (frames c1024-q95-420): time ×1.51 (1.75→2.64 s)
* **20_waving-c1040** (frames c1040-png): time ×1.58 (1.83→2.88 s)
* **20_waving-c1040-q90-420** (frames c1040-q90-420): time ×1.56 (1.81→2.82 s)
* **20_waving-c1040-q95-420** (frames c1040-q95-420): time ×2.01 (1.76→3.55 s)
* **20_waving-q75-420** (recon-jpeg-420 q75): time ×1.85 (1.75→3.24 s)
* **20_waving-q85-420** (recon-jpeg-420 q85): time ×1.61 (1.96→3.14 s)
* **20_waving-q90-420** (recon-jpeg-420 q90): time ×1.74 (1.80→3.13 s)
* **20_waving-q90-444** (recon-jpeg-444 q90): time ×1.79 (1.86→3.32 s)
* **20_waving-q95-420** (recon-jpeg-420 q95): time ×1.83 (1.89→3.47 s)
* **20_waving-q95-444** (recon-jpeg-444 q95): time ×1.78 (1.97→3.50 s)
* **thinking-1040-q95-420** (recon-jpeg-420 fixture-q95): time ×1.57 (1.72→2.70 s)
* **torch-1025** (recon-png fixture-png): time ×1.69 (1.79→3.03 s)
* **torch-1025-q95-420** (recon-jpeg-420 fixture-q95): time ×1.81 (1.64→2.97 s)
* **torch-1025-q95-444** (recon-jpeg-444 fixture-q95): time ×1.69 (1.66→2.79 s)
* **victory-1025** (recon-png fixture-png): time ×1.67 (1.80→3.01 s)
* **victory-1025-q95-420** (recon-jpeg-420 fixture-q95): time ×1.71 (1.74→2.98 s)
* **victory-1025-q98-420** (recon-jpeg-420 fixture-q98): time ×1.75 (1.76→3.08 s)

## 4. Fail

None.

## 5. Measures per class (min / median / p95 / max)

| class | measure | n | before | after |
|---|---|---:|---|---|
| frames | outline | 98 | 0 / 0.06013 / 0.1091 / 0.1119 | 0 / 0.06013 / 0.1091 / 0.1119 |
| frames | step | 98 | -4.193 / -0.005014 / 2.224 / 2.819 | -4.193 / -0.005014 / 2.224 / 2.819 |
| frames | chroma | 98 | 0.1103 / 7.742 / 9.109 / 9.364 | 0.1103 / 7.742 / 9.109 / 9.364 |
| frames | texture | 98 | 1.591 / 10.34 / 12.36 / 12.75 | 1.591 / 10.34 / 12.36 / 12.75 |
| frames | out_of_range | 126 | 0 / 0.00552 / 0.01356 / 0.01724 | 0 / 0.00552 / 0.01356 / 0.01724 |
| frames | holes | 98 | 0 / 0 / 0 / 0 | 0 / 0 / 0 / 0 |
| frames | clamped | 98 | 0 / 219 / 414 / 421 | 0 / 219 / 414 / 421 |
| recon-jpeg-420 | outline | 41 | 0.05889 / 0.07546 / 0.1451 / 0.1469 | 0.05889 / 0.07546 / 0.1451 / 0.1469 |
| recon-jpeg-420 | step | 41 | -4.193 / -0.03637 / 3.639 / 4.046 | -4.193 / -0.03637 / 3.639 / 4.046 |
| recon-jpeg-420 | chroma | 41 | 7.399 / 8.239 / 9.288 / 9.364 | 7.399 / 8.239 / 9.288 / 9.364 |
| recon-jpeg-420 | texture | 41 | 9.447 / 10.74 / 14.32 / 14.55 | 9.447 / 10.74 / 14.32 / 14.55 |
| recon-jpeg-420 | out_of_range | 89 | 0.001356 / 0.01046 / 0.02247 / 0.0244 | 0.001356 / 0.01046 / 0.02247 / 0.0244 |
| recon-jpeg-420 | holes | 41 | 0 / 0 / 0 / 0 | 0 / 0 / 0 / 0 |
| recon-jpeg-420 | clamped | 41 | 193 / 334 / 421 / 423 | 193 / 334 / 421 / 423 |
| recon-jpeg-444 | outline | 44 | 0.02235 / 0.0767 / 0.1081 / 0.119 | 0.02235 / 0.0767 / 0.1081 / 0.119 |
| recon-jpeg-444 | step | 44 | -3.952 / 0.1618 / 2.411 / 2.756 | -3.952 / 0.1618 / 2.411 / 2.756 |
| recon-jpeg-444 | chroma | 44 | 0.3749 / 2.872 / 4.976 / 5.017 | 0.3749 / 2.872 / 4.976 / 5.017 |
| recon-jpeg-444 | texture | 44 | 5.223 / 9.253 / 11.68 / 11.95 | 5.223 / 9.253 / 11.68 / 11.95 |
| recon-jpeg-444 | out_of_range | 44 | 0 / 0.0006295 / 0.004358 / 0.004455 | 0 / 0.0006295 / 0.004358 / 0.004455 |
| recon-jpeg-444 | holes | 44 | 0 / 0 / 0 / 0 | 0 / 0 / 0 / 0 |
| recon-jpeg-444 | clamped | 44 | 15 / 97.5 / 226 / 240 | 15 / 97.5 / 226 / 240 |
| recon-png | outline | 25 | 0 / 0.001893 / 0.0398 / 0.0398 | 0 / 0.001893 / 0.0398 / 0.0398 |
| recon-png | step | 25 | -3.67 / -0.003764 / 0.304 / 13.2 | -3.67 / -0.003764 / 0.304 / 13.2 |
| recon-png | chroma | 25 | 0.1103 / 0.2222 / 0.8452 / 8.208 | 0.1103 / 0.2222 / 0.8452 / 8.208 |
| recon-png | texture | 25 | 1.591 / 1.795 / 2.05 / 10.45 | 1.591 / 1.795 / 2.05 / 10.45 |
| recon-png | out_of_range | 25 | 0 / 0 / 0 / 0 | 0 / 0 / 0 / 0 |
| recon-png | holes | 25 | 0 / 0 / 0 / 0 | 0 / 0 / 0 / 0 |
| recon-png | clamped | 25 | 0 / 0 / 37 / 180 | 0 / 0 / 37 / 180 |
| recon-resized | outline | 63 | 0.1105 / 0.1194 / 0.1553 / 0.1728 | 0.1105 / 0.1194 / 0.1553 / 0.1728 |
| recon-resized | step | 63 | -4.69 / -3.188 / -1.821 / -1.686 | -4.69 / -3.188 / -1.821 / -1.686 |
| recon-resized | chroma | 63 | 0.9827 / 1.395 / 7.121 / 7.201 | 0.9827 / 1.395 / 7.121 / 7.201 |
| recon-resized | texture | 63 | 4.711 / 5.599 / 11.22 / 11.59 | 4.711 / 5.599 / 11.22 / 11.59 |
| recon-resized | out_of_range | 63 | 0 / 0 / 0.005716 / 0.006859 | 0 / 0 / 0.005716 / 0.006859 |
| recon-resized | holes | 63 | 0 / 0 / 0 / 0 | 0 / 0 / 0 / 0 |
| recon-resized | clamped | 63 | 0 / 42 / 366 / 401 | 0 / 42 / 366 / 401 |
| recon-webp | outline | 1 | 0.07835 / 0.07835 / 0.07835 / 0.07835 | 0.07835 / 0.07835 / 0.07835 / 0.07835 |
| recon-webp | step | 1 | 0.8926 / 0.8926 / 0.8926 / 0.8926 | 0.8926 / 0.8926 / 0.8926 / 0.8926 |
| recon-webp | chroma | 1 | 6.427 / 6.427 / 6.427 / 6.427 | 6.427 / 6.427 / 6.427 / 6.427 |
| recon-webp | texture | 1 | 10.11 / 10.11 / 10.11 / 10.11 | 10.11 / 10.11 / 10.11 / 10.11 |
| recon-webp | out_of_range | 1 | 0.004455 / 0.004455 / 0.004455 / 0.004455 | 0.004455 / 0.004455 / 0.004455 / 0.004455 |
| recon-webp | holes | 1 | 0 / 0 / 0 / 0 | 0 / 0 / 0 / 0 |
| recon-webp | clamped | 1 | 271 / 271 / 271 / 271 | 271 / 271 / 271 / 271 |
| transparent | out_of_range | 1 | 0 / 0 / 0 / 0 | 0 / 0 / 0 / 0 |

## 6. Time per file (slower than ×1.5 from 0.25 s is attention)

Before: median 1.78 s, max 6.65 s, total 900 s. After: median 3.16 s, max 6.58 s, total 1404 s.
* 01_pointing_finger-c1024: 1.66→2.87 s
* 01_pointing_finger-c1024-q90-420: 1.63→3.04 s
* 01_pointing_finger-c1024-q95-420: 1.72→3.14 s
* 01_pointing_finger-c1025: 1.79→3.29 s
* 01_pointing_finger-c1025-q90-420: 1.60→3.08 s
* 01_pointing_finger-c1025-q95-420: 1.66→2.93 s
* 01_pointing_finger-c1040: 1.74→3.22 s
* 01_pointing_finger-c1040-q90-420: 1.57→2.96 s
* 01_pointing_finger-c1040-q95-420: 1.64→2.68 s
* 01_pointing_finger-q85-420: 1.87→3.21 s
* 01_pointing_finger-q90-420: 1.98→3.45 s
* 01_pointing_finger-q90-444: 1.93→3.26 s
* 01_pointing_finger-q95-420: 1.87→3.90 s
* 01_pointing_finger-q95-444: 1.84→3.27 s
* 01_pointing_finger-x0.9: 3.03→4.71 s
* 01_pointing_finger-x0.9-q90-420: 2.27→4.25 s
* 01_pointing_finger-x1.1: 3.62→5.75 s
* 02_skeptical_look-c1024: 1.72→3.28 s
* 02_skeptical_look-c1024-q90-420: 1.71→2.84 s
* 02_skeptical_look-c1024-q95-420: 1.70→3.12 s
* 02_skeptical_look-c1025: 1.79→3.22 s
* 02_skeptical_look-c1025-q90-420: 1.78→2.92 s
* 02_skeptical_look-c1025-q95-420: 1.60→3.01 s
* 02_skeptical_look-c1040: 1.86→3.07 s
* 02_skeptical_look-c1040-q90-420: 1.65→2.70 s
* 02_skeptical_look-c1040-q95-420: 1.62→2.75 s
* 02_skeptical_look-q75-420: 1.79→3.12 s
* 02_skeptical_look-q85-420: 1.78→3.03 s
* 02_skeptical_look-q90-420: 1.87→3.15 s
* 02_skeptical_look-q95-444: 2.06→3.20 s
* 02_skeptical_look-x0.9: 3.20→5.05 s
* 03_blessing: 2.50→4.27 s
* 03_blessing-c1024: 1.63→3.76 s
* 03_blessing-c1024-q90-420: 1.61→2.87 s
* 03_blessing-c1024-q95-420: 1.62→2.96 s
* 03_blessing-c1025: 1.79→2.90 s
* 03_blessing-c1025-q90-420: 1.61→3.14 s
* 03_blessing-c1025-q95-420: 1.66→3.09 s
* 03_blessing-c1040: 1.80→3.25 s
* 03_blessing-c1040-q90-420: 1.76→3.77 s
* 03_blessing-c1040-q95-420: 1.60→2.99 s
* 03_blessing-q75-420: 1.72→3.01 s
* 03_blessing-q85-420: 1.81→3.23 s
* 03_blessing-q90-420: 2.05→3.61 s
* 03_blessing-q90-444: 2.14→3.40 s
* 03_blessing-q95-420: 2.00→3.48 s
* 03_blessing-q95-444: 2.04→3.49 s
* 04_tearing_scroll: 2.68→4.16 s
* 04_tearing_scroll-c1024: 1.64→3.08 s
* 04_tearing_scroll-c1024-q90-420: 1.62→2.96 s
* 04_tearing_scroll-c1024-q95-420: 1.63→2.89 s
* 04_tearing_scroll-c1025: 1.81→3.19 s
* 04_tearing_scroll-c1025-q90-420: 1.61→3.12 s
* 04_tearing_scroll-c1025-q95-420: 1.61→2.87 s
* 04_tearing_scroll-c1040: 1.75→3.36 s
* 04_tearing_scroll-c1040-q90-420: 1.54→3.41 s
* 04_tearing_scroll-c1040-q95-420: 1.59→2.87 s
* 04_tearing_scroll-q75-420: 1.81→2.89 s
* 04_tearing_scroll-q85-420: 1.77→3.17 s
* 04_tearing_scroll-q90-420: 1.91→3.15 s
* 04_tearing_scroll-q90-444: 1.96→3.46 s
* 04_tearing_scroll-q95-444: 1.92→3.47 s
* 05_torch_and_cross: 2.78→4.33 s
* 05_torch_and_cross-c1024: 1.68→3.10 s
* 05_torch_and_cross-c1024-q90-420: 1.75→2.86 s
* 05_torch_and_cross-c1024-q95-420: 1.67→3.06 s
* 05_torch_and_cross-c1025: 1.72→3.69 s
* 05_torch_and_cross-c1025-q90-420: 1.75→3.28 s
* 05_torch_and_cross-c1025-q95-420: 1.63→3.10 s
* 05_torch_and_cross-c1040: 1.71→3.52 s
* 05_torch_and_cross-c1040-q90-420: 1.66→2.95 s
* 05_torch_and_cross-c1040-q95-420: 1.65→3.06 s
* 05_torch_and_cross-q75-420: 1.80→3.07 s
* 05_torch_and_cross-q85-420: 1.77→2.87 s
* 05_torch_and_cross-q90-420: 1.76→3.09 s
* 05_torch_and_cross-q90-444: 2.03→3.16 s
* 05_torch_and_cross-q95-420: 1.85→3.27 s
* 05_torch_and_cross-q95-444: 1.95→3.60 s
* 06_thumbs_up: 2.77→4.33 s
* 06_thumbs_up-c1024: 1.69→3.18 s
* 06_thumbs_up-c1024-q90-420: 1.68→3.21 s
* 06_thumbs_up-c1024-q95-420: 1.72→3.99 s
* 06_thumbs_up-c1025: 1.86→3.36 s
* 06_thumbs_up-c1025-q90-420: 1.90→3.32 s
* 06_thumbs_up-c1025-q95-420: 1.66→3.12 s
* 06_thumbs_up-c1040: 1.86→3.08 s
* 06_thumbs_up-c1040-q90-420: 1.63→2.77 s
* 06_thumbs_up-c1040-q95-420: 1.65→3.37 s
* 06_thumbs_up-q75-420: 1.69→2.96 s
* 06_thumbs_up-q85-420: 1.72→3.00 s
* 06_thumbs_up-q90-420: 1.76→3.28 s
* 06_thumbs_up-q90-444: 1.83→3.79 s
* 06_thumbs_up-q95-420: 1.94→3.50 s
* 06_thumbs_up-q95-444: 1.91→3.73 s
* 06_thumbs_up-x0.9-q90-420: 2.22→3.81 s
* 06_thumbs_up-x1.1: 3.57→5.76 s
* 07_shock: 2.84→4.65 s
* 07_shock-c1024: 1.68→2.74 s
* 07_shock-c1024-q90-420: 1.68→3.26 s
* 07_shock-c1024-q95-420: 1.77→2.81 s
* 07_shock-c1025: 1.83→3.10 s
* 07_shock-c1025-q90-420: 1.68→3.04 s
* 07_shock-c1025-q95-420: 1.65→3.10 s
* 07_shock-c1040: 1.86→3.06 s
* 07_shock-c1040-q90-420: 1.63→2.69 s
* 07_shock-c1040-q95-420: 1.61→2.66 s
* 07_shock-q75-420: 1.71→3.26 s
* 07_shock-q85-420: 1.73→2.89 s
* 07_shock-q90-420: 1.72→3.46 s
* 07_shock-q90-444: 1.82→3.17 s
* 07_shock-q95-420: 1.81→3.34 s
* 07_shock-q95-444: 1.85→3.25 s
* 07_shock-x0.9: 2.77→4.65 s
* 08_facepalm-c1024: 1.74→2.89 s
* 08_facepalm-c1024-q90-420: 1.75→2.95 s
* 08_facepalm-c1024-q95-420: 1.72→3.05 s
* 08_facepalm-c1025: 1.81→3.23 s
* 08_facepalm-c1025-q90-420: 1.65→2.90 s
* 08_facepalm-c1025-q95-420: 1.64→2.77 s
* 08_facepalm-c1040: 1.72→3.06 s
* 08_facepalm-c1040-q90-420: 1.56→2.73 s
* 08_facepalm-c1040-q95-420: 1.55→3.01 s
* 08_facepalm-q75-420: 1.71→3.13 s
* 08_facepalm-q85-420: 1.72→3.18 s
* 08_facepalm-q90-420: 1.78→3.45 s
* 08_facepalm-q90-444: 1.78→3.16 s
* 08_facepalm-q95-420: 1.79→3.44 s
* 08_facepalm-q95-444: 1.84→3.18 s
* 09_thinking: 2.67→4.59 s
* 09_thinking-c1024: 1.66→3.20 s
* 09_thinking-c1024-q90-420: 1.62→3.10 s
* 09_thinking-c1024-q95-420: 1.62→3.15 s
* 09_thinking-c1025: 1.84→3.28 s
* 09_thinking-c1025-q90-420: 1.69→2.84 s
* 09_thinking-c1025-q95-420: 1.78→2.80 s
* 09_thinking-c1040: 1.75→2.97 s
* 09_thinking-c1040-q90-420: 1.54→2.67 s
* 09_thinking-c1040-q95-420: 1.59→2.84 s
* 09_thinking-q75-420: 1.74→3.12 s
* 09_thinking-q85-420: 1.73→3.15 s
* 09_thinking-q90-420: 1.81→2.99 s
* 09_thinking-q90-444: 1.78→3.45 s
* 09_thinking-q95-420: 1.83→3.57 s
* 09_thinking-q95-444: 1.83→3.21 s
* 10_this_is_fine: 2.74→4.93 s
* 10_this_is_fine-c1024: 1.77→3.29 s
* 10_this_is_fine-c1024-q90-420: 1.67→2.98 s
* 10_this_is_fine-c1024-q95-420: 1.66→3.30 s
* 10_this_is_fine-c1025: 1.75→3.42 s
* 10_this_is_fine-c1025-q90-420: 1.61→3.17 s
* 10_this_is_fine-c1025-q95-420: 1.59→3.16 s
* 10_this_is_fine-c1040: 1.71→3.32 s
* 10_this_is_fine-c1040-q90-420: 1.60→3.09 s
* 10_this_is_fine-c1040-q95-420: 1.54→3.07 s
* 10_this_is_fine-q75-420: 1.69→2.75 s
* 10_this_is_fine-q85-420: 1.71→3.22 s
* 10_this_is_fine-q90-420: 1.73→2.93 s
* 10_this_is_fine-q90-444: 1.77→3.26 s
* 10_this_is_fine-q95-420: 1.82→3.30 s
* 10_this_is_fine-q95-444: 1.82→3.02 s
* 10_this_is_fine_alternative: 2.83→4.58 s
* 10_this_is_fine_alternative-c1024: 1.71→2.99 s
* 10_this_is_fine_alternative-c1024-q90-420: 1.66→2.92 s
* 10_this_is_fine_alternative-c1024-q95-420: 1.74→3.04 s
* 10_this_is_fine_alternative-c1025: 1.87→3.10 s
* 10_this_is_fine_alternative-c1025-q90-420: 1.80→3.05 s
* 10_this_is_fine_alternative-c1025-q95-420: 1.75→2.85 s
* 10_this_is_fine_alternative-c1040: 1.99→3.12 s
* 10_this_is_fine_alternative-c1040-q90-420: 1.66→3.78 s
* 10_this_is_fine_alternative-c1040-q95-420: 1.64→2.89 s
* 10_this_is_fine_alternative-q75-420: 1.70→2.93 s
* 10_this_is_fine_alternative-q85-420: 1.71→3.08 s
* 10_this_is_fine_alternative-q90-420: 1.73→3.02 s
* 10_this_is_fine_alternative-q90-444: 1.78→3.33 s
* 10_this_is_fine_alternative-q95-420: 1.81→3.04 s
* 10_this_is_fine_alternative-q95-444: 1.83→2.96 s
* 11_crying: 1.35→2.25 s
* 11_crying-c1024: 1.27→2.27 s
* 11_crying-c1024-q90-420: 1.31→2.22 s
* 11_crying-c1024-q95-420: 1.44→2.44 s
* 11_crying-c1025: 1.06→2.23 s
* 11_crying-c1025-q90-420: 1.35→2.67 s
* 11_crying-c1025-q95-420: 1.06→2.10 s
* 11_crying-c1040: 1.04→1.94 s
* 11_crying-c1040-q90-420: 1.26→2.29 s
* 11_crying-c1040-q95-420: 1.04→1.86 s
* 11_crying-q75-420: 1.29→2.24 s
* 11_crying-q85-420: 1.31→2.19 s
* 11_crying-q90-420: 1.30→2.53 s
* 11_crying-q90-444: 1.11→1.96 s
* 11_crying-q95-420: 1.17→2.07 s
* 11_crying-q95-444: 1.12→2.09 s
* 11_crying-x0.9: 2.36→3.64 s
* 12_laughing-c1024: 1.67→3.17 s
* 12_laughing-c1024-q90-420: 1.63→2.99 s
* 12_laughing-c1024-q95-420: 1.63→2.99 s
* 12_laughing-c1025: 1.79→3.78 s
* 12_laughing-c1025-q90-420: 1.69→2.90 s
* 12_laughing-c1025-q95-420: 1.64→3.00 s
* 12_laughing-c1040: 1.80→3.04 s
* 12_laughing-c1040-q90-420: 1.69→2.89 s
* 12_laughing-c1040-q95-420: 1.65→2.72 s
* 12_laughing-q75-420: 1.75→2.95 s
* 12_laughing-q85-420: 1.77→3.23 s
* 12_laughing-q90-420: 1.78→2.91 s
* 12_laughing-q90-444: 1.77→2.99 s
* 12_laughing-q95-420: 1.82→3.22 s
* 12_laughing-q95-444: 1.83→3.00 s
* 13_angry: 2.89→4.41 s
* 13_angry-c1024: 1.70→3.14 s
* 13_angry-c1024-q90-420: 1.67→3.08 s
* 13_angry-c1024-q95-420: 1.64→3.32 s
* 13_angry-c1025: 1.85→3.30 s
* 13_angry-c1025-q90-420: 1.67→3.05 s
* 13_angry-c1025-q95-420: 1.60→3.05 s
* 13_angry-c1040: 1.74→3.03 s
* 13_angry-c1040-q90-420: 1.66→2.83 s
* 13_angry-c1040-q95-420: 1.55→2.77 s
* 13_angry-q85-420: 1.82→2.91 s
* 13_angry-q90-420: 1.84→3.24 s
* 13_angry-q90-444: 1.83→3.20 s
* 13_angry-q95-420: 1.87→2.97 s
* 13_angry-q95-444: 1.85→3.51 s
* 14_sleeping: 2.64→4.29 s
* 14_sleeping-c1024: 1.74→3.09 s
* 14_sleeping-c1024-q90-420: 1.63→2.82 s
* 14_sleeping-c1024-q95-420: 1.66→3.09 s
* 14_sleeping-c1025: 1.78→3.20 s
* 14_sleeping-c1025-q90-420: 1.60→3.53 s
* 14_sleeping-c1025-q95-420: 1.61→3.16 s
* 14_sleeping-c1040: 1.72→2.99 s
* 14_sleeping-c1040-q90-420: 1.56→2.92 s
* 14_sleeping-c1040-q95-420: 1.55→3.20 s
* 14_sleeping-q75-420: 1.72→2.93 s
* 14_sleeping-q85-420: 1.82→3.32 s
* 14_sleeping-q90-420: 1.80→3.58 s
* 14_sleeping-q90-444: 1.81→3.00 s
* 14_sleeping-q95-420: 1.86→4.13 s
* 14_sleeping-q95-444: 1.92→3.28 s
* 15_coffee: 2.77→4.75 s
* 15_coffee-c1024: 1.68→3.22 s
* 15_coffee-c1024-q90-420: 1.71→3.29 s
* 15_coffee-c1024-q95-420: 1.63→2.82 s
* 15_coffee-c1025: 1.75→3.38 s
* 15_coffee-c1025-q90-420: 1.66→2.74 s
* 15_coffee-c1025-q95-420: 1.61→3.29 s
* 15_coffee-c1040: 1.76→3.28 s
* 15_coffee-c1040-q90-420: 1.60→2.99 s
* 15_coffee-c1040-q95-420: 1.56→3.00 s
* 15_coffee-q75-420: 1.68→2.88 s
* 15_coffee-q85-420: 1.71→2.82 s
* 15_coffee-q90-420: 1.72→2.79 s
* 15_coffee-q90-444: 1.80→2.84 s
* 15_coffee-q95-420: 1.82→3.09 s
* 15_coffee-q95-444: 1.84→3.11 s
* 16_laptop: 2.72→4.68 s
* 16_laptop-c1024: 1.66→2.96 s
* 16_laptop-c1024-q90-420: 1.63→3.32 s
* 16_laptop-c1024-q95-420: 1.64→3.30 s
* 16_laptop-c1025: 1.82→3.24 s
* 16_laptop-c1025-q90-420: 1.63→3.37 s
* 16_laptop-c1025-q95-420: 1.60→3.10 s
* 16_laptop-c1040: 1.71→3.75 s
* 16_laptop-c1040-q90-420: 1.59→2.92 s
* 16_laptop-c1040-q95-420: 1.62→2.81 s
* 16_laptop-q75-420: 1.83→3.14 s
* 16_laptop-q85-420: 2.01→3.05 s
* 16_laptop-q90-420: 1.78→3.18 s
* 16_laptop-q90-444: 1.80→3.35 s
* 16_laptop-q95-420: 1.88→3.58 s
* 16_laptop-q95-444: 1.81→3.07 s
* 17_magnifying_glass: 2.62→4.48 s
* 17_magnifying_glass-c1024: 1.64→3.16 s
* 17_magnifying_glass-c1024-q90-420: 1.63→3.26 s
* 17_magnifying_glass-c1024-q95-420: 1.65→3.92 s
* 17_magnifying_glass-c1025: 1.77→3.38 s
* 17_magnifying_glass-c1025-q90-420: 1.63→2.75 s
* 17_magnifying_glass-c1025-q95-420: 1.66→2.93 s
* 17_magnifying_glass-c1040: 1.73→3.12 s
* 17_magnifying_glass-c1040-q90-420: 1.52→3.48 s
* 17_magnifying_glass-c1040-q95-420: 1.55→2.99 s
* 17_magnifying_glass-q75-420: 1.73→3.41 s
* 17_magnifying_glass-q85-420: 1.85→3.45 s
* 17_magnifying_glass-q90-420: 1.90→3.43 s
* 17_magnifying_glass-q90-444: 2.03→3.30 s
* 17_magnifying_glass-q95-420: 1.98→3.50 s
* 17_magnifying_glass-q95-444: 2.08→3.35 s
* 18_stamp: 2.71→5.02 s
* 18_stamp-c1024: 1.68→3.07 s
* 18_stamp-c1024-q90-420: 1.71→2.61 s
* 18_stamp-c1024-q95-420: 1.81→3.12 s
* 18_stamp-c1025: 1.92→3.26 s
* 18_stamp-c1025-q90-420: 1.63→2.93 s
* 18_stamp-c1025-q95-420: 1.58→3.11 s
* 18_stamp-c1040: 1.88→3.06 s
* 18_stamp-c1040-q95-420: 1.54→2.59 s
* 18_stamp-q75-420: 1.81→3.46 s
* 18_stamp-q85-420: 1.78→3.28 s
* 18_stamp-q90-420: 1.90→3.25 s
* 18_stamp-q90-444: 1.85→3.63 s
* 18_stamp-q95-444: 1.89→3.62 s
* 19_victory: 2.66→4.76 s
* 19_victory-c1024: 1.65→2.64 s
* 19_victory-c1024-q90-420: 1.91→3.03 s
* 19_victory-c1024-q95-420: 1.70→2.70 s
* 19_victory-c1025: 1.78→3.09 s
* 19_victory-c1025-q90-420: 1.66→2.76 s
* 19_victory-c1025-q95-420: 1.71→2.98 s
* 19_victory-c1040-q90-420: 1.61→2.66 s
* 19_victory-c1040-q95-420: 1.55→2.53 s
* 19_victory-q75-420: 1.92→3.17 s
* 19_victory-q85-420: 1.78→3.57 s
* 19_victory-q90-420: 1.80→3.46 s
* 19_victory-q90-444: 1.85→3.77 s
* 19_victory-q95-420: 1.88→3.37 s
* 19_victory-q95-444: 1.91→3.43 s
* 20_waving: 2.72→4.91 s
* 20_waving-c1024-q95-420: 1.75→2.64 s
* 20_waving-c1040: 1.83→2.88 s
* 20_waving-c1040-q90-420: 1.81→2.82 s
* 20_waving-c1040-q95-420: 1.76→3.55 s
* 20_waving-q75-420: 1.75→3.24 s
* 20_waving-q85-420: 1.96→3.14 s
* 20_waving-q90-420: 1.80→3.13 s
* 20_waving-q90-444: 1.86→3.32 s
* 20_waving-q95-420: 1.89→3.47 s
* 20_waving-q95-444: 1.97→3.50 s
* thinking-1040-q95-420: 1.72→2.70 s
* torch-1025: 1.79→3.03 s
* torch-1025-q95-420: 1.64→2.97 s
* torch-1025-q95-444: 1.66→2.79 s
* victory-1025: 1.80→3.01 s
* victory-1025-q95-420: 1.74→2.98 s
* victory-1025-q98-420: 1.76→3.08 s
