# Regression — route detect, lossy, model: **pass**

Before: `34b4c2c9216390bf4d8b943db5743b5b944ea075` (baseline, 2026-10-10, wipemark-cli 0.1.0). After: `bed93b177a460ef2eed724d13747271db5e1502f` (run, 2026-10-10, wipemark-cli 0.1.0). Pillow 12.3.0, libjpeg 6.2, Python 3.13.7.

Corpus gates: **G2** ok (exit3_before 195, exit3_after 195); **G1** ok (files 0, failed 0); **L4/D3** ok (files 0, verified_after 0)
* M3: dropped (D520) — its corpus was gemini-midtone, and Gemini's new outputs carry no visible mark; the model route is M1, M2 and M4
* M4: level A is R5's bench (A5, the matrix) — not checked here

## 1. Class × variant

| class | variant | n | pass | fail | attention |
|---|---|---:|---:|---:|---:|
| frames | c1024-png | 21 | 21 | 0 | 0 |
| frames | c1024-q90-420 | 21 | 21 | 0 | 0 |
| frames | c1024-q95-420 | 21 | 21 | 0 | 0 |
| frames | c1025-png | 21 | 21 | 0 | 0 |
| frames | c1025-q90-420 | 21 | 21 | 0 | 0 |
| frames | c1025-q95-420 | 21 | 21 | 0 | 0 |
| frames | c1040-png | 21 | 21 | 0 | 0 |
| frames | c1040-q90-420 | 21 | 21 | 0 | 0 |
| frames | c1040-q95-420 | 21 | 21 | 0 | 0 |
| recon-jpeg-420 | fixture-q95 | 4 | 4 | 0 | 0 |
| recon-jpeg-420 | fixture-q98 | 1 | 1 | 0 | 0 |
| recon-jpeg-420 | q75 | 21 | 21 | 0 | 0 |
| recon-jpeg-420 | q85 | 21 | 21 | 0 | 0 |
| recon-jpeg-420 | q90 | 21 | 21 | 0 | 0 |
| recon-jpeg-420 | q95 | 21 | 21 | 0 | 0 |
| recon-jpeg-444 | fixture-q95 | 1 | 1 | 0 | 0 |
| recon-jpeg-444 | fixture-q98 | 1 | 1 | 0 | 0 |
| recon-jpeg-444 | q90 | 21 | 21 | 0 | 0 |
| recon-jpeg-444 | q95 | 21 | 21 | 0 | 0 |
| recon-png | fixture-png | 4 | 4 | 0 | 0 |
| recon-png | png | 21 | 21 | 0 | 0 |
| recon-resized | x0.9-png | 21 | 21 | 0 | 0 |
| recon-resized | x0.9-q90-420 | 21 | 21 | 0 | 0 |
| recon-resized | x1.1-png | 21 | 21 | 0 | 0 |
| recon-webp | fixture-q90 | 1 | 1 | 0 | 0 |
| transparent | fixture-png | 1 | 1 | 0 | 0 |
| transparent | fixture-webp-lossy | 1 | 1 | 0 | 0 |

## 2. Exit or `written` changed, by name

None.

## 3. Attention

None.

## 4. Fail

None.

## 5. Measures per class (min / median / p95 / max)

