### Attempts: per model, tactic, intensity and language

| model | tactic | intensity | lang | n | passed | placeholder ✗ | numbers ✗ | length ✗ | script ✗ | identifier ✗ | restore ✗ | no-op | trunc. | divergence med (p10–p90) | word change med (p10–p90) | pairs carried over med | length ratio med (p10–p90) | lang kept / other | preface (passed) | s/attempt |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| qwen38-27b | back_translate | moderate | de | 64 | 86% | 0% | 3% | 2% | 0% | 0% | 0% | 6% | 0% | 0.69 (0.38–0.86) | 0.31 (0.13–0.54) | 0.46 | 1.02 (0.91–1.12) | 78% / 0% | 2% (2%) | 4.06 |
| qwen38-27b | back_translate | moderate | en | 58 | 84% | 2% | 2% | 5% | 0% | 2% | 2% | 7% | 0% | 0.60 (0.33–0.83) | 0.26 (0.13–0.49) | 0.57 | 1.01 (0.91–1.12) | 81% / 0% | 0% (0%) | 4.1 |
| qwen38-27b | back_translate | moderate | ru | 64 | 86% | 0% | 5% | 2% | 2% | 2% | 0% | 5% | 0% | 0.75 (0.54–0.90) | 0.40 (0.23–0.63) | 0.40 | 1.09 (0.97–1.21) | 80% / 0% | 2% (2%) | 4.72 |
| qwen38-27b | humanize | moderate | de | 68 | 81% | 0% | 2% | 10% | 0% | 0% | 0% | 6% | 2% | 0.75 (0.54–0.96) | 0.40 (0.22–0.75) | 0.43 | 0.93 (0.59–1.16) | 75% / 0% | 2% (2%) | 2.22 |
| qwen38-27b | humanize | moderate | en | 60 | 88% | 0% | 5% | 3% | 0% | 0% | 0% | 2% | 0% | 0.83 (0.59–0.95) | 0.50 (0.29–0.69) | 0.33 | 0.90 (0.69–1.05) | 77% / 0% | 0% (0%) | 1.71 |
| qwen38-27b | humanize | moderate | ru | 68 | 85% | 0% | 3% | 4% | 0% | 0% | 0% | 4% | 0% | 0.86 (0.67–0.98) | 0.52 (0.33–0.75) | 0.26 | 0.94 (0.66–1.20) | 73% / 0% | 0% (0%) | 2.22 |
| qwen38-27b | humanize | strong | de | 68 | 90% | 0% | 3% | 6% | 0% | 0% | 0% | 2% | 0% | 0.88 (0.68–0.97) | 0.56 (0.27–0.79) | 0.24 | 0.95 (0.68–1.19) | 81% / 0% | 0% (0%) | 2.24 |
| qwen38-27b | humanize | strong | en | 60 | 93% | 0% | 5% | 2% | 0% | 0% | 0% | 0% | 0% | 0.89 (0.69–0.96) | 0.58 (0.34–0.72) | 0.25 | 0.95 (0.72–1.22) | 80% / 0% | 0% (0%) | 1.77 |
| qwen38-27b | humanize | strong | ru | 68 | 87% | 0% | 6% | 6% | 0% | 0% | 0% | 0% | 0% | 0.90 (0.71–0.99) | 0.60 (0.33–0.79) | 0.20 | 0.96 (0.71–1.23) | 76% / 0% | 0% (0%) | 2.39 |
| qwen38-27b | paraphrase | light | de | 136 | 85% | 0% | 0% | 2% | 0% | 0% | 1% | 13% | 0% | 0.75 (0.10–0.99) | 0.42 (0.03–0.76) | 0.41 | 1.00 (0.89–1.32) | 82% / 0% | 0% (0%) | 2.3 |
| qwen38-27b | paraphrase | light | en | 120 | 90% | 0% | 1% | 3% | 0% | 0% | 0% | 6% | 0% | 0.79 (0.32–1.00) | 0.49 (0.08–0.71) | 0.35 | 1.03 (0.78–1.28) | 83% / 0% | 1% (0%) | 1.77 |
| qwen38-27b | paraphrase | light | ru | 136 | 89% | 0% | 2% | 0% | 0% | 0% | 0% | 8% | 0% | 0.77 (0.22–0.99) | 0.45 (0.07–0.75) | 0.38 | 1.04 (0.97–1.30) | 78% / 0% | 0% (0%) | 2.37 |
| qwen38-27b | paraphrase | moderate | de | 136 | 95% | 0% | 1% | 3% | 0% | 0% | 0% | 2% | 0% | 0.89 (0.71–1.00) | 0.68 (0.44–0.84) | 0.19 | 1.13 (0.97–1.53) | 86% / 0% | 1% (0%) | 2.51 |
| qwen38-27b | paraphrase | moderate | en | 120 | 94% | 0% | 2% | 1% | 0% | 0% | 0% | 2% | 0% | 0.86 (0.62–0.97) | 0.61 (0.39–0.77) | 0.23 | 1.16 (0.93–1.44) | 90% / 0% | 0% (0%) | 1.86 |
| qwen38-27b | paraphrase | moderate | ru | 136 | 90% | 1% | 4% | 4% | 1% | 0% | 1% | 1% | 0% | 0.95 (0.71–1.00) | 0.73 (0.46–0.89) | 0.10 | 1.19 (0.97–1.50) | 79% / 0% | 0% (0%) | 2.48 |
| qwen38-27b | paraphrase | strong | de | 136 | 92% | 0% | 2% | 5% | 0% | 0% | 0% | 2% | 0% | 0.92 (0.76–1.00) | 0.71 (0.50–0.86) | 0.14 | 1.16 (0.95–1.63) | 89% / 0% | 0% (0%) | 2.54 |
| qwen38-27b | paraphrase | strong | en | 120 | 93% | 0% | 2% | 3% | 0% | 0% | 0% | 2% | 0% | 0.89 (0.64–1.00) | 0.65 (0.33–0.77) | 0.20 | 1.13 (0.89–1.49) | 88% / 0% | 0% (0%) | 1.85 |
| qwen38-27b | paraphrase | strong | ru | 136 | 87% | 0% | 2% | 7% | 0% | 0% | 0% | 3% | 0% | 0.95 (0.68–1.00) | 0.72 (0.38–0.87) | 0.11 | 1.19 (0.93–1.58) | 79% / 0% | 0% (0%) | 2.49 |
| qwen38-27b | structural | moderate | de | 34 | 97% | 0% | 0% | 3% | 0% | 0% | 0% | 0% | 0% | 0.75 (0.56–0.90) | 0.41 (0.22–0.65) | 0.44 | 1.12 (0.78–1.29) | 79% / 0% | 0% (0%) | 4.61 |
| qwen38-27b | structural | moderate | en | 30 | 80% | 3% | 0% | 17% | 0% | 0% | 7% | 3% | 0% | 0.75 (0.41–0.89) | 0.38 (0.10–0.68) | 0.43 | 1.06 (0.86–1.68) | 83% / 0% | 0% (0%) | 3.83 |
| qwen38-27b | structural | moderate | ru | 34 | 82% | 0% | 0% | 15% | 0% | 0% | 0% | 3% | 0% | 0.75 (0.40–0.94) | 0.47 (0.17–0.72) | 0.40 | 1.16 (0.98–1.69) | 85% / 0% | 0% (0%) | 5.31 |
| qwen38-27b+dflash2 | back_translate | moderate | de | 64 | 86% | 0% | 3% | 2% | 0% | 0% | 0% | 6% | 0% | 0.71 (0.38–0.85) | 0.31 (0.13–0.56) | 0.46 | 1.04 (0.91–1.12) | 78% / 0% | 2% (2%) | 7.44 |
| qwen38-27b+dflash2 | back_translate | moderate | en | 58 | 84% | 2% | 2% | 5% | 0% | 0% | 2% | 7% | 0% | 0.61 (0.33–0.85) | 0.23 (0.12–0.49) | 0.57 | 1.00 (0.86–1.12) | 79% / 0% | 0% (0%) | 7.51 |
| qwen38-27b+dflash2 | back_translate | moderate | ru | 64 | 88% | 0% | 5% | 2% | 0% | 2% | 0% | 5% | 0% | 0.75 (0.54–0.91) | 0.40 (0.23–0.60) | 0.40 | 1.09 (0.97–1.21) | 80% / 0% | 2% (2%) | 10.75 |
| qwen38-27b+dflash2 | humanize | moderate | de | 68 | 76% | 0% | 2% | 15% | 0% | 0% | 0% | 6% | 2% | 0.75 (0.54–0.97) | 0.40 (0.22–0.75) | 0.41 | 0.93 (0.56–1.16) | 75% / 0% | 2% (2%) | 3.94 |
| qwen38-27b+dflash2 | humanize | moderate | en | 60 | 88% | 0% | 5% | 3% | 0% | 0% | 0% | 2% | 0% | 0.83 (0.59–0.95) | 0.50 (0.29–0.71) | 0.33 | 0.90 (0.69–1.07) | 77% / 0% | 0% (0%) | 3.03 |
| qwen38-27b+dflash2 | humanize | moderate | ru | 68 | 84% | 0% | 4% | 6% | 0% | 0% | 0% | 4% | 0% | 0.87 (0.67–1.00) | 0.53 (0.33–0.73) | 0.26 | 0.93 (0.66–1.20) | 72% / 0% | 0% (0%) | 4.73 |
| qwen38-27b+dflash2 | humanize | strong | de | 68 | 90% | 0% | 3% | 6% | 0% | 0% | 0% | 2% | 0% | 0.88 (0.69–0.97) | 0.55 (0.31–0.79) | 0.25 | 0.93 (0.68–1.19) | 81% / 0% | 0% (0%) | 4.89 |
| qwen38-27b+dflash2 | humanize | strong | en | 60 | 93% | 0% | 5% | 2% | 0% | 0% | 0% | 0% | 0% | 0.88 (0.69–0.96) | 0.58 (0.32–0.72) | 0.24 | 0.95 (0.72–1.22) | 83% / 0% | 0% (0%) | 3.36 |
| qwen38-27b+dflash2 | humanize | strong | ru | 68 | 88% | 0% | 6% | 4% | 0% | 0% | 0% | 0% | 0% | 0.90 (0.71–0.99) | 0.60 (0.33–0.79) | 0.20 | 0.94 (0.70–1.25) | 78% / 0% | 0% (0%) | 5.57 |
| qwen38-27b+dflash2 | paraphrase | light | de | 136 | 85% | 0% | 0% | 2% | 0% | 0% | 1% | 13% | 0% | 0.75 (0.10–0.99) | 0.43 (0.03–0.76) | 0.41 | 1.01 (0.89–1.32) | 82% / 0% | 0% (0%) | 4.2 |
| qwen38-27b+dflash2 | paraphrase | light | en | 120 | 91% | 0% | 1% | 2% | 0% | 0% | 0% | 6% | 0% | 0.79 (0.32–0.95) | 0.44 (0.07–0.67) | 0.37 | 1.02 (0.78–1.26) | 82% / 0% | 0% (0%) | 2.84 |
| qwen38-27b+dflash2 | paraphrase | light | ru | 136 | 90% | 0% | 2% | 0% | 0% | 0% | 0% | 7% | 0% | 0.79 (0.24–0.99) | 0.45 (0.07–0.75) | 0.35 | 1.05 (0.96–1.30) | 78% / 0% | 0% (0%) | 4.66 |
| qwen38-27b+dflash2 | paraphrase | moderate | de | 136 | 96% | 0% | 1% | 2% | 0% | 0% | 0% | 2% | 0% | 0.89 (0.71–1.00) | 0.68 (0.43–0.83) | 0.18 | 1.12 (0.94–1.52) | 87% / 0% | 1% (0%) | 6.0 |
| qwen38-27b+dflash2 | paraphrase | moderate | en | 120 | 94% | 0% | 2% | 1% | 0% | 0% | 0% | 2% | 0% | 0.87 (0.62–1.00) | 0.61 (0.42–0.78) | 0.23 | 1.16 (0.93–1.44) | 91% / 0% | 0% (0%) | 3.56 |
| qwen38-27b+dflash2 | paraphrase | moderate | ru | 136 | 91% | 1% | 4% | 4% | 1% | 0% | 1% | 1% | 0% | 0.94 (0.71–1.00) | 0.72 (0.46–0.88) | 0.11 | 1.19 (0.97–1.50) | 79% / 0% | 0% (0%) | 6.17 |
| qwen38-27b+dflash2 | paraphrase | strong | de | 136 | 91% | 0% | 3% | 6% | 0% | 0% | 0% | 2% | 0% | 0.93 (0.75–1.00) | 0.71 (0.50–0.86) | 0.12 | 1.17 (0.95–1.63) | 88% / 0% | 0% (0%) | 6.6 |
| qwen38-27b+dflash2 | paraphrase | strong | en | 120 | 93% | 0% | 2% | 3% | 0% | 0% | 0% | 2% | 0% | 0.89 (0.66–1.00) | 0.65 (0.33–0.77) | 0.20 | 1.13 (0.88–1.45) | 87% / 0% | 0% (0%) | 3.62 |
| qwen38-27b+dflash2 | paraphrase | strong | ru | 136 | 87% | 0% | 2% | 5% | 0% | 0% | 0% | 3% | 0% | 0.95 (0.67–1.00) | 0.72 (0.38–0.87) | 0.10 | 1.19 (0.93–1.53) | 79% / 0% | 0% (0%) | 6.12 |
| qwen38-27b+dflash2 | structural | moderate | de | 34 | 97% | 0% | 0% | 3% | 0% | 0% | 0% | 0% | 0% | 0.76 (0.56–0.88) | 0.44 (0.22–0.62) | 0.39 | 1.11 (0.73–1.29) | 79% / 0% | 0% (0%) | 7.69 |
| qwen38-27b+dflash2 | structural | moderate | en | 30 | 80% | 3% | 0% | 17% | 0% | 0% | 7% | 3% | 0% | 0.75 (0.41–0.89) | 0.38 (0.11–0.68) | 0.43 | 1.08 (0.86–1.68) | 83% / 0% | 0% (0%) | 5.46 |
| qwen38-27b+dflash2 | structural | moderate | ru | 34 | 82% | 0% | 0% | 15% | 0% | 0% | 0% | 3% | 0% | 0.77 (0.40–0.95) | 0.50 (0.17–0.73) | 0.37 | 1.16 (0.98–1.62) | 85% / 0% | 0% (0%) | 9.4 |

### Speed per model (every attempt)

| model | attempts | tokens out / call | s / attempt | tokens/s (incl. prompt) |
|---|---|---|---|---|
| qwen38-27b | 1852 | 82.0 | 2.55 | 31.9 |
| qwen38-27b+dflash2 | 1852 | 81.0 | 5.3 | 15.3 |

### Speed with and without a draft (every call)

| model | draft | calls | tokens out | s / call | tokens/s (incl. prompt) | accepted / step | tokens / step |
|---|---|---|---|---|---|---|---|
| qwen38-27b | none | 2135 | 151029 | 2.20 | 32.2 | – | – |
| qwen38-27b+dflash2 | 1a25c56858e1 | 2135 | 150383 | 4.58 | 15.4 | 1.63 | 2.63 |

### `paraphrase`, moderate: pass rate by kind of text

| model | prose-pd | machine | markdown | numbers | injection | short | quote |
|---|---|---|---|---|---|---|---|
| qwen38-27b | 44 of 44 | 63 of 64 | 120 of 132 | 18 of 24 | 32 of 36 | 27 of 28 | 31 of 32 |
| qwen38-27b+dflash2 | 44 of 44 | 63 of 64 | 120 of 132 | 18 of 24 | 34 of 36 | 27 of 28 | 31 of 32 |

### What rejected a candidate (`paraphrase`, all intensities; the loop's first reason)

| model | reason | n | share of attempts |
|---|---|---|---|
| qwen38-27b | no-op | 49 | 4% |
| qwen38-27b | guard length-drift (length-drift) | 34 | 3% |
| qwen38-27b | guard numbers (number-missing) | 19 | 2% |
| qwen38-27b | language | 9 | 1% |
| qwen38-27b | guard placeholder (placeholder-missing) | 1 | 0% |
| qwen38-27b+dflash2 | no-op | 47 | 4% |
| qwen38-27b+dflash2 | guard length-drift (length-drift) | 28 | 2% |
| qwen38-27b+dflash2 | guard numbers (number-missing) | 21 | 2% |
| qwen38-27b+dflash2 | language | 11 | 1% |
| qwen38-27b+dflash2 | guard placeholder (placeholder-missing) | 1 | 0% |

### Placeholders `⟦n⟧` (attempts on chunks that carry one)

| model | lang | attempts | every placeholder exactly once | placeholders kept / expected |
|---|---|---|---|---|
| qwen38-27b | en | 133 | 98% | 99% |
| qwen38-27b | ru | 133 | 99% | 99% |
| qwen38-27b | de | 133 | 100% | 100% |
| qwen38-27b+dflash2 | en | 133 | 98% | 99% |
| qwen38-27b+dflash2 | ru | 133 | 99% | 99% |
| qwen38-27b+dflash2 | de | 133 | 100% | 100% |

### Planted instructions (3 items per language)

| model | tactic | attempts | obeyed | obeyed **and** passed every check |
|---|---|---|---|---|
| qwen38-27b | back_translate | 18 | 0 (0%) | 0 |
| qwen38-27b | humanize | 36 | 2 (6%) | 0 |
| qwen38-27b | paraphrase | 108 | 4 (4%) | 0 |
| qwen38-27b | structural | 9 | 0 (0%) | 0 |
| qwen38-27b+dflash2 | back_translate | 18 | 0 (0%) | 0 |
| qwen38-27b+dflash2 | humanize | 36 | 2 (6%) | 0 |
| qwen38-27b+dflash2 | paraphrase | 108 | 2 (2%) | 0 |
| qwen38-27b+dflash2 | structural | 9 | 0 (0%) | 0 |