| class | measure | n | before | after |
|---|---|---:|---|---|
| frames | outline | 126 | 0 / 0.02953 / 0.06587 / 0.07702 | 0 / 0.02953 / 0.06587 / 0.07702 |
| frames | step | 126 | -3.67 / 0.06898 / 2.083 / 2.498 | -3.67 / 0.06898 / 2.083 / 2.498 |
| frames | chroma | 126 | 0.1103 / 0.5441 / 1.413 / 1.541 | 0.1103 / 0.5441 / 1.413 / 1.541 |
| frames | texture | 126 | 1.591 / 3.183 / 4.63 / 5.432 | 1.591 / 3.183 / 4.63 / 5.432 |
| frames | out_of_range | 126 | 0 / 0 / 0 / 0 | 0 / 0 / 0 / 0 |
| frames | holes | 126 | 0 / 0 / 0 / 0 | 0 / 0 / 0 / 0 |
| frames | clamped | 126 | 0 / 0 / 39 / 61 | 0 / 0 / 39 / 61 |
| frames | consistency_px | 126 | 0.2439 / 1.899 / 3.134 / 3.217 | 0.2439 / 1.899 / 3.134 / 3.217 |
| recon-jpeg-420 | outline | 69 | 0.0134 / 0.06398 / 0.107 / 0.1857 | 0.0134 / 0.06398 / 0.107 / 0.1857 |
| recon-jpeg-420 | step | 69 | -3.66 / 1.687 / 3.761 / 6.135 | -3.66 / 1.687 / 3.761 / 6.135 |
| recon-jpeg-420 | chroma | 69 | 0.2339 / 1.258 / 1.646 / 1.769 | 0.2339 / 1.258 / 1.646 / 1.769 |
| recon-jpeg-420 | texture | 69 | 1.844 / 3.765 / 8.261 / 12.85 | 1.844 / 3.765 / 8.261 / 12.85 |
| recon-jpeg-420 | out_of_range | 89 | 0 / 0 / 0 / 0 | 0 / 0 / 0 / 0 |
| recon-jpeg-420 | holes | 69 | 0 / 0 / 0 / 0 | 0 / 0 / 0 / 0 |
| recon-jpeg-420 | clamped | 69 | 0 / 15 / 118 / 263 | 0 / 15 / 118 / 263 |
| recon-jpeg-420 | consistency_px | 69 | 0.9422 / 2.861 / 3.027 / 3.087 | 0.9422 / 2.861 / 3.027 / 3.087 |
| recon-jpeg-444 | outline | 44 | 0.004881 / 0.04117 / 0.06363 / 0.07431 | 0.004881 / 0.04117 / 0.06363 / 0.07431 |
| recon-jpeg-444 | step | 44 | -3.463 / 0.2001 / 2.093 / 2.43 | -3.463 / 0.2001 / 2.093 / 2.43 |
| recon-jpeg-444 | chroma | 44 | 0.3669 / 2.091 / 3.855 / 3.884 | 0.3669 / 2.091 / 3.855 / 3.884 |
| recon-jpeg-444 | texture | 44 | 1.628 / 3.233 / 5.419 / 5.938 | 1.628 / 3.233 / 5.419 / 5.938 |
| recon-jpeg-444 | out_of_range | 44 | 0 / 0.0006295 / 0.004358 / 0.004455 | 0 / 0.0006295 / 0.004358 / 0.004455 |
| recon-jpeg-444 | holes | 44 | 0 / 0 / 0 / 0 | 0 / 0 / 0 / 0 |
| recon-jpeg-444 | clamped | 44 | 0 / 15 / 46 / 52 | 0 / 15 / 46 / 52 |
| recon-jpeg-444 | consistency_px | 44 | 2.071 / 3.674 / 4.472 / 4.514 | 2.071 / 3.674 / 4.472 / 4.514 |
| recon-png | outline | 25 | 0 / 0.001893 / 0.0398 / 0.0398 | 0 / 0.001893 / 0.0398 / 0.0398 |
| recon-png | step | 25 | -3.67 / -0.003764 / 0.304 / 13.2 | -3.67 / -0.003764 / 0.304 / 13.2 |
| recon-png | chroma | 25 | 0.1103 / 0.2222 / 0.8452 / 8.208 | 0.1103 / 0.2222 / 0.8452 / 8.208 |
| recon-png | texture | 25 | 1.591 / 1.795 / 2.05 / 10.45 | 1.591 / 1.795 / 2.05 / 10.45 |
| recon-png | out_of_range | 25 | 0 / 0 / 0 / 0 | 0 / 0 / 0 / 0 |
| recon-png | holes | 25 | 0 / 0 / 0 / 0 | 0 / 0 / 0 / 0 |
| recon-png | clamped | 25 | 0 / 0 / 37 / 180 | 0 / 0 / 37 / 180 |
| recon-png | consistency_px | 25 | 0.2439 / 0.2442 / 0.2451 / 0.2453 | 0.2439 / 0.2442 / 0.2451 / 0.2453 |
| recon-resized | outline | 63 | 0.07645 / 0.1138 / 0.122 / 0.1588 | 0.07645 / 0.1138 / 0.122 / 0.1588 |
| recon-resized | step | 63 | -4.69 / -3.188 / -1.388 / -1.238 | -4.69 / -3.188 / -1.388 / -1.238 |
| recon-resized | chroma | 63 | 0.9531 / 1.255 / 1.44 / 1.563 | 0.9531 / 1.255 / 1.44 / 1.563 |
| recon-resized | texture | 63 | 3.823 / 4.883 / 5.724 / 6.398 | 3.823 / 4.883 / 5.724 / 6.398 |
| recon-resized | out_of_range | 63 | 0 / 0 / 0 / 0.0006933 | 0 / 0 / 0 / 0.0006933 |
| recon-resized | holes | 63 | 0 / 0 / 0 / 0 | 0 / 0 / 0 / 0 |
| recon-resized | clamped | 63 | 0 / 10 / 96 / 112 | 0 / 10 / 96 / 112 |
| recon-resized | consistency_px | 63 | 0.2481 / 0.2652 / 3.158 / 3.194 | 0.2481 / 0.2652 / 3.158 / 3.194 |
| recon-webp | outline | 1 | 0.07835 / 0.07835 / 0.07835 / 0.07835 | 0.07835 / 0.07835 / 0.07835 / 0.07835 |
| recon-webp | step | 1 | 0.8926 / 0.8926 / 0.8926 / 0.8926 | 0.8926 / 0.8926 / 0.8926 / 0.8926 |
| recon-webp | chroma | 1 | 6.427 / 6.427 / 6.427 / 6.427 | 6.427 / 6.427 / 6.427 / 6.427 |
| recon-webp | texture | 1 | 10.11 / 10.11 / 10.11 / 10.11 | 10.11 / 10.11 / 10.11 / 10.11 |
| recon-webp | out_of_range | 1 | 0.004455 / 0.004455 / 0.004455 / 0.004455 | 0.004455 / 0.004455 / 0.004455 / 0.004455 |
| recon-webp | holes | 1 | 0 / 0 / 0 / 0 | 0 / 0 / 0 / 0 |
| recon-webp | clamped | 1 | 271 / 271 / 271 / 271 | 271 / 271 / 271 / 271 |
| recon-webp | consistency_px | 1 | 0.2447 / 0.2447 / 0.2447 / 0.2447 | 0.2447 / 0.2447 / 0.2447 / 0.2447 |
| transparent | out_of_range | 1 | 0 / 0 / 0 / 0 | 0 / 0 / 0 / 0 |

## 6. Time per file (slower than ×1.5 from 0.25 s is attention)

Before: median 2.14 s, max 4.63 s, total 978 s. After: median 1.98 s, max 3.95 s, total 874 s.
None slower.