### Selection policies, GPU 2 × 2 (D61) — every language

| model | tactic | intensity | policy | chunks | rewritten | rewritten, by words | divergence med (p10–p90) | word change med (p10–p90) | pairs carried over, mean (all chunks) | pairs carried over, by words | judged CHANGED | calls / chunk | s / chunk |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| qwen38-27b | paraphrase | light | min ≥ 0.05 (E4-3) | 98 | 100% | 100% | 0.67 (0.24–0.91) | 0.38 (0.05–0.67) | 52% | 53% | – | 2.0 | 4.4 |
| qwen38-27b | paraphrase | light | max ≥ 0.05 | 98 | 100% | 100% | 0.89 (0.63–0.99) | 0.59 (0.35–0.80) | 26% | 25% | – | 2.0 | 4.4 |
| qwen38-27b | paraphrase | light | max ≥ 0.2 (E4-7) | 98 | 100% | 100% | 0.89 (0.63–0.99) | 0.59 (0.35–0.80) | 26% | 25% | – | 2.0 | 4.4 |
| qwen38-27b | paraphrase | light | min ≥ 0.2 | 98 | 100% | 100% | 0.72 (0.33–0.92) | 0.43 (0.12–0.71) | 46% | 46% | – | 2.0 | 4.4 |
| qwen38-27b | paraphrase | light | min ≥ 0.3 | 98 | 100% | 100% | 0.73 (0.36–0.93) | 0.44 (0.15–0.71) | 44% | 42% | – | 2.0 | 4.4 |
| qwen38-27b | paraphrase | light | min ≥ 0.4 | 98 | 100% | 100% | 0.79 (0.48–0.95) | 0.49 (0.21–0.75) | 37% | 37% | – | 2.04 | 4.4 |
| qwen38-27b | paraphrase | light | min ≥ 0.6 | 98 | 95% | 95% | 0.83 (0.63–0.99) | 0.52 (0.30–0.76) | 33% | 34% | – | 2.18 | 4.7 |
| qwen38-27b | paraphrase | light | min ≥ 0.75 | 98 | 88% | 90% | 0.87 (0.78–0.99) | 0.57 (0.43–0.80) | 31% | 30% | – | 2.33 | 5.0 |
| qwen38-27b | paraphrase | moderate | min ≥ 0.05 (E4-3) | 98 | 100% | 100% | 0.87 (0.64–0.97) | 0.63 (0.39–0.82) | 27% | 26% | – | 2.04 | 4.8 |
| qwen38-27b | paraphrase | moderate | max ≥ 0.05 | 98 | 100% | 100% | 0.94 (0.79–1.00) | 0.70 (0.51–0.86) | 15% | 15% | – | 2.04 | 4.8 |
| qwen38-27b | paraphrase | moderate | max ≥ 0.2 (E4-7) | 98 | 100% | 100% | 0.94 (0.79–1.00) | 0.70 (0.51–0.86) | 15% | 15% | – | 2.04 | 4.8 |
| qwen38-27b | paraphrase | moderate | min ≥ 0.2 | 98 | 100% | 100% | 0.88 (0.66–0.97) | 0.63 (0.40–0.82) | 26% | 24% | – | 2.04 | 4.8 |
| qwen38-27b | paraphrase | moderate | min ≥ 0.3 | 98 | 100% | 100% | 0.88 (0.66–0.97) | 0.63 (0.40–0.82) | 26% | 24% | – | 2.04 | 4.8 |
| qwen38-27b | paraphrase | moderate | min ≥ 0.4 | 98 | 100% | 100% | 0.88 (0.67–0.97) | 0.63 (0.43–0.82) | 25% | 23% | – | 2.04 | 4.8 |
| qwen38-27b | paraphrase | moderate | min ≥ 0.6 | 98 | 100% | 100% | 0.88 (0.69–0.97) | 0.64 (0.43–0.82) | 24% | 22% | – | 2.06 | 4.8 |
| qwen38-27b | paraphrase | moderate | min ≥ 0.75 | 98 | 96% | 96% | 0.90 (0.79–1.00) | 0.67 (0.51–0.85) | 21% | 21% | – | 2.18 | 5.1 |
| qwen38-27b | paraphrase | strong | min ≥ 0.05 (E4-3) | 98 | 100% | 100% | 0.88 (0.66–0.99) | 0.64 (0.34–0.81) | 25% | 25% | – | 2.02 | 4.7 |
| qwen38-27b | paraphrase | strong | max ≥ 0.05 | 98 | 100% | 100% | 0.94 (0.80–1.00) | 0.71 (0.54–0.85) | 14% | 14% | – | 2.02 | 4.7 |
| qwen38-27b | paraphrase | strong | max ≥ 0.2 (E4-7) | 98 | 100% | 100% | 0.94 (0.80–1.00) | 0.71 (0.54–0.85) | 14% | 14% | – | 2.02 | 4.7 |
| qwen38-27b | paraphrase | strong | min ≥ 0.2 | 98 | 100% | 100% | 0.88 (0.68–0.99) | 0.64 (0.40–0.81) | 24% | 23% | – | 2.02 | 4.7 |
| qwen38-27b | paraphrase | strong | min ≥ 0.3 | 98 | 100% | 100% | 0.88 (0.69–0.99) | 0.64 (0.40–0.81) | 23% | 22% | – | 2.02 | 4.7 |
| qwen38-27b | paraphrase | strong | min ≥ 0.4 | 98 | 100% | 100% | 0.88 (0.69–0.99) | 0.64 (0.40–0.81) | 23% | 22% | – | 2.02 | 4.7 |
| qwen38-27b | paraphrase | strong | min ≥ 0.6 | 98 | 100% | 100% | 0.89 (0.70–0.99) | 0.64 (0.43–0.81) | 22% | 21% | – | 2.04 | 4.7 |
| qwen38-27b | paraphrase | strong | min ≥ 0.75 | 98 | 97% | 98% | 0.90 (0.78–1.00) | 0.67 (0.46–0.81) | 20% | 19% | – | 2.12 | 4.8 |
| qwen38-27b+dflash2 | paraphrase | light | min ≥ 0.05 (E4-3) | 98 | 100% | 100% | 0.70 (0.28–0.92) | 0.41 (0.05–0.67) | 50% | 50% | – | 2.0 | 8.6 |
| qwen38-27b+dflash2 | paraphrase | light | max ≥ 0.05 | 98 | 100% | 100% | 0.89 (0.70–1.00) | 0.60 (0.38–0.80) | 24% | 24% | – | 2.0 | 8.6 |
| qwen38-27b+dflash2 | paraphrase | light | max ≥ 0.2 (E4-7) | 98 | 100% | 100% | 0.89 (0.70–1.00) | 0.60 (0.38–0.80) | 24% | 24% | – | 2.0 | 8.6 |
| qwen38-27b+dflash2 | paraphrase | light | min ≥ 0.2 | 98 | 100% | 100% | 0.73 (0.33–0.93) | 0.44 (0.12–0.71) | 45% | 45% | – | 2.0 | 8.6 |
| qwen38-27b+dflash2 | paraphrase | light | min ≥ 0.3 | 98 | 100% | 100% | 0.77 (0.36–0.93) | 0.45 (0.15–0.71) | 42% | 40% | – | 2.0 | 8.6 |
| qwen38-27b+dflash2 | paraphrase | light | min ≥ 0.4 | 98 | 100% | 100% | 0.80 (0.50–0.95) | 0.50 (0.21–0.75) | 35% | 35% | – | 2.02 | 8.6 |
| qwen38-27b+dflash2 | paraphrase | light | min ≥ 0.6 | 98 | 96% | 96% | 0.83 (0.63–0.99) | 0.53 (0.30–0.76) | 32% | 32% | – | 2.14 | 8.9 |
| qwen38-27b+dflash2 | paraphrase | light | min ≥ 0.75 | 98 | 89% | 91% | 0.87 (0.79–0.99) | 0.58 (0.43–0.77) | 31% | 29% | – | 2.29 | 9.3 |
| qwen38-27b+dflash2 | paraphrase | moderate | min ≥ 0.05 (E4-3) | 98 | 100% | 100% | 0.88 (0.65–0.95) | 0.63 (0.39–0.80) | 28% | 26% | – | 2.02 | 11.0 |
| qwen38-27b+dflash2 | paraphrase | moderate | max ≥ 0.05 | 98 | 100% | 100% | 0.94 (0.77–1.00) | 0.70 (0.50–0.86) | 15% | 16% | – | 2.02 | 11.0 |
| qwen38-27b+dflash2 | paraphrase | moderate | max ≥ 0.2 (E4-7) | 98 | 100% | 100% | 0.94 (0.77–1.00) | 0.70 (0.50–0.86) | 15% | 16% | – | 2.02 | 11.0 |
| qwen38-27b+dflash2 | paraphrase | moderate | min ≥ 0.2 | 98 | 100% | 100% | 0.88 (0.66–0.95) | 0.64 (0.40–0.80) | 26% | 25% | – | 2.02 | 11.0 |
| qwen38-27b+dflash2 | paraphrase | moderate | min ≥ 0.3 | 98 | 100% | 100% | 0.88 (0.66–0.95) | 0.64 (0.40–0.80) | 26% | 25% | – | 2.02 | 11.0 |
| qwen38-27b+dflash2 | paraphrase | moderate | min ≥ 0.4 | 98 | 100% | 100% | 0.88 (0.67–0.95) | 0.64 (0.41–0.80) | 25% | 24% | – | 2.02 | 11.0 |
| qwen38-27b+dflash2 | paraphrase | moderate | min ≥ 0.6 | 98 | 100% | 100% | 0.88 (0.69–0.95) | 0.64 (0.41–0.80) | 24% | 23% | – | 2.04 | 11.1 |
| qwen38-27b+dflash2 | paraphrase | moderate | min ≥ 0.75 | 98 | 96% | 96% | 0.91 (0.78–0.99) | 0.67 (0.50–0.82) | 22% | 21% | – | 2.16 | 11.5 |
| qwen38-27b+dflash2 | paraphrase | strong | min ≥ 0.05 (E4-3) | 98 | 99% | 98% | 0.89 (0.66–0.99) | 0.66 (0.33–0.82) | 25% | 25% | – | 2.04 | 11.5 |
| qwen38-27b+dflash2 | paraphrase | strong | max ≥ 0.05 | 98 | 99% | 98% | 0.95 (0.80–1.00) | 0.71 (0.56–0.84) | 15% | 15% | – | 2.04 | 11.5 |
| qwen38-27b+dflash2 | paraphrase | strong | max ≥ 0.2 (E4-7) | 98 | 99% | 98% | 0.95 (0.80–1.00) | 0.71 (0.56–0.84) | 14% | 15% | – | 2.06 | 11.6 |
| qwen38-27b+dflash2 | paraphrase | strong | min ≥ 0.2 | 98 | 99% | 98% | 0.89 (0.68–0.99) | 0.66 (0.40–0.82) | 24% | 23% | – | 2.06 | 11.6 |
| qwen38-27b+dflash2 | paraphrase | strong | min ≥ 0.3 | 98 | 99% | 98% | 0.89 (0.70–0.99) | 0.66 (0.40–0.82) | 23% | 22% | – | 2.06 | 11.6 |
| qwen38-27b+dflash2 | paraphrase | strong | min ≥ 0.4 | 98 | 99% | 98% | 0.89 (0.70–0.99) | 0.66 (0.40–0.82) | 23% | 22% | – | 2.06 | 11.6 |
| qwen38-27b+dflash2 | paraphrase | strong | min ≥ 0.6 | 98 | 99% | 98% | 0.89 (0.72–0.99) | 0.66 (0.43–0.82) | 22% | 21% | – | 2.08 | 11.8 |
| qwen38-27b+dflash2 | paraphrase | strong | min ≥ 0.75 | 98 | 96% | 96% | 0.91 (0.80–1.00) | 0.67 (0.50–0.83) | 21% | 20% | – | 2.14 | 11.8 |

### Selection policies, CPU 1 × 2 (D61) — every language

| model | tactic | intensity | policy | chunks | rewritten | rewritten, by words | divergence med (p10–p90) | word change med (p10–p90) | pairs carried over, mean (all chunks) | pairs carried over, by words | judged CHANGED | calls / chunk | s / chunk |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| qwen38-27b | paraphrase | light | min ≥ 0.05 (E4-3) | 98 | 100% | 100% | 0.78 (0.30–0.97) | 0.47 (0.07–0.75) | 43% | 45% | – | 1.04 | 2.3 |
| qwen38-27b | paraphrase | light | max ≥ 0.05 | 98 | 100% | 100% | 0.78 (0.30–0.97) | 0.47 (0.07–0.75) | 43% | 45% | – | 1.04 | 2.3 |
| qwen38-27b | paraphrase | light | max ≥ 0.2 (E4-7) | 98 | 100% | 100% | 0.81 (0.34–0.99) | 0.48 (0.12–0.76) | 39% | 40% | – | 1.1 | 2.4 |
| qwen38-27b | paraphrase | light | min ≥ 0.2 | 98 | 100% | 100% | 0.81 (0.34–0.99) | 0.48 (0.12–0.76) | 39% | 40% | – | 1.1 | 2.4 |
| qwen38-27b | paraphrase | light | min ≥ 0.3 | 98 | 100% | 100% | 0.82 (0.41–0.99) | 0.50 (0.15–0.76) | 37% | 37% | – | 1.14 | 2.6 |
| qwen38-27b | paraphrase | light | min ≥ 0.4 | 98 | 98% | 98% | 0.83 (0.51–0.99) | 0.51 (0.26–0.77) | 33% | 33% | – | 1.23 | 2.7 |
| qwen38-27b | paraphrase | light | min ≥ 0.6 | 98 | 91% | 92% | 0.86 (0.70–0.99) | 0.55 (0.33–0.80) | 32% | 32% | – | 1.33 | 2.9 |
| qwen38-27b | paraphrase | light | min ≥ 0.75 | 98 | 84% | 86% | 0.89 (0.78–1.00) | 0.60 (0.43–0.80) | 33% | 32% | – | 1.48 | 3.3 |
| qwen38-27b | paraphrase | moderate | min ≥ 0.05 (E4-3) | 98 | 98% | 98% | 0.92 (0.66–1.00) | 0.68 (0.40–0.85) | 23% | 23% | – | 1.06 | 2.4 |
| qwen38-27b | paraphrase | moderate | max ≥ 0.05 | 98 | 98% | 98% | 0.92 (0.66–1.00) | 0.68 (0.40–0.85) | 23% | 23% | – | 1.06 | 2.4 |
| qwen38-27b | paraphrase | moderate | max ≥ 0.2 (E4-7) | 98 | 98% | 98% | 0.92 (0.67–1.00) | 0.68 (0.43–0.85) | 22% | 22% | – | 1.09 | 2.5 |
| qwen38-27b | paraphrase | moderate | min ≥ 0.2 | 98 | 98% | 98% | 0.92 (0.67–1.00) | 0.68 (0.43–0.85) | 22% | 22% | – | 1.09 | 2.5 |
| qwen38-27b | paraphrase | moderate | min ≥ 0.3 | 98 | 98% | 98% | 0.92 (0.67–1.00) | 0.68 (0.43–0.85) | 22% | 22% | – | 1.09 | 2.5 |
| qwen38-27b | paraphrase | moderate | min ≥ 0.4 | 98 | 98% | 98% | 0.92 (0.69–1.00) | 0.68 (0.49–0.85) | 21% | 20% | – | 1.11 | 2.6 |
| qwen38-27b | paraphrase | moderate | min ≥ 0.6 | 98 | 97% | 96% | 0.92 (0.69–1.00) | 0.68 (0.49–0.85) | 21% | 21% | – | 1.12 | 2.6 |
| qwen38-27b | paraphrase | moderate | min ≥ 0.75 | 98 | 91% | 91% | 0.92 (0.81–1.00) | 0.69 (0.52–0.86) | 23% | 22% | – | 1.22 | 2.8 |
| qwen38-27b | paraphrase | strong | min ≥ 0.05 (E4-3) | 98 | 99% | 99% | 0.92 (0.74–1.00) | 0.67 (0.44–0.81) | 22% | 21% | – | 1.06 | 2.5 |
| qwen38-27b | paraphrase | strong | max ≥ 0.05 | 98 | 99% | 99% | 0.92 (0.74–1.00) | 0.67 (0.44–0.81) | 22% | 21% | – | 1.06 | 2.5 |
| qwen38-27b | paraphrase | strong | max ≥ 0.2 (E4-7) | 98 | 99% | 99% | 0.92 (0.75–1.00) | 0.67 (0.46–0.81) | 20% | 20% | – | 1.09 | 2.6 |
| qwen38-27b | paraphrase | strong | min ≥ 0.2 | 98 | 99% | 99% | 0.92 (0.75–1.00) | 0.67 (0.46–0.81) | 20% | 20% | – | 1.09 | 2.6 |
| qwen38-27b | paraphrase | strong | min ≥ 0.3 | 98 | 99% | 99% | 0.92 (0.75–1.00) | 0.67 (0.46–0.81) | 20% | 20% | – | 1.09 | 2.6 |
| qwen38-27b | paraphrase | strong | min ≥ 0.4 | 98 | 99% | 99% | 0.92 (0.75–1.00) | 0.67 (0.46–0.81) | 20% | 20% | – | 1.09 | 2.6 |
| qwen38-27b | paraphrase | strong | min ≥ 0.6 | 98 | 98% | 98% | 0.92 (0.76–1.00) | 0.67 (0.51–0.82) | 20% | 19% | – | 1.11 | 2.6 |
| qwen38-27b | paraphrase | strong | min ≥ 0.75 | 98 | 94% | 95% | 0.92 (0.80–1.00) | 0.67 (0.53–0.82) | 21% | 20% | – | 1.17 | 2.7 |
| qwen38-27b | humanize | moderate | min ≥ 0.05 (E4-3) | 98 | 97% | 98% | 0.79 (0.54–0.96) | 0.47 (0.25–0.74) | 38% | 38% | – | 1.1 | 2.2 |
| qwen38-27b | humanize | moderate | max ≥ 0.05 | 98 | 97% | 98% | 0.79 (0.54–0.96) | 0.47 (0.25–0.74) | 38% | 38% | – | 1.1 | 2.2 |
| qwen38-27b | humanize | moderate | max ≥ 0.2 (E4-7) | 98 | 96% | 97% | 0.80 (0.63–0.96) | 0.48 (0.29–0.74) | 37% | 36% | – | 1.13 | 2.3 |
| qwen38-27b | humanize | moderate | min ≥ 0.2 | 98 | 96% | 97% | 0.80 (0.63–0.96) | 0.48 (0.29–0.74) | 37% | 36% | – | 1.13 | 2.3 |
| qwen38-27b | humanize | moderate | min ≥ 0.3 | 98 | 96% | 97% | 0.80 (0.63–0.96) | 0.48 (0.29–0.74) | 37% | 36% | – | 1.13 | 2.3 |
| qwen38-27b | humanize | moderate | min ≥ 0.4 | 98 | 94% | 97% | 0.80 (0.66–0.96) | 0.48 (0.29–0.74) | 37% | 36% | – | 1.17 | 2.4 |
| qwen38-27b | humanize | moderate | min ≥ 0.6 | 98 | 93% | 95% | 0.81 (0.67–0.96) | 0.49 (0.30–0.74) | 36% | 35% | – | 1.21 | 2.4 |
| qwen38-27b | humanize | moderate | min ≥ 0.75 | 98 | 74% | 79% | 0.87 (0.76–0.97) | 0.52 (0.34–0.77) | 44% | 43% | – | 1.42 | 2.9 |
| qwen38-27b | humanize | strong | min ≥ 0.05 (E4-3) | 98 | 95% | 94% | 0.88 (0.67–0.97) | 0.56 (0.31–0.75) | 30% | 31% | – | 1.07 | 2.3 |
| qwen38-27b | humanize | strong | max ≥ 0.05 | 98 | 95% | 94% | 0.88 (0.67–0.97) | 0.56 (0.31–0.75) | 30% | 31% | – | 1.07 | 2.3 |
| qwen38-27b | humanize | strong | max ≥ 0.2 (E4-7) | 98 | 95% | 94% | 0.88 (0.67–0.97) | 0.56 (0.31–0.75) | 30% | 31% | – | 1.07 | 2.3 |
| qwen38-27b | humanize | strong | min ≥ 0.2 | 98 | 95% | 94% | 0.88 (0.67–0.97) | 0.56 (0.31–0.75) | 30% | 31% | – | 1.07 | 2.3 |
| qwen38-27b | humanize | strong | min ≥ 0.3 | 98 | 95% | 94% | 0.88 (0.68–0.97) | 0.56 (0.31–0.75) | 30% | 30% | – | 1.08 | 2.3 |
| qwen38-27b | humanize | strong | min ≥ 0.4 | 98 | 94% | 93% | 0.89 (0.68–0.97) | 0.57 (0.31–0.75) | 30% | 30% | – | 1.09 | 2.3 |
| qwen38-27b | humanize | strong | min ≥ 0.6 | 98 | 93% | 93% | 0.89 (0.73–0.97) | 0.57 (0.43–0.75) | 29% | 29% | – | 1.13 | 2.4 |
| qwen38-27b | humanize | strong | min ≥ 0.75 | 98 | 87% | 88% | 0.90 (0.80–0.97) | 0.59 (0.45–0.77) | 30% | 30% | – | 1.24 | 2.6 |
| qwen38-27b | back_translate | moderate | min ≥ 0.05 (E4-3) | 93 | 95% | 97% | 0.71 (0.46–0.90) | 0.33 (0.15–0.57) | 49% | 47% | – | 2.22 | 4.7 |
| qwen38-27b | back_translate | moderate | max ≥ 0.05 | 93 | 95% | 97% | 0.71 (0.46–0.90) | 0.33 (0.15–0.57) | 49% | 47% | – | 2.22 | 4.7 |
| qwen38-27b | back_translate | moderate | max ≥ 0.2 (E4-7) | 93 | 95% | 97% | 0.71 (0.46–0.90) | 0.33 (0.15–0.57) | 49% | 47% | – | 2.22 | 4.7 |
| qwen38-27b | back_translate | moderate | min ≥ 0.2 | 93 | 95% | 97% | 0.71 (0.46–0.90) | 0.33 (0.15–0.57) | 49% | 47% | – | 2.22 | 4.7 |
| qwen38-27b | back_translate | moderate | min ≥ 0.3 | 93 | 95% | 97% | 0.72 (0.49–0.90) | 0.33 (0.17–0.57) | 48% | 46% | – | 2.26 | 4.8 |
| qwen38-27b | back_translate | moderate | min ≥ 0.4 | 93 | 92% | 95% | 0.73 (0.51–0.90) | 0.33 (0.20–0.57) | 48% | 47% | – | 2.34 | 4.9 |
| qwen38-27b | back_translate | moderate | min ≥ 0.6 | 93 | 73% | 74% | 0.76 (0.62–0.90) | 0.42 (0.25–0.58) | 54% | 53% | – | 2.75 | 5.8 |
| qwen38-27b | back_translate | moderate | min ≥ 0.75 | 93 | 43% | 42% | 0.83 (0.76–0.92) | 0.49 (0.39–0.64) | 69% | 69% | – | 3.23 | 6.9 |
| qwen38-27b+dflash2 | paraphrase | light | min ≥ 0.05 (E4-3) | 98 | 100% | 100% | 0.80 (0.30–0.97) | 0.48 (0.07–0.75) | 42% | 45% | – | 1.04 | 4.1 |
| qwen38-27b+dflash2 | paraphrase | light | max ≥ 0.05 | 98 | 100% | 100% | 0.80 (0.30–0.97) | 0.48 (0.07–0.75) | 42% | 45% | – | 1.04 | 4.1 |
| qwen38-27b+dflash2 | paraphrase | light | max ≥ 0.2 (E4-7) | 98 | 100% | 100% | 0.81 (0.34–0.99) | 0.50 (0.12–0.76) | 38% | 40% | – | 1.1 | 4.4 |
| qwen38-27b+dflash2 | paraphrase | light | min ≥ 0.2 | 98 | 100% | 100% | 0.81 (0.34–0.99) | 0.50 (0.12–0.76) | 38% | 40% | – | 1.1 | 4.4 |
| qwen38-27b+dflash2 | paraphrase | light | min ≥ 0.3 | 98 | 100% | 100% | 0.82 (0.41–0.99) | 0.50 (0.15–0.76) | 36% | 36% | – | 1.14 | 4.8 |
| qwen38-27b+dflash2 | paraphrase | light | min ≥ 0.4 | 98 | 99% | 100% | 0.83 (0.54–0.99) | 0.53 (0.24–0.76) | 32% | 31% | – | 1.23 | 5.2 |
| qwen38-27b+dflash2 | paraphrase | light | min ≥ 0.6 | 98 | 93% | 94% | 0.86 (0.70–0.99) | 0.57 (0.33–0.80) | 30% | 30% | – | 1.33 | 5.7 |
| qwen38-27b+dflash2 | paraphrase | light | min ≥ 0.75 | 98 | 86% | 88% | 0.89 (0.79–1.00) | 0.61 (0.43–0.80) | 31% | 30% | – | 1.48 | 6.4 |
| qwen38-27b+dflash2 | paraphrase | moderate | min ≥ 0.05 (E4-3) | 98 | 99% | 98% | 0.92 (0.66–1.00) | 0.67 (0.39–0.84) | 22% | 23% | – | 1.05 | 5.6 |
| qwen38-27b+dflash2 | paraphrase | moderate | max ≥ 0.05 | 98 | 99% | 98% | 0.92 (0.66–1.00) | 0.67 (0.39–0.84) | 22% | 23% | – | 1.05 | 5.6 |
| qwen38-27b+dflash2 | paraphrase | moderate | max ≥ 0.2 (E4-7) | 98 | 99% | 98% | 0.92 (0.67–1.00) | 0.68 (0.40–0.84) | 21% | 21% | – | 1.08 | 5.8 |
| qwen38-27b+dflash2 | paraphrase | moderate | min ≥ 0.2 | 98 | 99% | 98% | 0.92 (0.67–1.00) | 0.68 (0.40–0.84) | 21% | 21% | – | 1.08 | 5.8 |
| qwen38-27b+dflash2 | paraphrase | moderate | min ≥ 0.3 | 98 | 99% | 98% | 0.92 (0.67–1.00) | 0.68 (0.40–0.84) | 21% | 21% | – | 1.08 | 5.8 |
| qwen38-27b+dflash2 | paraphrase | moderate | min ≥ 0.4 | 98 | 99% | 98% | 0.92 (0.69–1.00) | 0.68 (0.43–0.84) | 20% | 20% | – | 1.1 | 5.9 |
| qwen38-27b+dflash2 | paraphrase | moderate | min ≥ 0.6 | 98 | 98% | 97% | 0.92 (0.69–1.00) | 0.68 (0.47–0.85) | 20% | 21% | – | 1.11 | 5.9 |
| qwen38-27b+dflash2 | paraphrase | moderate | min ≥ 0.75 | 98 | 92% | 91% | 0.93 (0.79–1.00) | 0.69 (0.52–0.86) | 22% | 22% | – | 1.22 | 6.3 |
| qwen38-27b+dflash2 | paraphrase | strong | min ≥ 0.05 (E4-3) | 98 | 98% | 98% | 0.92 (0.73–1.00) | 0.67 (0.43–0.83) | 22% | 22% | – | 1.06 | 5.9 |
| qwen38-27b+dflash2 | paraphrase | strong | max ≥ 0.05 | 98 | 98% | 98% | 0.92 (0.73–1.00) | 0.67 (0.43–0.83) | 22% | 22% | – | 1.06 | 5.9 |
| qwen38-27b+dflash2 | paraphrase | strong | max ≥ 0.2 (E4-7) | 98 | 97% | 97% | 0.92 (0.75–1.00) | 0.67 (0.44–0.83) | 21% | 21% | – | 1.09 | 6.0 |
| qwen38-27b+dflash2 | paraphrase | strong | min ≥ 0.2 | 98 | 97% | 97% | 0.92 (0.75–1.00) | 0.67 (0.44–0.83) | 21% | 21% | – | 1.09 | 6.0 |
| qwen38-27b+dflash2 | paraphrase | strong | min ≥ 0.3 | 98 | 97% | 97% | 0.92 (0.75–1.00) | 0.67 (0.44–0.83) | 21% | 21% | – | 1.09 | 6.0 |
| qwen38-27b+dflash2 | paraphrase | strong | min ≥ 0.4 | 98 | 97% | 97% | 0.92 (0.75–1.00) | 0.67 (0.44–0.83) | 21% | 21% | – | 1.09 | 6.0 |
| qwen38-27b+dflash2 | paraphrase | strong | min ≥ 0.6 | 98 | 96% | 95% | 0.92 (0.76–1.00) | 0.67 (0.45–0.83) | 21% | 21% | – | 1.11 | 6.2 |
| qwen38-27b+dflash2 | paraphrase | strong | min ≥ 0.75 | 98 | 93% | 94% | 0.92 (0.80–1.00) | 0.69 (0.55–0.84) | 22% | 20% | – | 1.17 | 6.4 |
| qwen38-27b+dflash2 | humanize | moderate | min ≥ 0.05 (E4-3) | 98 | 94% | 92% | 0.79 (0.54–0.94) | 0.47 (0.23–0.69) | 41% | 44% | – | 1.12 | 4.3 |
| qwen38-27b+dflash2 | humanize | moderate | max ≥ 0.05 | 98 | 94% | 92% | 0.79 (0.54–0.94) | 0.47 (0.23–0.69) | 41% | 44% | – | 1.12 | 4.3 |
| qwen38-27b+dflash2 | humanize | moderate | max ≥ 0.2 (E4-7) | 98 | 93% | 91% | 0.79 (0.61–0.94) | 0.47 (0.29–0.69) | 40% | 42% | – | 1.15 | 4.5 |
| qwen38-27b+dflash2 | humanize | moderate | min ≥ 0.2 | 98 | 93% | 91% | 0.79 (0.61–0.94) | 0.47 (0.29–0.69) | 40% | 42% | – | 1.15 | 4.5 |
| qwen38-27b+dflash2 | humanize | moderate | min ≥ 0.3 | 98 | 93% | 91% | 0.79 (0.61–0.94) | 0.47 (0.29–0.69) | 40% | 42% | – | 1.15 | 4.5 |
| qwen38-27b+dflash2 | humanize | moderate | min ≥ 0.4 | 98 | 91% | 91% | 0.80 (0.66–0.94) | 0.48 (0.29–0.69) | 40% | 42% | – | 1.19 | 4.6 |
| qwen38-27b+dflash2 | humanize | moderate | min ≥ 0.6 | 98 | 89% | 88% | 0.81 (0.67–0.94) | 0.49 (0.30–0.69) | 40% | 43% | – | 1.23 | 4.7 |
| qwen38-27b+dflash2 | humanize | moderate | min ≥ 0.75 | 98 | 69% | 70% | 0.87 (0.76–0.97) | 0.54 (0.34–0.75) | 48% | 51% | – | 1.45 | 5.5 |
| qwen38-27b+dflash2 | humanize | strong | min ≥ 0.05 (E4-3) | 98 | 96% | 94% | 0.88 (0.67–0.97) | 0.57 (0.31–0.74) | 30% | 30% | – | 1.07 | 4.8 |
| qwen38-27b+dflash2 | humanize | strong | max ≥ 0.05 | 98 | 96% | 94% | 0.88 (0.67–0.97) | 0.57 (0.31–0.74) | 30% | 30% | – | 1.07 | 4.8 |
| qwen38-27b+dflash2 | humanize | strong | max ≥ 0.2 (E4-7) | 98 | 96% | 94% | 0.88 (0.67–0.97) | 0.57 (0.31–0.74) | 30% | 30% | – | 1.07 | 4.8 |
| qwen38-27b+dflash2 | humanize | strong | min ≥ 0.2 | 98 | 96% | 94% | 0.88 (0.67–0.97) | 0.57 (0.31–0.74) | 30% | 30% | – | 1.07 | 4.8 |
| qwen38-27b+dflash2 | humanize | strong | min ≥ 0.3 | 98 | 96% | 94% | 0.88 (0.68–0.97) | 0.57 (0.31–0.74) | 29% | 30% | – | 1.08 | 4.9 |
| qwen38-27b+dflash2 | humanize | strong | min ≥ 0.4 | 98 | 95% | 94% | 0.88 (0.69–0.97) | 0.57 (0.31–0.74) | 29% | 30% | – | 1.09 | 4.9 |
| qwen38-27b+dflash2 | humanize | strong | min ≥ 0.6 | 98 | 94% | 94% | 0.89 (0.73–0.97) | 0.58 (0.38–0.74) | 28% | 28% | – | 1.13 | 5.1 |
| qwen38-27b+dflash2 | humanize | strong | min ≥ 0.75 | 98 | 87% | 88% | 0.90 (0.80–0.97) | 0.60 (0.45–0.75) | 30% | 30% | – | 1.27 | 5.5 |
| qwen38-27b+dflash2 | back_translate | moderate | min ≥ 0.05 (E4-3) | 93 | 95% | 97% | 0.71 (0.46–0.88) | 0.33 (0.15–0.57) | 49% | 47% | – | 2.22 | 9.4 |
| qwen38-27b+dflash2 | back_translate | moderate | max ≥ 0.05 | 93 | 95% | 97% | 0.71 (0.46–0.88) | 0.33 (0.15–0.57) | 49% | 47% | – | 2.22 | 9.4 |
| qwen38-27b+dflash2 | back_translate | moderate | max ≥ 0.2 (E4-7) | 93 | 95% | 97% | 0.71 (0.46–0.88) | 0.33 (0.15–0.57) | 49% | 47% | – | 2.22 | 9.4 |
| qwen38-27b+dflash2 | back_translate | moderate | min ≥ 0.2 | 93 | 95% | 97% | 0.71 (0.46–0.88) | 0.33 (0.15–0.57) | 49% | 47% | – | 2.22 | 9.4 |
| qwen38-27b+dflash2 | back_translate | moderate | min ≥ 0.3 | 93 | 95% | 97% | 0.72 (0.49–0.88) | 0.33 (0.17–0.57) | 48% | 46% | – | 2.26 | 9.5 |
| qwen38-27b+dflash2 | back_translate | moderate | min ≥ 0.4 | 93 | 92% | 95% | 0.73 (0.51–0.90) | 0.34 (0.20–0.58) | 48% | 47% | – | 2.34 | 9.8 |
| qwen38-27b+dflash2 | back_translate | moderate | min ≥ 0.6 | 93 | 74% | 74% | 0.76 (0.62–0.90) | 0.42 (0.25–0.59) | 53% | 53% | – | 2.75 | 11.3 |
| qwen38-27b+dflash2 | back_translate | moderate | min ≥ 0.75 | 93 | 44% | 42% | 0.82 (0.75–0.92) | 0.49 (0.41–0.66) | 68% | 68% | – | 3.23 | 13.4 |

### Paragraphs kept as they were — GPU 2 × 2, `paraphrase` moderate, the loop's policy (E4-3 before, E4-7 after)

| model | policy | chunks | kept as they were | by kind | why their attempts failed |
|---|---|---|---|---|---|
| qwen38-27b | min ≥ 0.05 (E4-3) | 98 | 0 (0%) |  |  |
| qwen38-27b | max ≥ 0.2 (E4-7) | 98 | 0 (0%) |  |  |
| qwen38-27b+dflash2 | min ≥ 0.05 (E4-3) | 98 | 0 (0%) |  |  |
| qwen38-27b+dflash2 | max ≥ 0.2 (E4-7) | 98 | 0 (0%) |  |  |

### Selection policies per language — GPU 2 × 2, `paraphrase`

| model | intensity | lang | policy | rewritten | divergence med | word change med | judged CHANGED |
|---|---|---|---|---|---|---|---|
| qwen38-27b | moderate | en | min ≥ 0.05 (E4-3) | 100% | 0.79 | 0.59 | – |
| qwen38-27b | moderate | ru | min ≥ 0.05 (E4-3) | 100% | 0.93 | 0.70 | – |
| qwen38-27b | moderate | de | min ≥ 0.05 (E4-3) | 100% | 0.87 | 0.63 | – |
| qwen38-27b | moderate | en | max ≥ 0.2 (E4-7) | 100% | 0.91 | 0.65 | – |
| qwen38-27b | moderate | ru | max ≥ 0.2 (E4-7) | 100% | 0.97 | 0.78 | – |
| qwen38-27b | moderate | de | max ≥ 0.2 (E4-7) | 100% | 0.94 | 0.71 | – |
| qwen38-27b | moderate | en | min ≥ 0.2 | 100% | 0.79 | 0.60 | – |
| qwen38-27b | moderate | ru | min ≥ 0.2 | 100% | 0.93 | 0.70 | – |
| qwen38-27b | moderate | de | min ≥ 0.2 | 100% | 0.87 | 0.63 | – |
| qwen38-27b | moderate | en | min ≥ 0.6 | 100% | 0.83 | 0.60 | – |
| qwen38-27b | moderate | ru | min ≥ 0.6 | 100% | 0.93 | 0.70 | – |
| qwen38-27b | moderate | de | min ≥ 0.6 | 100% | 0.87 | 0.64 | – |
| qwen38-27b | strong | en | min ≥ 0.05 (E4-3) | 100% | 0.83 | 0.59 | – |
| qwen38-27b | strong | ru | min ≥ 0.05 (E4-3) | 100% | 0.91 | 0.69 | – |
| qwen38-27b | strong | de | min ≥ 0.05 (E4-3) | 100% | 0.90 | 0.66 | – |
| qwen38-27b | strong | en | max ≥ 0.2 (E4-7) | 100% | 0.92 | 0.67 | – |
| qwen38-27b | strong | ru | max ≥ 0.2 (E4-7) | 100% | 0.97 | 0.74 | – |
| qwen38-27b | strong | de | max ≥ 0.2 (E4-7) | 100% | 0.95 | 0.74 | – |
| qwen38-27b | strong | en | min ≥ 0.2 | 100% | 0.83 | 0.59 | – |
| qwen38-27b | strong | ru | min ≥ 0.2 | 100% | 0.91 | 0.69 | – |
| qwen38-27b | strong | de | min ≥ 0.2 | 100% | 0.90 | 0.66 | – |
| qwen38-27b | strong | en | min ≥ 0.6 | 100% | 0.85 | 0.63 | – |
| qwen38-27b | strong | ru | min ≥ 0.6 | 100% | 0.91 | 0.69 | – |
| qwen38-27b | strong | de | min ≥ 0.6 | 100% | 0.90 | 0.66 | – |
| qwen38-27b+dflash2 | moderate | en | min ≥ 0.05 (E4-3) | 100% | 0.79 | 0.59 | – |
| qwen38-27b+dflash2 | moderate | ru | min ≥ 0.05 (E4-3) | 100% | 0.92 | 0.70 | – |
| qwen38-27b+dflash2 | moderate | de | min ≥ 0.05 (E4-3) | 100% | 0.88 | 0.65 | – |
| qwen38-27b+dflash2 | moderate | en | max ≥ 0.2 (E4-7) | 100% | 0.91 | 0.65 | – |
| qwen38-27b+dflash2 | moderate | ru | max ≥ 0.2 (E4-7) | 100% | 0.97 | 0.78 | – |
| qwen38-27b+dflash2 | moderate | de | max ≥ 0.2 (E4-7) | 100% | 0.93 | 0.70 | – |
| qwen38-27b+dflash2 | moderate | en | min ≥ 0.2 | 100% | 0.79 | 0.60 | – |
| qwen38-27b+dflash2 | moderate | ru | min ≥ 0.2 | 100% | 0.92 | 0.70 | – |
| qwen38-27b+dflash2 | moderate | de | min ≥ 0.2 | 100% | 0.88 | 0.65 | – |
| qwen38-27b+dflash2 | moderate | en | min ≥ 0.6 | 100% | 0.83 | 0.60 | – |
| qwen38-27b+dflash2 | moderate | ru | min ≥ 0.6 | 100% | 0.92 | 0.70 | – |
| qwen38-27b+dflash2 | moderate | de | min ≥ 0.6 | 100% | 0.89 | 0.65 | – |
| qwen38-27b+dflash2 | strong | en | min ≥ 0.05 (E4-3) | 100% | 0.83 | 0.57 | – |
| qwen38-27b+dflash2 | strong | ru | min ≥ 0.05 (E4-3) | 100% | 0.93 | 0.73 | – |
| qwen38-27b+dflash2 | strong | de | min ≥ 0.05 (E4-3) | 97% | 0.92 | 0.66 | – |
| qwen38-27b+dflash2 | strong | en | max ≥ 0.2 (E4-7) | 100% | 0.92 | 0.67 | – |
| qwen38-27b+dflash2 | strong | ru | max ≥ 0.2 (E4-7) | 100% | 0.96 | 0.75 | – |
| qwen38-27b+dflash2 | strong | de | max ≥ 0.2 (E4-7) | 97% | 0.96 | 0.76 | – |
| qwen38-27b+dflash2 | strong | en | min ≥ 0.2 | 100% | 0.83 | 0.57 | – |
| qwen38-27b+dflash2 | strong | ru | min ≥ 0.2 | 100% | 0.93 | 0.73 | – |
| qwen38-27b+dflash2 | strong | de | min ≥ 0.2 | 97% | 0.92 | 0.66 | – |
| qwen38-27b+dflash2 | strong | en | min ≥ 0.6 | 100% | 0.83 | 0.59 | – |
| qwen38-27b+dflash2 | strong | ru | min ≥ 0.6 | 100% | 0.93 | 0.73 | – |
| qwen38-27b+dflash2 | strong | de | min ≥ 0.6 | 97% | 0.92 | 0.66 | – |

### Voice — the loop's pick (E4-7) beside the least diverged; every language

GPU 2 × 2 where the run made four candidates a chunk, CPU 1 × 2 where it made two. Second and first person are pronouns counted, kept by count; a chunk kept as it was keeps its voice. The register shift is a proxy: the share of the answer's words that a list of formal words and suffixes matches and the source has no word for (`docs/architecture/prompt-bench.md`, "Voice").

| model | tactic | intensity | executor | policy | chunks | with 2nd person | 2nd person kept, by count | per chunk, med [q1–q3] | lost any / lost all | ты↔вы, du↔Sie switched | 1st person kept | words ×, all · per chunk med [q1–q3] | register shift med [q1–q3] · mean | voice judged YES / PARTLY / NO |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| qwen38-27b | paraphrase | light | GPU 2 × 2 | max ≥ 0.2 (E4-7) | 98 | 33 | 61% | 1.00 [0.20–1.00] | 48% / 21% | 0 of 22 | 83% | 1.01 · 1.02 [0.97–1.12] | 0.01 [0.00–0.04] · 0.029 | – |
| qwen38-27b | paraphrase | light | GPU 2 × 2 | min ≥ 0.2 | 98 | 33 | 53% | 0.67 [0.00–1.00] | 61% / 36% | 0 of 22 | 85% | 0.97 · 1.00 [0.94–1.04] | 0.00 [0.00–0.01] · 0.012 | – |
| qwen38-27b | paraphrase | moderate | GPU 2 × 2 | max ≥ 0.2 (E4-7) | 98 | 33 | 58% | 0.60 [0.00–1.00] | 70% / 36% | 0 of 22 | 78% | 1.09 · 1.09 [1.00–1.22] | 0.03 [0.00–0.05] · 0.040 | – |
| qwen38-27b | paraphrase | moderate | GPU 2 × 2 | min ≥ 0.2 | 98 | 33 | 65% | 1.00 [0.33–1.00] | 48% / 24% | 1 of 22 | 88% | 1.08 · 1.09 [1.00–1.24] | 0.01 [0.00–0.04] · 0.029 | – |
| qwen38-27b | paraphrase | strong | GPU 2 × 2 | max ≥ 0.2 (E4-7) | 98 | 33 | 49% | 0.40 [0.00–0.80] | 79% / 36% | 0 of 22 | 73% | 1.07 · 1.13 [0.95–1.29] | 0.03 [0.00–0.05] · 0.036 | – |
| qwen38-27b | paraphrase | strong | GPU 2 × 2 | min ≥ 0.2 | 98 | 33 | 68% | 0.67 [0.00–1.00] | 61% / 30% | 1 of 22 | 83% | 1.07 · 1.08 [0.96–1.23] | 0.01 [0.00–0.04] · 0.025 | – |
| qwen38-27b | humanize | moderate | CPU 1 × 2 | max ≥ 0.2 (E4-7) | 98 | 33 | 60% | 0.60 [0.00–1.00] | 73% / 36% | 1 of 22 | 78% | 0.89 · 0.94 [0.81–1.00] | 0.00 [0.00–0.00] · 0.007 | – |
| qwen38-27b | humanize | moderate | CPU 1 × 2 | min ≥ 0.2 | 98 | 33 | 60% | 0.60 [0.00–1.00] | 73% / 36% | 1 of 22 | 78% | 0.89 · 0.94 [0.81–1.00] | 0.00 [0.00–0.00] · 0.007 | – |
| qwen38-27b | humanize | strong | CPU 1 × 2 | max ≥ 0.2 (E4-7) | 98 | 33 | 56% | 0.50 [0.00–1.00] | 73% / 36% | 0 of 22 | 81% | 0.91 · 0.93 [0.82–1.04] | 0.00 [0.00–0.00] · 0.008 | – |
| qwen38-27b | humanize | strong | CPU 1 × 2 | min ≥ 0.2 | 98 | 33 | 56% | 0.50 [0.00–1.00] | 73% / 36% | 0 of 22 | 81% | 0.91 · 0.93 [0.82–1.04] | 0.00 [0.00–0.00] · 0.008 | – |
| qwen38-27b | back_translate | moderate | CPU 1 × 2 | max ≥ 0.2 (E4-7) | 93 | 31 | 77% | 1.00 [0.50–1.00] | 45% / 16% | 3 of 21 | 92% | 1.01 · 1.01 [0.98–1.07] | 0.00 [0.00–0.02] · 0.015 | – |
| qwen38-27b | back_translate | moderate | CPU 1 × 2 | min ≥ 0.2 | 93 | 31 | 77% | 1.00 [0.50–1.00] | 45% / 16% | 3 of 21 | 92% | 1.01 · 1.01 [0.98–1.07] | 0.00 [0.00–0.02] · 0.015 | – |
| qwen38-27b+dflash2 | paraphrase | light | GPU 2 × 2 | max ≥ 0.2 (E4-7) | 98 | 33 | 55% | 0.67 [0.20–1.00] | 58% / 24% | 0 of 22 | 83% | 1.00 · 1.01 [0.95–1.13] | 0.01 [0.00–0.04] · 0.029 | – |
| qwen38-27b+dflash2 | paraphrase | light | GPU 2 × 2 | min ≥ 0.2 | 98 | 33 | 52% | 0.67 [0.00–1.00] | 61% / 33% | 0 of 22 | 80% | 0.97 · 1.00 [0.94–1.04] | 0.00 [0.00–0.02] · 0.015 | – |
| qwen38-27b+dflash2 | paraphrase | moderate | GPU 2 × 2 | max ≥ 0.2 (E4-7) | 98 | 33 | 64% | 0.67 [0.00–1.00] | 64% / 36% | 0 of 22 | 80% | 1.09 · 1.09 [1.00–1.22] | 0.03 [0.00–0.06] · 0.043 | – |
| qwen38-27b+dflash2 | paraphrase | moderate | GPU 2 × 2 | min ≥ 0.2 | 98 | 33 | 59% | 1.00 [0.00–1.00] | 48% / 27% | 1 of 22 | 88% | 1.06 · 1.07 [0.98–1.20] | 0.01 [0.00–0.04] · 0.026 | – |
| qwen38-27b+dflash2 | paraphrase | strong | GPU 2 × 2 | max ≥ 0.2 (E4-7) | 98 | 33 | 46% | 0.33 [0.00–0.80] | 76% / 36% | 0 of 22 | 75% | 1.08 · 1.13 [0.98–1.30] | 0.03 [0.00–0.06] · 0.038 | – |
| qwen38-27b+dflash2 | paraphrase | strong | GPU 2 × 2 | min ≥ 0.2 | 98 | 33 | 64% | 0.67 [0.00–1.00] | 64% / 30% | 0 of 22 | 83% | 1.06 · 1.08 [0.95–1.20] | 0.01 [0.00–0.04] · 0.024 | – |
| qwen38-27b+dflash2 | humanize | moderate | CPU 1 × 2 | max ≥ 0.2 (E4-7) | 98 | 33 | 62% | 0.60 [0.00–1.00] | 70% / 36% | 1 of 22 | 78% | 0.90 · 0.95 [0.82–1.00] | 0.00 [0.00–0.00] · 0.006 | – |
| qwen38-27b+dflash2 | humanize | moderate | CPU 1 × 2 | min ≥ 0.2 | 98 | 33 | 62% | 0.60 [0.00–1.00] | 70% / 36% | 1 of 22 | 78% | 0.90 · 0.95 [0.82–1.00] | 0.00 [0.00–0.00] · 0.006 | – |
| qwen38-27b+dflash2 | humanize | strong | CPU 1 × 2 | max ≥ 0.2 (E4-7) | 98 | 33 | 56% | 0.50 [0.00–1.00] | 73% / 36% | 0 of 22 | 81% | 0.92 · 0.93 [0.82–1.02] | 0.00 [0.00–0.00] · 0.007 | – |
| qwen38-27b+dflash2 | humanize | strong | CPU 1 × 2 | min ≥ 0.2 | 98 | 33 | 56% | 0.50 [0.00–1.00] | 73% / 36% | 0 of 22 | 81% | 0.92 · 0.93 [0.82–1.02] | 0.00 [0.00–0.00] · 0.007 | – |
| qwen38-27b+dflash2 | back_translate | moderate | CPU 1 × 2 | max ≥ 0.2 (E4-7) | 93 | 31 | 78% | 1.00 [0.50–1.00] | 42% / 13% | 3 of 21 | 92% | 1.01 · 1.01 [1.00–1.07] | 0.00 [0.00–0.02] · 0.015 | – |
| qwen38-27b+dflash2 | back_translate | moderate | CPU 1 × 2 | min ≥ 0.2 | 93 | 31 | 78% | 1.00 [0.50–1.00] | 42% / 13% | 3 of 21 | 92% | 1.01 · 1.01 [1.00–1.07] | 0.00 [0.00–0.02] · 0.015 | – |

### The language check (candidates that passed, chunk language detected; the loop refuses 20+ words since E4-7)

| model | passed, chunk language known | answer in another language | answer language unknown | of those two, planted-instruction items |
|---|---|---|---|---|
| qwen38-27b | 1284 | 0 (0%) | 24 (2%) | 0 |
| qwen38-27b+dflash2 | 1286 | 0 (0%) | 23 (2%) | 0 |

### The no-op floor and the length window (every tactic)

| model | candidates that passed all but the floor | divergence < 0.05 | < 0.10 | < 0.20 | length-guard rejections (0.6–1.6) | of them inside 0.5–2.0 | judged CHANGED: inside 0.6–1.6 | in 0.5–0.6 or 1.6–2.0 | outside 0.5–2.0 |
|---|---|---|---|---|---|---|---|---|---|
| qwen38-27b | 1721 | 2% | 3% | 4% | 67 | 22 | – of 0 | – of 0 | – of 0 |
| qwen38-27b+dflash2 | 1721 | 2% | 3% | 4% | 64 | 19 | – of 0 | – of 0 | – of 0 |

### Judged CHANGED by divergence band (every judged candidate inside the length window)

| model | < 0.2 | 0.2–0.4 | 0.4–0.6 | 0.6–0.8 | ≥ 0.8 |
|---|---|---|---|---|---|
| qwen38-27b | – of 0 | – of 0 | – of 0 | – of 0 | – of 0 |
| qwen38-27b+dflash2 | – of 0 | – of 0 | – of 0 | – of 0 | – of 0 |

### The judge's calibration

| judge | case | expected | n | as expected |
|---|---|---|---|---|
