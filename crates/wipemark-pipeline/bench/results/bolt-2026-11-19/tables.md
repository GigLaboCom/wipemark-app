### Attempts: per model, tactic, intensity and language

| model | tactic | intensity | lang | n | passed | placeholder ✗ | numbers ✗ | length ✗ | script ✗ | identifier ✗ | restore ✗ | no-op | trunc. | divergence med (p10–p90) | word change med (p10–p90) | pairs carried over med | length ratio med (p10–p90) | lang kept / other | preface (passed) | s/attempt |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| gemma4-12b | humanize | moderate | de | 68 | 91% | 0% | 0% | 2% | 0% | 0% | 0% | 6% | 0% | 0.75 (0.49–0.91) | 0.39 (0.17–0.63) | 0.44 | 0.97 (0.76–1.23) | 79% / 0% | 3% (3%) | 1.25 |
| gemma4-12b | humanize | moderate | en | 60 | 100% | 0% | 0% | 0% | 0% | 0% | 0% | 0% | 0% | 0.82 (0.63–0.92) | 0.46 (0.29–0.67) | 0.33 | 0.92 (0.68–1.28) | 82% / 0% | 0% (0%) | 0.99 |
| gemma4-12b | humanize | moderate | ru | 68 | 96% | 0% | 2% | 0% | 3% | 0% | 0% | 0% | 0% | 0.86 (0.70–0.97) | 0.54 (0.37–0.78) | 0.26 | 1.02 (0.84–1.18) | 76% / 3% | 0% (0%) | 1.35 |
| gemma4-12b | humanize | strong | de | 68 | 98% | 0% | 0% | 0% | 0% | 0% | 0% | 0% | 0% | 0.83 (0.72–0.95) | 0.52 (0.37–0.72) | 0.30 | 0.97 (0.76–1.23) | 78% / 0% | 3% (3%) | 1.29 |
| gemma4-12b | humanize | strong | en | 60 | 98% | 0% | 0% | 0% | 0% | 0% | 0% | 2% | 0% | 0.85 (0.68–0.95) | 0.52 (0.36–0.73) | 0.29 | 0.94 (0.72–1.32) | 85% / 0% | 0% (0%) | 1.01 |
| gemma4-12b | humanize | strong | ru | 68 | 96% | 0% | 2% | 0% | 3% | 0% | 0% | 0% | 0% | 0.91 (0.75–1.00) | 0.64 (0.48–0.89) | 0.17 | 1.05 (0.86–1.37) | 82% / 3% | 2% (2%) | 1.39 |
| gemma4-12b | paraphrase | light | de | 136 | 99% | 0% | 0% | 1% | 0% | 0% | 0% | 0% | 0% | 0.77 (0.60–0.91) | 0.44 (0.27–0.62) | 0.36 | 1.08 (0.99–1.38) | 84% / 0% | 0% (0%) | 1.39 |
| gemma4-12b | paraphrase | light | en | 120 | 96% | 0% | 0% | 1% | 0% | 0% | 0% | 3% | 0% | 0.85 (0.50–0.95) | 0.52 (0.24–0.73) | 0.27 | 1.12 (0.98–1.54) | 88% / 0% | 0% (0%) | 1.12 |
| gemma4-12b | paraphrase | light | ru | 136 | 96% | 0% | 1% | 2% | 1% | 0% | 0% | 2% | 0% | 0.89 (0.70–1.00) | 0.65 (0.46–0.86) | 0.20 | 1.19 (1.01–1.54) | 86% / 0% | 0% (0%) | 1.42 |
| gemma4-12b | paraphrase | moderate | de | 136 | 96% | 0% | 0% | 4% | 0% | 0% | 0% | 0% | 0% | 0.89 (0.73–1.00) | 0.66 (0.47–0.80) | 0.18 | 1.21 (1.02–1.69) | 94% / 0% | 0% (0%) | 1.47 |
| gemma4-12b | paraphrase | moderate | en | 120 | 97% | 0% | 0% | 3% | 0% | 0% | 0% | 0% | 0% | 0.93 (0.76–1.00) | 0.70 (0.56–0.83) | 0.14 | 1.22 (0.98–1.58) | 97% / 0% | 0% (0%) | 1.14 |
| gemma4-12b | paraphrase | moderate | ru | 136 | 96% | 0% | 0% | 4% | 0% | 0% | 0% | 0% | 0% | 0.94 (0.80–1.00) | 0.74 (0.58–0.89) | 0.11 | 1.29 (1.08–1.77) | 88% / 0% | 1% (1%) | 1.45 |
| gemma4-12b | paraphrase | strong | de | 136 | 95% | 0% | 0% | 5% | 0% | 0% | 0% | 0% | 0% | 0.91 (0.74–1.00) | 0.69 (0.50–0.82) | 0.16 | 1.22 (1.05–1.71) | 94% / 0% | 0% (0%) | 1.49 |
| gemma4-12b | paraphrase | strong | en | 120 | 93% | 0% | 0% | 7% | 0% | 0% | 0% | 0% | 0% | 0.93 (0.76–1.00) | 0.73 (0.59–0.85) | 0.12 | 1.24 (0.99–1.68) | 98% / 0% | 0% (0%) | 1.16 |
| gemma4-12b | paraphrase | strong | ru | 136 | 94% | 0% | 1% | 4% | 1% | 0% | 0% | 0% | 0% | 0.95 (0.83–1.00) | 0.76 (0.63–0.90) | 0.08 | 1.33 (1.09–1.81) | 89% / 0% | 0% (0%) | 1.48 |
| nemotron-bolt | humanize | moderate | de | 68 | 100% | 0% | 0% | 0% | 0% | 0% | 0% | 0% | 0% | 0.75 (0.57–0.89) | 0.32 (0.20–0.54) | 0.43 | 0.83 (0.64–0.97) | 76% / 0% | 0% (0%) | 1.02 |
| nemotron-bolt | humanize | moderate | en | 60 | 90% | 0% | 0% | 7% | 0% | 0% | 0% | 3% | 0% | 0.80 (0.50–0.93) | 0.43 (0.23–0.64) | 0.35 | 0.79 (0.62–1.00) | 75% / 0% | 0% (0%) | 0.73 |
| nemotron-bolt | humanize | moderate | ru | 68 | 98% | 0% | 0% | 0% | 0% | 0% | 0% | 2% | 0% | 0.76 (0.50–0.92) | 0.35 (0.16–0.56) | 0.43 | 0.83 (0.68–1.00) | 74% / 0% | 0% (0%) | 0.96 |
| nemotron-bolt | humanize | strong | de | 68 | 97% | 0% | 0% | 2% | 0% | 0% | 0% | 2% | 0% | 0.75 (0.57–0.91) | 0.33 (0.20–0.54) | 0.43 | 0.82 (0.64–1.00) | 78% / 0% | 0% (0%) | 0.98 |
| nemotron-bolt | humanize | strong | en | 60 | 92% | 0% | 0% | 7% | 0% | 0% | 0% | 2% | 0% | 0.85 (0.57–0.93) | 0.50 (0.21–0.67) | 0.29 | 0.84 (0.63–1.00) | 75% / 0% | 0% (0%) | 0.74 |
| nemotron-bolt | humanize | strong | ru | 68 | 97% | 0% | 0% | 2% | 2% | 0% | 0% | 2% | 0% | 0.80 (0.53–0.93) | 0.40 (0.19–0.58) | 0.36 | 0.86 (0.68–1.00) | 76% / 0% | 0% (0%) | 1.08 |
| nemotron-bolt | paraphrase | light | de | 136 | 99% | 0% | 0% | 0% | 0% | 0% | 0% | 1% | 0% | 0.66 (0.46–0.88) | 0.35 (0.17–0.57) | 0.55 | 0.98 (0.88–1.06) | 76% / 0% | 0% (0%) | 0.99 |
| nemotron-bolt | paraphrase | light | en | 120 | 100% | 0% | 0% | 0% | 0% | 0% | 0% | 0% | 0% | 0.89 (0.70–1.00) | 0.59 (0.40–0.77) | 0.22 | 1.02 (0.83–1.17) | 81% / 0% | 0% (0%) | 0.81 |
| nemotron-bolt | paraphrase | light | ru | 136 | 98% | 0% | 0% | 1% | 0% | 0% | 0% | 0% | 0% | 0.83 (0.62–0.96) | 0.48 (0.30–0.72) | 0.29 | 1.05 (0.90–1.21) | 76% / 0% | 0% (0%) | 0.95 |
| nemotron-bolt | paraphrase | moderate | de | 136 | 100% | 0% | 0% | 0% | 0% | 0% | 0% | 0% | 0% | 0.77 (0.59–0.91) | 0.50 (0.32–0.67) | 0.39 | 0.99 (0.89–1.12) | 74% / 0% | 0% (0%) | 0.99 |
| nemotron-bolt | paraphrase | moderate | en | 120 | 100% | 0% | 0% | 0% | 0% | 0% | 0% | 0% | 0% | 0.90 (0.77–0.98) | 0.67 (0.52–0.80) | 0.19 | 1.07 (0.81–1.29) | 83% / 0% | 0% (0%) | 0.79 |
| nemotron-bolt | paraphrase | moderate | ru | 136 | 97% | 0% | 0% | 1% | 0% | 0% | 0% | 0% | 0% | 0.88 (0.70–0.97) | 0.56 (0.38–0.75) | 0.21 | 1.08 (0.95–1.25) | 74% / 0% | 0% (0%) | 0.95 |
| nemotron-bolt | paraphrase | strong | de | 136 | 100% | 0% | 0% | 0% | 0% | 0% | 0% | 0% | 0% | 0.86 (0.68–0.95) | 0.56 (0.40–0.71) | 0.27 | 1.00 (0.90–1.14) | 78% / 0% | 0% (0%) | 1.0 |
| nemotron-bolt | paraphrase | strong | en | 120 | 99% | 0% | 0% | 0% | 0% | 0% | 0% | 0% | 0% | 0.91 (0.79–1.00) | 0.67 (0.56–0.80) | 0.18 | 1.08 (0.85–1.29) | 80% / 0% | 0% (0%) | 0.8 |
| nemotron-bolt | paraphrase | strong | ru | 136 | 98% | 0% | 0% | 1% | 0% | 0% | 0% | 0% | 0% | 0.90 (0.71–1.00) | 0.59 (0.38–0.81) | 0.19 | 1.09 (0.95–1.28) | 76% / 0% | 0% (0%) | 0.96 |

### Speed per model (every attempt)

| model | attempts | tokens out / call | s / attempt | tokens/s (incl. prompt) |
|---|---|---|---|---|
| gemma4-12b | 1568 | 72.0 | 1.32 | 54.8 |
| nemotron-bolt | 1568 | 64.0 | 0.92 | 69.2 |

### Speed with and without a draft (every call)

| model | draft | calls | tokens out | s / call | tokens/s (incl. prompt) | accepted / step | tokens / step |
|---|---|---|---|---|---|---|---|
| gemma4-12b | none | 1568 | 113471 | 1.30 | 55.6 | – | – |
| nemotron-bolt | none | 1568 | 99885 | 0.90 | 70.6 | – | – |

### `paraphrase`, moderate: pass rate by kind of text

| model | prose-pd | machine | markdown | numbers | injection | short | quote |
|---|---|---|---|---|---|---|---|
| gemma4-12b | 44 of 44 | 64 of 64 | 118 of 132 | 24 of 24 | 35 of 36 | 27 of 28 | 32 of 32 |
| nemotron-bolt | 44 of 44 | 63 of 64 | 132 of 132 | 24 of 24 | 34 of 36 | 28 of 28 | 31 of 32 |

### What rejected a candidate (`paraphrase`, all intensities; the loop's first reason)

| model | reason | n | share of attempts |
|---|---|---|---|
| gemma4-12b | guard length-drift (length-drift) | 39 | 3% |
| gemma4-12b | no-op | 6 | 1% |
| gemma4-12b | guard numbers (number-missing) | 2 | 0% |
| gemma4-12b | guard script (script-drift) | 2 | 0% |
| gemma4-12b | language | 2 | 0% |
| nemotron-bolt | language | 7 | 1% |
| nemotron-bolt | guard length-drift (length-drift) | 3 | 0% |
| nemotron-bolt | no-op | 1 | 0% |

### Placeholders `⟦n⟧` (attempts on chunks that carry one)

| model | lang | attempts | every placeholder exactly once | placeholders kept / expected |
|---|---|---|---|---|
| gemma4-12b | en | 112 | 100% | 100% |
| gemma4-12b | ru | 112 | 100% | 100% |
| gemma4-12b | de | 112 | 100% | 100% |
| nemotron-bolt | en | 112 | 100% | 100% |
| nemotron-bolt | ru | 112 | 100% | 100% |
| nemotron-bolt | de | 112 | 100% | 100% |

### Planted instructions (3 items per language)

| model | tactic | attempts | obeyed | obeyed **and** passed every check |
|---|---|---|---|---|
| gemma4-12b | humanize | 36 | 6 (17%) | 0 |
| gemma4-12b | paraphrase | 108 | 0 (0%) | 0 |
| nemotron-bolt | humanize | 36 | 0 (0%) | 0 |
| nemotron-bolt | paraphrase | 108 | 0 (0%) | 0 |

### Selection policies, GPU 2 × 2 (D61) — every language

| model | tactic | intensity | policy | chunks | rewritten | rewritten, by words | divergence med (p10–p90) | word change med (p10–p90) | pairs carried over, mean (all chunks) | pairs carried over, by words | judged CHANGED | calls / chunk | s / chunk |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| gemma4-12b | paraphrase | light | min ≥ 0.05 (E4-3) | 98 | 100% | 100% | 0.80 (0.55–0.94) | 0.48 (0.27–0.75) | 35% | 38% | 4% of 98 | 2.0 | 2.6 |
| gemma4-12b | paraphrase | light | max ≥ 0.05 | 98 | 100% | 100% | 0.86 (0.67–0.99) | 0.56 (0.33–0.78) | 26% | 30% | 9% of 98 | 2.0 | 2.6 |
| gemma4-12b | paraphrase | light | max ≥ 0.2 (E4-7) | 98 | 100% | 100% | 0.86 (0.67–0.99) | 0.56 (0.33–0.78) | 26% | 30% | 9% of 98 | 2.0 | 2.6 |
| gemma4-12b | paraphrase | light | min ≥ 0.2 | 98 | 100% | 100% | 0.80 (0.56–0.94) | 0.49 (0.30–0.75) | 34% | 38% | 5% of 98 | 2.0 | 2.6 |
| gemma4-12b | paraphrase | light | min ≥ 0.3 | 98 | 100% | 100% | 0.80 (0.56–0.94) | 0.49 (0.30–0.75) | 34% | 37% | 5% of 98 | 2.0 | 2.6 |
| gemma4-12b | paraphrase | light | min ≥ 0.4 | 98 | 100% | 100% | 0.80 (0.56–0.94) | 0.49 (0.30–0.75) | 34% | 37% | 5% of 98 | 2.0 | 2.6 |
| gemma4-12b | paraphrase | light | min ≥ 0.6 | 98 | 99% | 99% | 0.81 (0.65–0.94) | 0.50 (0.33–0.75) | 31% | 34% | 7% of 97 | 2.06 | 2.8 |
| gemma4-12b | paraphrase | light | min ≥ 0.75 | 98 | 85% | 85% | 0.84 (0.77–0.95) | 0.56 (0.39–0.78) | 36% | 37% | 8% of 83 | 2.49 | 3.4 |
| gemma4-12b | paraphrase | moderate | min ≥ 0.05 (E4-3) | 98 | 99% | 100% | 0.90 (0.74–0.99) | 0.67 (0.52–0.81) | 21% | 21% | 8% of 97 | 2.06 | 2.8 |
| gemma4-12b | paraphrase | moderate | max ≥ 0.05 | 98 | 99% | 100% | 0.93 (0.78–1.00) | 0.71 (0.58–0.83) | 16% | 16% | 7% of 97 | 2.06 | 2.8 |
| gemma4-12b | paraphrase | moderate | max ≥ 0.2 (E4-7) | 98 | 99% | 100% | 0.93 (0.78–1.00) | 0.71 (0.58–0.83) | 16% | 16% | 7% of 97 | 2.06 | 2.8 |
| gemma4-12b | paraphrase | moderate | min ≥ 0.2 | 98 | 99% | 100% | 0.90 (0.74–0.99) | 0.67 (0.52–0.81) | 21% | 21% | 8% of 97 | 2.06 | 2.8 |
| gemma4-12b | paraphrase | moderate | min ≥ 0.3 | 98 | 99% | 100% | 0.90 (0.74–0.99) | 0.67 (0.52–0.81) | 21% | 21% | 8% of 97 | 2.06 | 2.8 |
| gemma4-12b | paraphrase | moderate | min ≥ 0.4 | 98 | 99% | 100% | 0.90 (0.74–0.99) | 0.67 (0.52–0.81) | 21% | 21% | 8% of 97 | 2.06 | 2.8 |
| gemma4-12b | paraphrase | moderate | min ≥ 0.6 | 98 | 99% | 100% | 0.90 (0.74–0.99) | 0.67 (0.52–0.81) | 21% | 21% | 8% of 97 | 2.06 | 2.8 |
| gemma4-12b | paraphrase | moderate | min ≥ 0.75 | 98 | 96% | 98% | 0.91 (0.77–1.00) | 0.67 (0.55–0.82) | 22% | 21% | 7% of 94 | 2.16 | 2.9 |
| gemma4-12b | paraphrase | strong | min ≥ 0.05 (E4-3) | 98 | 98% | 100% | 0.91 (0.73–1.00) | 0.70 (0.56–0.88) | 19% | 18% | 8% of 96 | 2.06 | 2.8 |
| gemma4-12b | paraphrase | strong | max ≥ 0.05 | 98 | 98% | 100% | 0.95 (0.81–1.00) | 0.75 (0.62–0.89) | 14% | 14% | 10% of 96 | 2.06 | 2.8 |
| gemma4-12b | paraphrase | strong | max ≥ 0.2 (E4-7) | 98 | 98% | 100% | 0.95 (0.81–1.00) | 0.75 (0.62–0.89) | 14% | 14% | 10% of 96 | 2.06 | 2.8 |
| gemma4-12b | paraphrase | strong | min ≥ 0.2 | 98 | 98% | 100% | 0.91 (0.73–1.00) | 0.70 (0.56–0.88) | 19% | 18% | 8% of 96 | 2.06 | 2.8 |
| gemma4-12b | paraphrase | strong | min ≥ 0.3 | 98 | 98% | 100% | 0.91 (0.73–1.00) | 0.70 (0.56–0.88) | 19% | 18% | 8% of 96 | 2.06 | 2.8 |
| gemma4-12b | paraphrase | strong | min ≥ 0.4 | 98 | 98% | 100% | 0.91 (0.73–1.00) | 0.70 (0.56–0.88) | 19% | 18% | 8% of 96 | 2.06 | 2.8 |
| gemma4-12b | paraphrase | strong | min ≥ 0.6 | 98 | 98% | 100% | 0.91 (0.73–1.00) | 0.70 (0.56–0.88) | 19% | 18% | 8% of 96 | 2.06 | 2.8 |
| gemma4-12b | paraphrase | strong | min ≥ 0.75 | 98 | 94% | 95% | 0.92 (0.81–1.00) | 0.72 (0.59–0.88) | 20% | 20% | 10% of 92 | 2.16 | 2.9 |
| nemotron-bolt | paraphrase | light | min ≥ 0.05 (E4-3) | 98 | 100% | 100% | 0.76 (0.44–0.93) | 0.45 (0.24–0.67) | 40% | 43% | 11% of 98 | 2.0 | 1.8 |
| nemotron-bolt | paraphrase | light | max ≥ 0.05 | 98 | 100% | 100% | 0.83 (0.61–0.96) | 0.50 (0.29–0.73) | 32% | 33% | 10% of 98 | 2.0 | 1.8 |
| nemotron-bolt | paraphrase | light | max ≥ 0.2 (E4-7) | 98 | 100% | 100% | 0.83 (0.61–0.96) | 0.50 (0.29–0.73) | 32% | 33% | 10% of 98 | 2.0 | 1.8 |
| nemotron-bolt | paraphrase | light | min ≥ 0.2 | 98 | 100% | 100% | 0.76 (0.44–0.93) | 0.45 (0.24–0.67) | 40% | 43% | 11% of 98 | 2.0 | 1.8 |
| nemotron-bolt | paraphrase | light | min ≥ 0.3 | 98 | 100% | 100% | 0.76 (0.44–0.93) | 0.45 (0.24–0.67) | 40% | 43% | 11% of 98 | 2.0 | 1.8 |
| nemotron-bolt | paraphrase | light | min ≥ 0.4 | 98 | 100% | 100% | 0.77 (0.54–0.93) | 0.45 (0.26–0.67) | 38% | 40% | 11% of 98 | 2.0 | 1.8 |
| nemotron-bolt | paraphrase | light | min ≥ 0.6 | 98 | 95% | 96% | 0.79 (0.63–0.93) | 0.49 (0.32–0.68) | 38% | 39% | 12% of 93 | 2.12 | 1.9 |
| nemotron-bolt | paraphrase | light | min ≥ 0.75 | 98 | 74% | 73% | 0.86 (0.75–0.94) | 0.52 (0.41–0.74) | 45% | 49% | 11% of 73 | 2.61 | 2.5 |
| nemotron-bolt | paraphrase | moderate | min ≥ 0.05 (E4-3) | 98 | 100% | 100% | 0.84 (0.62–0.96) | 0.56 (0.35–0.75) | 32% | 30% | 13% of 98 | 2.0 | 1.8 |
| nemotron-bolt | paraphrase | moderate | max ≥ 0.05 | 98 | 100% | 100% | 0.89 (0.68–0.97) | 0.60 (0.43–0.78) | 25% | 24% | 13% of 98 | 2.0 | 1.8 |
| nemotron-bolt | paraphrase | moderate | max ≥ 0.2 (E4-7) | 98 | 100% | 100% | 0.89 (0.68–0.97) | 0.60 (0.43–0.78) | 25% | 24% | 13% of 98 | 2.0 | 1.8 |
| nemotron-bolt | paraphrase | moderate | min ≥ 0.2 | 98 | 100% | 100% | 0.84 (0.62–0.96) | 0.56 (0.35–0.75) | 32% | 30% | 13% of 98 | 2.0 | 1.8 |
| nemotron-bolt | paraphrase | moderate | min ≥ 0.3 | 98 | 100% | 100% | 0.84 (0.62–0.96) | 0.56 (0.35–0.75) | 32% | 30% | 13% of 98 | 2.0 | 1.8 |
| nemotron-bolt | paraphrase | moderate | min ≥ 0.4 | 98 | 100% | 100% | 0.84 (0.62–0.96) | 0.56 (0.37–0.75) | 32% | 30% | 13% of 98 | 2.0 | 1.8 |
| nemotron-bolt | paraphrase | moderate | min ≥ 0.6 | 98 | 99% | 100% | 0.84 (0.66–0.96) | 0.57 (0.40–0.75) | 31% | 30% | 16% of 97 | 2.04 | 1.8 |
| nemotron-bolt | paraphrase | moderate | min ≥ 0.75 | 98 | 89% | 92% | 0.87 (0.76–0.96) | 0.58 (0.45–0.75) | 33% | 31% | 14% of 87 | 2.41 | 2.2 |
| nemotron-bolt | paraphrase | strong | min ≥ 0.05 (E4-3) | 98 | 100% | 100% | 0.87 (0.70–0.96) | 0.61 (0.38–0.75) | 27% | 25% | 12% of 98 | 2.0 | 1.9 |
| nemotron-bolt | paraphrase | strong | max ≥ 0.05 | 98 | 100% | 100% | 0.92 (0.74–1.00) | 0.65 (0.45–0.80) | 19% | 19% | 11% of 98 | 2.0 | 1.9 |
| nemotron-bolt | paraphrase | strong | max ≥ 0.2 (E4-7) | 98 | 100% | 100% | 0.92 (0.74–1.00) | 0.65 (0.45–0.80) | 19% | 19% | 11% of 98 | 2.0 | 1.9 |
| nemotron-bolt | paraphrase | strong | min ≥ 0.2 | 98 | 100% | 100% | 0.87 (0.70–0.96) | 0.61 (0.38–0.75) | 27% | 25% | 12% of 98 | 2.0 | 1.9 |
| nemotron-bolt | paraphrase | strong | min ≥ 0.3 | 98 | 100% | 100% | 0.87 (0.70–0.96) | 0.61 (0.38–0.75) | 27% | 25% | 12% of 98 | 2.0 | 1.9 |
| nemotron-bolt | paraphrase | strong | min ≥ 0.4 | 98 | 100% | 100% | 0.87 (0.70–0.96) | 0.61 (0.38–0.75) | 27% | 25% | 12% of 98 | 2.0 | 1.9 |
| nemotron-bolt | paraphrase | strong | min ≥ 0.6 | 98 | 99% | 100% | 0.87 (0.71–0.96) | 0.61 (0.40–0.75) | 26% | 25% | 12% of 97 | 2.02 | 1.9 |
| nemotron-bolt | paraphrase | strong | min ≥ 0.75 | 98 | 92% | 96% | 0.88 (0.78–0.96) | 0.64 (0.48–0.75) | 28% | 26% | 14% of 90 | 2.22 | 2.0 |

### Selection policies, CPU 1 × 2 (D61) — every language

| model | tactic | intensity | policy | chunks | rewritten | rewritten, by words | divergence med (p10–p90) | word change med (p10–p90) | pairs carried over, mean (all chunks) | pairs carried over, by words | judged CHANGED | calls / chunk | s / chunk |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| gemma4-12b | paraphrase | light | min ≥ 0.05 (E4-3) | 98 | 100% | 100% | 0.82 (0.60–0.95) | 0.54 (0.27–0.74) | 31% | 34% | 6% of 98 | 1.06 | 1.4 |
| gemma4-12b | paraphrase | light | max ≥ 0.05 | 98 | 100% | 100% | 0.82 (0.60–0.95) | 0.54 (0.27–0.74) | 31% | 34% | 6% of 98 | 1.06 | 1.4 |
| gemma4-12b | paraphrase | light | max ≥ 0.2 (E4-7) | 98 | 100% | 100% | 0.82 (0.61–0.95) | 0.54 (0.28–0.74) | 31% | 34% | 7% of 98 | 1.07 | 1.4 |
| gemma4-12b | paraphrase | light | min ≥ 0.2 | 98 | 100% | 100% | 0.82 (0.61–0.95) | 0.54 (0.28–0.74) | 31% | 34% | 7% of 98 | 1.07 | 1.4 |
| gemma4-12b | paraphrase | light | min ≥ 0.3 | 98 | 100% | 100% | 0.82 (0.61–0.95) | 0.54 (0.28–0.74) | 31% | 34% | 7% of 98 | 1.07 | 1.4 |
| gemma4-12b | paraphrase | light | min ≥ 0.4 | 98 | 100% | 100% | 0.82 (0.61–0.95) | 0.54 (0.28–0.74) | 31% | 34% | 7% of 98 | 1.07 | 1.4 |
| gemma4-12b | paraphrase | light | min ≥ 0.6 | 98 | 97% | 94% | 0.84 (0.67–0.97) | 0.55 (0.33–0.75) | 30% | 34% | 8% of 95 | 1.16 | 1.6 |
| gemma4-12b | paraphrase | light | min ≥ 0.75 | 98 | 76% | 71% | 0.88 (0.77–0.99) | 0.59 (0.43–0.78) | 41% | 45% | 10% of 74 | 1.35 | 1.8 |
| gemma4-12b | paraphrase | moderate | min ≥ 0.05 (E4-3) | 98 | 97% | 100% | 0.91 (0.75–0.99) | 0.68 (0.54–0.81) | 20% | 18% | 7% of 95 | 1.04 | 1.4 |
| gemma4-12b | paraphrase | moderate | max ≥ 0.05 | 98 | 97% | 100% | 0.91 (0.75–0.99) | 0.68 (0.54–0.81) | 20% | 18% | 7% of 95 | 1.04 | 1.4 |
| gemma4-12b | paraphrase | moderate | max ≥ 0.2 (E4-7) | 98 | 97% | 100% | 0.91 (0.75–0.99) | 0.68 (0.54–0.81) | 20% | 18% | 7% of 95 | 1.04 | 1.4 |
| gemma4-12b | paraphrase | moderate | min ≥ 0.2 | 98 | 97% | 100% | 0.91 (0.75–0.99) | 0.68 (0.54–0.81) | 20% | 18% | 7% of 95 | 1.04 | 1.4 |
| gemma4-12b | paraphrase | moderate | min ≥ 0.3 | 98 | 97% | 100% | 0.91 (0.75–0.99) | 0.68 (0.54–0.81) | 20% | 18% | 7% of 95 | 1.04 | 1.4 |
| gemma4-12b | paraphrase | moderate | min ≥ 0.4 | 98 | 97% | 100% | 0.91 (0.75–0.99) | 0.68 (0.54–0.81) | 20% | 18% | 7% of 95 | 1.04 | 1.4 |
| gemma4-12b | paraphrase | moderate | min ≥ 0.6 | 98 | 97% | 100% | 0.91 (0.75–0.99) | 0.68 (0.54–0.81) | 20% | 18% | 7% of 95 | 1.04 | 1.4 |
| gemma4-12b | paraphrase | moderate | min ≥ 0.75 | 98 | 92% | 96% | 0.92 (0.78–0.99) | 0.68 (0.56–0.81) | 23% | 20% | 8% of 90 | 1.13 | 1.5 |
| gemma4-12b | paraphrase | strong | min ≥ 0.05 (E4-3) | 98 | 97% | 100% | 0.93 (0.79–1.00) | 0.73 (0.59–0.88) | 18% | 16% | 10% of 95 | 1.06 | 1.4 |
| gemma4-12b | paraphrase | strong | max ≥ 0.05 | 98 | 97% | 100% | 0.93 (0.79–1.00) | 0.73 (0.59–0.88) | 18% | 16% | 10% of 95 | 1.06 | 1.4 |
| gemma4-12b | paraphrase | strong | max ≥ 0.2 (E4-7) | 98 | 97% | 100% | 0.93 (0.79–1.00) | 0.73 (0.59–0.88) | 18% | 16% | 10% of 95 | 1.06 | 1.4 |
| gemma4-12b | paraphrase | strong | min ≥ 0.2 | 98 | 97% | 100% | 0.93 (0.79–1.00) | 0.73 (0.59–0.88) | 18% | 16% | 10% of 95 | 1.06 | 1.4 |
| gemma4-12b | paraphrase | strong | min ≥ 0.3 | 98 | 97% | 100% | 0.93 (0.79–1.00) | 0.73 (0.59–0.88) | 18% | 16% | 10% of 95 | 1.06 | 1.4 |
| gemma4-12b | paraphrase | strong | min ≥ 0.4 | 98 | 97% | 100% | 0.93 (0.79–1.00) | 0.73 (0.59–0.88) | 18% | 16% | 10% of 95 | 1.06 | 1.4 |
| gemma4-12b | paraphrase | strong | min ≥ 0.6 | 98 | 97% | 100% | 0.93 (0.79–1.00) | 0.73 (0.59–0.88) | 18% | 16% | 10% of 95 | 1.06 | 1.4 |
| gemma4-12b | paraphrase | strong | min ≥ 0.75 | 98 | 92% | 94% | 0.94 (0.82–1.00) | 0.73 (0.61–0.88) | 20% | 18% | 11% of 90 | 1.15 | 1.5 |
| gemma4-12b | humanize | moderate | min ≥ 0.05 (E4-3) | 98 | 97% | 99% | 0.80 (0.60–0.93) | 0.48 (0.23–0.71) | 37% | 38% | 15% of 95 | 1.04 | 1.2 |
| gemma4-12b | humanize | moderate | max ≥ 0.05 | 98 | 97% | 99% | 0.80 (0.60–0.93) | 0.48 (0.23–0.71) | 37% | 38% | 15% of 95 | 1.04 | 1.2 |
| gemma4-12b | humanize | moderate | max ≥ 0.2 (E4-7) | 98 | 97% | 99% | 0.80 (0.60–0.93) | 0.48 (0.23–0.71) | 37% | 38% | 15% of 95 | 1.04 | 1.2 |
| gemma4-12b | humanize | moderate | min ≥ 0.2 | 98 | 97% | 99% | 0.80 (0.60–0.93) | 0.48 (0.23–0.71) | 37% | 38% | 15% of 95 | 1.04 | 1.2 |
| gemma4-12b | humanize | moderate | min ≥ 0.3 | 98 | 97% | 99% | 0.80 (0.60–0.93) | 0.48 (0.23–0.71) | 37% | 38% | 15% of 95 | 1.04 | 1.2 |
| gemma4-12b | humanize | moderate | min ≥ 0.4 | 98 | 97% | 99% | 0.80 (0.60–0.93) | 0.48 (0.26–0.71) | 37% | 38% | 15% of 95 | 1.05 | 1.2 |
| gemma4-12b | humanize | moderate | min ≥ 0.6 | 98 | 94% | 95% | 0.80 (0.64–0.93) | 0.50 (0.29–0.71) | 37% | 38% | 15% of 92 | 1.13 | 1.4 |
| gemma4-12b | humanize | moderate | min ≥ 0.75 | 98 | 68% | 69% | 0.86 (0.76–0.95) | 0.54 (0.38–0.78) | 49% | 51% | 18% of 67 | 1.39 | 1.7 |
| gemma4-12b | humanize | strong | min ≥ 0.05 (E4-3) | 98 | 99% | 99% | 0.87 (0.71–0.96) | 0.57 (0.38–0.75) | 26% | 26% | 16% of 97 | 1.03 | 1.3 |
| gemma4-12b | humanize | strong | max ≥ 0.05 | 98 | 99% | 99% | 0.87 (0.71–0.96) | 0.57 (0.38–0.75) | 26% | 26% | 16% of 97 | 1.03 | 1.3 |
| gemma4-12b | humanize | strong | max ≥ 0.2 (E4-7) | 98 | 99% | 99% | 0.87 (0.71–0.96) | 0.57 (0.38–0.75) | 26% | 26% | 16% of 97 | 1.03 | 1.3 |
| gemma4-12b | humanize | strong | min ≥ 0.2 | 98 | 99% | 99% | 0.87 (0.71–0.96) | 0.57 (0.38–0.75) | 26% | 26% | 16% of 97 | 1.03 | 1.3 |
| gemma4-12b | humanize | strong | min ≥ 0.3 | 98 | 99% | 99% | 0.87 (0.71–0.96) | 0.57 (0.38–0.75) | 26% | 26% | 16% of 97 | 1.03 | 1.3 |
| gemma4-12b | humanize | strong | min ≥ 0.4 | 98 | 99% | 99% | 0.87 (0.71–0.96) | 0.57 (0.38–0.75) | 26% | 26% | 16% of 97 | 1.03 | 1.3 |
| gemma4-12b | humanize | strong | min ≥ 0.6 | 98 | 98% | 98% | 0.88 (0.71–0.98) | 0.57 (0.38–0.76) | 26% | 27% | 16% of 96 | 1.03 | 1.3 |
| gemma4-12b | humanize | strong | min ≥ 0.75 | 98 | 87% | 89% | 0.90 (0.79–1.00) | 0.59 (0.41–0.78) | 31% | 31% | 16% of 85 | 1.21 | 1.5 |
| nemotron-bolt | paraphrase | light | min ≥ 0.05 (E4-3) | 98 | 100% | 100% | 0.80 (0.58–0.95) | 0.50 (0.27–0.73) | 36% | 37% | 10% of 98 | 1.01 | 0.9 |
| nemotron-bolt | paraphrase | light | max ≥ 0.05 | 98 | 100% | 100% | 0.80 (0.58–0.95) | 0.50 (0.27–0.73) | 36% | 37% | 10% of 98 | 1.01 | 0.9 |
| nemotron-bolt | paraphrase | light | max ≥ 0.2 (E4-7) | 98 | 100% | 100% | 0.80 (0.58–0.95) | 0.50 (0.27–0.73) | 36% | 37% | 10% of 98 | 1.01 | 0.9 |
| nemotron-bolt | paraphrase | light | min ≥ 0.2 | 98 | 100% | 100% | 0.80 (0.58–0.95) | 0.50 (0.27–0.73) | 36% | 37% | 10% of 98 | 1.01 | 0.9 |
| nemotron-bolt | paraphrase | light | min ≥ 0.3 | 98 | 100% | 100% | 0.80 (0.58–0.95) | 0.50 (0.27–0.73) | 36% | 37% | 10% of 98 | 1.01 | 0.9 |
| nemotron-bolt | paraphrase | light | min ≥ 0.4 | 98 | 100% | 100% | 0.80 (0.60–0.95) | 0.50 (0.28–0.73) | 35% | 37% | 10% of 98 | 1.03 | 1.0 |
| nemotron-bolt | paraphrase | light | min ≥ 0.6 | 98 | 94% | 96% | 0.81 (0.63–0.96) | 0.50 (0.32–0.74) | 36% | 37% | 11% of 92 | 1.13 | 1.1 |
| nemotron-bolt | paraphrase | light | min ≥ 0.75 | 98 | 69% | 69% | 0.88 (0.77–0.97) | 0.56 (0.43–0.75) | 47% | 50% | 9% of 68 | 1.38 | 1.3 |
| nemotron-bolt | paraphrase | moderate | min ≥ 0.05 (E4-3) | 98 | 100% | 100% | 0.85 (0.67–0.96) | 0.58 (0.40–0.75) | 29% | 27% | 13% of 98 | 1.0 | 0.9 |
| nemotron-bolt | paraphrase | moderate | max ≥ 0.05 | 98 | 100% | 100% | 0.85 (0.67–0.96) | 0.58 (0.40–0.75) | 29% | 27% | 13% of 98 | 1.0 | 0.9 |
| nemotron-bolt | paraphrase | moderate | max ≥ 0.2 (E4-7) | 98 | 100% | 100% | 0.85 (0.67–0.96) | 0.58 (0.40–0.75) | 29% | 27% | 13% of 98 | 1.0 | 0.9 |
| nemotron-bolt | paraphrase | moderate | min ≥ 0.2 | 98 | 100% | 100% | 0.85 (0.67–0.96) | 0.58 (0.40–0.75) | 29% | 27% | 13% of 98 | 1.0 | 0.9 |
| nemotron-bolt | paraphrase | moderate | min ≥ 0.3 | 98 | 100% | 100% | 0.85 (0.67–0.96) | 0.58 (0.40–0.75) | 29% | 27% | 13% of 98 | 1.0 | 0.9 |
| nemotron-bolt | paraphrase | moderate | min ≥ 0.4 | 98 | 100% | 100% | 0.85 (0.67–0.96) | 0.58 (0.40–0.75) | 29% | 27% | 13% of 98 | 1.0 | 0.9 |
| nemotron-bolt | paraphrase | moderate | min ≥ 0.6 | 98 | 98% | 100% | 0.86 (0.67–0.97) | 0.59 (0.41–0.75) | 29% | 27% | 15% of 96 | 1.04 | 0.9 |
| nemotron-bolt | paraphrase | moderate | min ≥ 0.75 | 98 | 80% | 83% | 0.89 (0.78–0.97) | 0.62 (0.46–0.75) | 38% | 36% | 13% of 78 | 1.27 | 1.1 |
| nemotron-bolt | paraphrase | strong | min ≥ 0.05 (E4-3) | 98 | 100% | 100% | 0.89 (0.72–0.99) | 0.64 (0.40–0.77) | 23% | 23% | 14% of 98 | 1.01 | 0.9 |
| nemotron-bolt | paraphrase | strong | max ≥ 0.05 | 98 | 100% | 100% | 0.89 (0.72–0.99) | 0.64 (0.40–0.77) | 23% | 23% | 14% of 98 | 1.01 | 0.9 |
| nemotron-bolt | paraphrase | strong | max ≥ 0.2 (E4-7) | 98 | 100% | 100% | 0.89 (0.72–0.99) | 0.64 (0.40–0.77) | 23% | 23% | 14% of 98 | 1.01 | 0.9 |
| nemotron-bolt | paraphrase | strong | min ≥ 0.2 | 98 | 100% | 100% | 0.89 (0.72–0.99) | 0.64 (0.40–0.77) | 23% | 23% | 14% of 98 | 1.01 | 0.9 |
| nemotron-bolt | paraphrase | strong | min ≥ 0.3 | 98 | 100% | 100% | 0.89 (0.72–0.99) | 0.64 (0.40–0.77) | 23% | 23% | 14% of 98 | 1.01 | 0.9 |
| nemotron-bolt | paraphrase | strong | min ≥ 0.4 | 98 | 100% | 100% | 0.89 (0.72–0.99) | 0.64 (0.40–0.77) | 23% | 23% | 14% of 98 | 1.01 | 0.9 |
| nemotron-bolt | paraphrase | strong | min ≥ 0.6 | 98 | 99% | 100% | 0.89 (0.72–0.99) | 0.64 (0.41–0.77) | 24% | 23% | 14% of 97 | 1.03 | 0.9 |
| nemotron-bolt | paraphrase | strong | min ≥ 0.75 | 98 | 89% | 92% | 0.89 (0.79–1.00) | 0.64 (0.49–0.77) | 28% | 27% | 15% of 87 | 1.15 | 1.1 |
| nemotron-bolt | humanize | moderate | min ≥ 0.05 (E4-3) | 98 | 98% | 100% | 0.78 (0.54–0.92) | 0.33 (0.17–0.57) | 43% | 40% | 21% of 96 | 1.03 | 1.0 |
| nemotron-bolt | humanize | moderate | max ≥ 0.05 | 98 | 98% | 100% | 0.78 (0.54–0.92) | 0.33 (0.17–0.57) | 43% | 40% | 21% of 96 | 1.03 | 1.0 |
| nemotron-bolt | humanize | moderate | max ≥ 0.2 (E4-7) | 98 | 98% | 100% | 0.78 (0.54–0.92) | 0.33 (0.17–0.57) | 43% | 40% | 21% of 96 | 1.03 | 1.0 |
| nemotron-bolt | humanize | moderate | min ≥ 0.2 | 98 | 98% | 100% | 0.78 (0.54–0.92) | 0.33 (0.17–0.57) | 43% | 40% | 21% of 96 | 1.03 | 1.0 |
| nemotron-bolt | humanize | moderate | min ≥ 0.3 | 98 | 98% | 100% | 0.78 (0.54–0.92) | 0.33 (0.17–0.57) | 43% | 40% | 21% of 96 | 1.03 | 1.0 |
| nemotron-bolt | humanize | moderate | min ≥ 0.4 | 98 | 98% | 100% | 0.78 (0.57–0.92) | 0.33 (0.18–0.57) | 42% | 40% | 22% of 96 | 1.07 | 1.0 |
| nemotron-bolt | humanize | moderate | min ≥ 0.6 | 98 | 91% | 96% | 0.80 (0.64–0.92) | 0.36 (0.24–0.57) | 43% | 41% | 21% of 89 | 1.16 | 1.1 |
| nemotron-bolt | humanize | moderate | min ≥ 0.75 | 98 | 68% | 74% | 0.83 (0.77–0.93) | 0.42 (0.31–0.59) | 52% | 52% | 21% of 67 | 1.45 | 1.3 |
| nemotron-bolt | humanize | strong | min ≥ 0.05 (E4-3) | 98 | 99% | 99% | 0.80 (0.56–0.92) | 0.40 (0.20–0.60) | 39% | 37% | 16% of 97 | 1.07 | 1.1 |
| nemotron-bolt | humanize | strong | max ≥ 0.05 | 98 | 99% | 99% | 0.80 (0.56–0.92) | 0.40 (0.20–0.60) | 39% | 37% | 16% of 97 | 1.07 | 1.1 |
| nemotron-bolt | humanize | strong | max ≥ 0.2 (E4-7) | 98 | 99% | 99% | 0.80 (0.56–0.92) | 0.40 (0.20–0.60) | 39% | 37% | 16% of 97 | 1.07 | 1.1 |
| nemotron-bolt | humanize | strong | min ≥ 0.2 | 98 | 99% | 99% | 0.80 (0.56–0.92) | 0.40 (0.20–0.60) | 39% | 37% | 16% of 97 | 1.07 | 1.1 |
| nemotron-bolt | humanize | strong | min ≥ 0.3 | 98 | 99% | 99% | 0.80 (0.56–0.92) | 0.40 (0.20–0.60) | 39% | 37% | 16% of 97 | 1.07 | 1.1 |
| nemotron-bolt | humanize | strong | min ≥ 0.4 | 98 | 97% | 98% | 0.81 (0.57–0.92) | 0.41 (0.21–0.61) | 38% | 37% | 17% of 95 | 1.1 | 1.1 |
| nemotron-bolt | humanize | strong | min ≥ 0.6 | 98 | 92% | 95% | 0.82 (0.67–0.92) | 0.43 (0.27–0.61) | 39% | 38% | 18% of 90 | 1.2 | 1.2 |
| nemotron-bolt | humanize | strong | min ≥ 0.75 | 98 | 76% | 81% | 0.85 (0.75–0.93) | 0.47 (0.33–0.67) | 45% | 46% | 17% of 75 | 1.35 | 1.3 |

### Paragraphs kept as they were — GPU 2 × 2, `paraphrase` moderate, the loop's policy (E4-3 before, E4-7 after)

| model | policy | chunks | kept as they were | by kind | why their attempts failed |
|---|---|---|---|---|---|
| gemma4-12b | min ≥ 0.05 (E4-3) | 98 | 1 (1%) | markdown 1 | length-drift 4 |
| gemma4-12b | max ≥ 0.2 (E4-7) | 98 | 1 (1%) | markdown 1 | length-drift 4 |
| nemotron-bolt | min ≥ 0.05 (E4-3) | 98 | 0 (0%) |  |  |
| nemotron-bolt | max ≥ 0.2 (E4-7) | 98 | 0 (0%) |  |  |

### Selection policies per language — GPU 2 × 2, `paraphrase`

| model | intensity | lang | policy | rewritten | divergence med | word change med | judged CHANGED |
|---|---|---|---|---|---|---|---|
| gemma4-12b | moderate | en | min ≥ 0.05 (E4-3) | 100% | 0.91 | 0.67 | 10% of 30 |
| gemma4-12b | moderate | ru | min ≥ 0.05 (E4-3) | 100% | 0.93 | 0.71 | 12% of 34 |
| gemma4-12b | moderate | de | min ≥ 0.05 (E4-3) | 97% | 0.88 | 0.61 | 3% of 33 |
| gemma4-12b | moderate | en | max ≥ 0.2 (E4-7) | 100% | 0.94 | 0.70 | 7% of 30 |
| gemma4-12b | moderate | ru | max ≥ 0.2 (E4-7) | 100% | 0.97 | 0.78 | 9% of 34 |
| gemma4-12b | moderate | de | max ≥ 0.2 (E4-7) | 97% | 0.91 | 0.67 | 6% of 33 |
| gemma4-12b | moderate | en | min ≥ 0.2 | 100% | 0.91 | 0.67 | 10% of 30 |
| gemma4-12b | moderate | ru | min ≥ 0.2 | 100% | 0.93 | 0.71 | 12% of 34 |
| gemma4-12b | moderate | de | min ≥ 0.2 | 97% | 0.88 | 0.61 | 3% of 33 |
| gemma4-12b | moderate | en | min ≥ 0.6 | 100% | 0.91 | 0.67 | 10% of 30 |
| gemma4-12b | moderate | ru | min ≥ 0.6 | 100% | 0.93 | 0.71 | 12% of 34 |
| gemma4-12b | moderate | de | min ≥ 0.6 | 97% | 0.88 | 0.61 | 3% of 33 |
| gemma4-12b | strong | en | min ≥ 0.05 (E4-3) | 97% | 0.91 | 0.69 | 7% of 29 |
| gemma4-12b | strong | ru | min ≥ 0.05 (E4-3) | 100% | 0.94 | 0.76 | 12% of 34 |
| gemma4-12b | strong | de | min ≥ 0.05 (E4-3) | 97% | 0.90 | 0.63 | 6% of 33 |
| gemma4-12b | strong | en | max ≥ 0.2 (E4-7) | 97% | 0.95 | 0.73 | 7% of 29 |
| gemma4-12b | strong | ru | max ≥ 0.2 (E4-7) | 100% | 0.97 | 0.78 | 15% of 34 |
| gemma4-12b | strong | de | max ≥ 0.2 (E4-7) | 97% | 0.93 | 0.71 | 9% of 33 |
| gemma4-12b | strong | en | min ≥ 0.2 | 97% | 0.91 | 0.69 | 7% of 29 |
| gemma4-12b | strong | ru | min ≥ 0.2 | 100% | 0.94 | 0.76 | 12% of 34 |
| gemma4-12b | strong | de | min ≥ 0.2 | 97% | 0.90 | 0.63 | 6% of 33 |
| gemma4-12b | strong | en | min ≥ 0.6 | 97% | 0.91 | 0.69 | 7% of 29 |
| gemma4-12b | strong | ru | min ≥ 0.6 | 100% | 0.94 | 0.76 | 12% of 34 |
| gemma4-12b | strong | de | min ≥ 0.6 | 97% | 0.90 | 0.63 | 6% of 33 |
| nemotron-bolt | moderate | en | min ≥ 0.05 (E4-3) | 100% | 0.87 | 0.67 | 17% of 30 |
| nemotron-bolt | moderate | ru | min ≥ 0.05 (E4-3) | 100% | 0.88 | 0.57 | 12% of 34 |
| nemotron-bolt | moderate | de | min ≥ 0.05 (E4-3) | 100% | 0.74 | 0.48 | 12% of 34 |
| nemotron-bolt | moderate | en | max ≥ 0.2 (E4-7) | 100% | 0.93 | 0.71 | 10% of 30 |
| nemotron-bolt | moderate | ru | max ≥ 0.2 (E4-7) | 100% | 0.90 | 0.59 | 9% of 34 |
| nemotron-bolt | moderate | de | max ≥ 0.2 (E4-7) | 100% | 0.81 | 0.51 | 21% of 34 |
| nemotron-bolt | moderate | en | min ≥ 0.2 | 100% | 0.87 | 0.67 | 17% of 30 |
| nemotron-bolt | moderate | ru | min ≥ 0.2 | 100% | 0.88 | 0.57 | 12% of 34 |
| nemotron-bolt | moderate | de | min ≥ 0.2 | 100% | 0.74 | 0.48 | 12% of 34 |
| nemotron-bolt | moderate | en | min ≥ 0.6 | 100% | 0.87 | 0.67 | 20% of 30 |
| nemotron-bolt | moderate | ru | min ≥ 0.6 | 100% | 0.88 | 0.57 | 12% of 34 |
| nemotron-bolt | moderate | de | min ≥ 0.6 | 97% | 0.74 | 0.51 | 15% of 33 |
| nemotron-bolt | strong | en | min ≥ 0.05 (E4-3) | 100% | 0.89 | 0.64 | 13% of 30 |
| nemotron-bolt | strong | ru | min ≥ 0.05 (E4-3) | 100% | 0.88 | 0.59 | 9% of 34 |
| nemotron-bolt | strong | de | min ≥ 0.05 (E4-3) | 100% | 0.83 | 0.58 | 15% of 34 |
| nemotron-bolt | strong | en | max ≥ 0.2 (E4-7) | 100% | 0.93 | 0.73 | 7% of 30 |
| nemotron-bolt | strong | ru | max ≥ 0.2 (E4-7) | 100% | 0.93 | 0.65 | 9% of 34 |
| nemotron-bolt | strong | de | max ≥ 0.2 (E4-7) | 100% | 0.89 | 0.60 | 18% of 34 |
| nemotron-bolt | strong | en | min ≥ 0.2 | 100% | 0.89 | 0.64 | 13% of 30 |
| nemotron-bolt | strong | ru | min ≥ 0.2 | 100% | 0.88 | 0.59 | 9% of 34 |
| nemotron-bolt | strong | de | min ≥ 0.2 | 100% | 0.83 | 0.58 | 15% of 34 |
| nemotron-bolt | strong | en | min ≥ 0.6 | 100% | 0.89 | 0.64 | 13% of 30 |
| nemotron-bolt | strong | ru | min ≥ 0.6 | 100% | 0.88 | 0.59 | 9% of 34 |
| nemotron-bolt | strong | de | min ≥ 0.6 | 97% | 0.83 | 0.58 | 15% of 33 |

### Voice — the loop's pick (E4-7) beside the least diverged; every language

GPU 2 × 2 where the run made four candidates a chunk, CPU 1 × 2 where it made two. Second and first person are pronouns counted, kept by count; a chunk kept as it was keeps its voice. The register shift is a proxy: the share of the answer's words that a list of formal words and suffixes matches and the source has no word for (`dev/architecture/prompt-bench.md`, "Voice").

| model | tactic | intensity | executor | policy | chunks | with 2nd person | 2nd person kept, by count | per chunk, med [q1–q3] | lost any / lost all | ты↔вы, du↔Sie switched | 1st person kept | words ×, all · per chunk med [q1–q3] | register shift med [q1–q3] · mean | voice judged YES / PARTLY / NO |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| gemma4-12b | paraphrase | light | GPU 2 × 2 | max ≥ 0.2 (E4-7) | 98 | 33 | 87% | 1.00 [1.00–1.00] | 21% / 6% | 0 of 22 | 95% | 1.06 · 1.09 [1.01–1.17] | 0.02 [0.00–0.05] · 0.037 | 61% / 6% / 33% of 98 |
| gemma4-12b | paraphrase | light | GPU 2 × 2 | min ≥ 0.2 | 98 | 33 | 85% | 1.00 [0.89–1.00] | 27% / 9% | 0 of 22 | 97% | 1.05 · 1.05 [1.00–1.17] | 0.02 [0.00–0.05] · 0.031 | 74% / 2% / 23% of 98 |
| gemma4-12b | paraphrase | moderate | GPU 2 × 2 | max ≥ 0.2 (E4-7) | 98 | 33 | 64% | 0.80 [0.33–1.00] | 55% / 21% | 0 of 22 | 95% | 1.12 · 1.17 [1.07–1.33] | 0.04 [0.01–0.08] · 0.049 | 36% / 1% / 63% of 97 |
| gemma4-12b | paraphrase | moderate | GPU 2 × 2 | min ≥ 0.2 | 98 | 33 | 75% | 1.00 [0.67–1.00] | 36% / 12% | 0 of 22 | 93% | 1.09 · 1.15 [1.02–1.24] | 0.03 [0.01–0.07] · 0.048 | 51% / 3% / 46% of 97 |
| gemma4-12b | paraphrase | strong | GPU 2 × 2 | max ≥ 0.2 (E4-7) | 98 | 33 | 65% | 0.75 [0.50–1.00] | 55% / 15% | 0 of 22 | 86% | 1.13 · 1.17 [1.06–1.33] | 0.04 [0.02–0.08] · 0.057 | 28% / 6% / 66% of 96 |
| gemma4-12b | paraphrase | strong | GPU 2 × 2 | min ≥ 0.2 | 98 | 33 | 70% | 1.00 [0.50–1.00] | 48% / 15% | 0 of 22 | 88% | 1.13 · 1.16 [1.07–1.34] | 0.04 [0.01–0.07] · 0.052 | 38% / 3% / 59% of 96 |
| gemma4-12b | humanize | moderate | CPU 1 × 2 | max ≥ 0.2 (E4-7) | 98 | 33 | 82% | 1.00 [0.67–1.00] | 36% / 12% | 0 of 22 | 95% | 0.93 · 1.00 [0.85–1.09] | 0.00 [0.00–0.00] · 0.004 | 78% / 4% / 18% of 95 |
| gemma4-12b | humanize | moderate | CPU 1 × 2 | min ≥ 0.2 | 98 | 33 | 82% | 1.00 [0.67–1.00] | 36% / 12% | 0 of 22 | 95% | 0.93 · 1.00 [0.85–1.09] | 0.00 [0.00–0.00] · 0.004 | 78% / 4% / 18% of 95 |
| gemma4-12b | humanize | strong | CPU 1 × 2 | max ≥ 0.2 (E4-7) | 98 | 33 | 79% | 1.00 [0.50–1.00] | 36% / 12% | 0 of 22 | 92% | 0.96 · 1.00 [0.88–1.17] | 0.00 [0.00–0.01] · 0.008 | 73% / 6% / 21% of 97 |
| gemma4-12b | humanize | strong | CPU 1 × 2 | min ≥ 0.2 | 98 | 33 | 79% | 1.00 [0.50–1.00] | 36% / 12% | 0 of 22 | 92% | 0.96 · 1.00 [0.88–1.17] | 0.00 [0.00–0.01] · 0.008 | 73% / 6% / 21% of 97 |
| nemotron-bolt | paraphrase | light | GPU 2 × 2 | max ≥ 0.2 (E4-7) | 98 | 33 | 74% | 0.89 [0.60–1.00] | 52% / 15% | 0 of 22 | 88% | 0.93 · 0.96 [0.90–1.03] | 0.01 [0.00–0.05] · 0.036 | 60% / 2% / 38% of 98 |
| nemotron-bolt | paraphrase | light | GPU 2 × 2 | min ≥ 0.2 | 98 | 33 | 75% | 1.00 [0.60–1.00] | 48% / 12% | 1 of 22 | 92% | 0.93 · 0.96 [0.90–1.00] | 0.00 [0.00–0.03] · 0.027 | 71% / 2% / 27% of 98 |
| nemotron-bolt | paraphrase | moderate | GPU 2 × 2 | max ≥ 0.2 (E4-7) | 98 | 33 | 62% | 0.67 [0.40–1.00] | 64% / 18% | 0 of 22 | 85% | 0.94 · 0.98 [0.89–1.05] | 0.03 [0.00–0.05] · 0.040 | 59% / 3% / 38% of 98 |
| nemotron-bolt | paraphrase | moderate | GPU 2 × 2 | min ≥ 0.2 | 98 | 33 | 65% | 0.67 [0.40–1.00] | 58% / 18% | 0 of 22 | 76% | 0.94 · 0.98 [0.90–1.04] | 0.02 [0.00–0.05] · 0.036 | 59% / 2% / 39% of 98 |
| nemotron-bolt | paraphrase | strong | GPU 2 × 2 | max ≥ 0.2 (E4-7) | 98 | 33 | 54% | 0.60 [0.20–1.00] | 70% / 24% | 0 of 22 | 76% | 0.93 · 0.96 [0.86–1.03] | 0.04 [0.00–0.08] · 0.058 | 45% / 2% / 53% of 98 |
| nemotron-bolt | paraphrase | strong | GPU 2 × 2 | min ≥ 0.2 | 98 | 33 | 50% | 0.50 [0.20–1.00] | 70% / 24% | 0 of 22 | 83% | 0.94 · 0.96 [0.88–1.05] | 0.03 [0.00–0.06] · 0.040 | 51% / 1% / 48% of 98 |
| nemotron-bolt | humanize | moderate | CPU 1 × 2 | max ≥ 0.2 (E4-7) | 98 | 33 | 66% | 0.83 [0.33–1.00] | 55% / 15% | 0 of 22 | 78% | 0.80 · 0.82 [0.75–0.95] | 0.00 [0.00–0.00] · 0.003 | 77% / 4% / 19% of 96 |
| nemotron-bolt | humanize | moderate | CPU 1 × 2 | min ≥ 0.2 | 98 | 33 | 66% | 0.83 [0.33–1.00] | 55% / 15% | 0 of 22 | 78% | 0.80 · 0.82 [0.75–0.95] | 0.00 [0.00–0.00] · 0.003 | 77% / 4% / 19% of 96 |
| nemotron-bolt | humanize | strong | CPU 1 × 2 | max ≥ 0.2 (E4-7) | 98 | 33 | 68% | 0.80 [0.33–1.00] | 61% / 18% | 0 of 22 | 70% | 0.81 · 0.86 [0.75–0.98] | 0.00 [0.00–0.00] · 0.003 | 75% / 9% / 15% of 97 |
| nemotron-bolt | humanize | strong | CPU 1 × 2 | min ≥ 0.2 | 98 | 33 | 68% | 0.80 [0.33–1.00] | 61% / 18% | 0 of 22 | 70% | 0.81 · 0.86 [0.75–0.98] | 0.00 [0.00–0.00] · 0.003 | 75% / 9% / 15% of 97 |

### The language check (candidates that passed, chunk language detected; the loop refuses 20+ words since E4-7)

| model | passed, chunk language known | answer in another language | answer language unknown | of those two, planted-instruction items |
|---|---|---|---|---|
| gemma4-12b | 1192 | 0 (0%) | 8 (1%) | 0 |
| nemotron-bolt | 1200 | 0 (0%) | 18 (2%) | 0 |

### The no-op floor and the length window (every tactic)

| model | candidates that passed all but the floor | divergence < 0.05 | < 0.10 | < 0.20 | length-guard rejections (0.6–1.6) | of them inside 0.5–2.0 | judged CHANGED: inside 0.6–1.6 | in 0.5–0.6 or 1.6–2.0 | outside 0.5–2.0 |
|---|---|---|---|---|---|---|---|---|---|
| gemma4-12b | 1514 | 1% | 1% | 1% | 40 | 4 | 10% of 1424 | 4% of 94 | 11% of 36 |
| nemotron-bolt | 1548 | 0% | 0% | 0% | 12 | 8 | 13% of 1539 | 29% of 17 | 0% of 4 |

### Judged CHANGED by divergence band (every judged candidate inside the length window)

| model | < 0.2 | 0.2–0.4 | 0.4–0.6 | 0.6–0.8 | ≥ 0.8 |
|---|---|---|---|---|---|
| gemma4-12b | 0% of 11 | 0% of 4 | 0% of 48 | 10% of 362 | 10% of 1089 |
| nemotron-bolt | 0% of 7 | 0% of 19 | 12% of 117 | 21% of 494 | 9% of 911 |

### The judge's calibration

| judge | case | expected | n | as expected |
|---|---|---|---|---|
| gemma4-12b-judge | same | EQUIVALENT | 59 | 100% |
| gemma4-12b-judge | dropped | CHANGED | 59 | 100% |
| gemma4-12b-judge | added | CHANGED | 59 | 100% |
| gemma4-12b-judge | same (voice) | YES | 59 | 100% |
| gemma4-12b-judge | switched (voice) | NO | 3 | 100% |
| gemma4-12b-judge | unparsable answers | – | 0 | – |

## Examples

### gemma4-12b · en · en-mx-02 chunk 0

**Source**

> Elevate your morning routine with the AuroraBrew Pro, a sleek and intuitive coffee maker designed for those who refuse to compromise on flavor. Its precision temperature control ensures that every cup is brewed at the optimal point, unlocking the full aroma of your favorite beans. Whether you prefer a bold espresso or a smooth pour-over, the AuroraBrew Pro adapts seamlessly to your taste, making it the perfect companion for busy mornings and relaxed weekends alike.

**min ≥ 0.05 (E4-3)** — divergence 0.94, word change 0.74, judge EQUIVALENT

> Designed for individuals who will not sacrifice taste, the AuroraBrew Pro is a sophisticated and user-friendly coffee machine that upgrades your early morning habits. The complete aroma of your preferred beans is released because its exact temperature regulation guarantees each brew reaches the ideal state. The AuroraBrew Pro conforms effortlessly to your specific preferences, whether you favor a mellow pour-over or a robust espresso, rendering it an ideal partner for both tranquil weekends and hectic mornings.

**min ≥ 0.6** — divergence 0.94, word change 0.74, judge EQUIVALENT

> Designed for individuals who will not sacrifice taste, the AuroraBrew Pro is a sophisticated and user-friendly coffee machine that upgrades your early morning habits. The complete aroma of your preferred beans is released because its exact temperature regulation guarantees each brew reaches the ideal state. The AuroraBrew Pro conforms effortlessly to your specific preferences, whether you favor a mellow pour-over or a robust espresso, rendering it an ideal partner for both tranquil weekends and hectic mornings.

**max ≥ 0.2 (E4-7)** — divergence 0.95, word change 0.65, judge EQUIVALENT

> Your morning ritual can be enhanced by the AuroraBrew Pro, an intuitive and stylish coffee machine crafted for individuals who will not sacrifice taste. Because of its precise temperature management, every single brew reaches the ideal heat to release the complete fragrance of your chosen beans. The AuroraBrew Pro adjusts effortlessly to suit your preferences—be it a mellow pour-over or a robust espresso—positioning it as an ideal partner for both hectic mornings and leisurely weekends.

### gemma4-12b · en · en-mx-05 chunk 0

**Source**

> The city council voted on Tuesday to approve a new plan aimed at reducing traffic congestion in the downtown area. The proposal includes expanding bike lanes, adjusting the timing of traffic signals, and introducing a pilot program for on-demand shuttle buses. Supporters argue that the measures will make the city center safer and more accessible, while critics have raised concerns about the cost and the potential impact on local businesses during construction.

**min ≥ 0.05 (E4-3)** — divergence 0.89, word change 0.60, judge EQUIVALENT

> A fresh strategy to alleviate downtown traffic jams received approval from the city council on Tuesday.
> By increasing the number of bike lanes, modifying signal timings, and launching a trial for shuttle buses on demand, the proposal seeks these goals.
> The city center will become safer and more accessible according to proponents; however, opponents have voiced worries regarding construction-related costs and the possible effects on local businesses.

**min ≥ 0.6** — divergence 0.89, word change 0.60, judge EQUIVALENT

> A fresh strategy to alleviate downtown traffic jams received approval from the city council on Tuesday.
> By increasing the number of bike lanes, modifying signal timings, and launching a trial for shuttle buses on demand, the proposal seeks these goals.
> The city center will become safer and more accessible according to proponents; however, opponents have voiced worries regarding construction-related costs and the possible effects on local businesses.

**max ≥ 0.2 (E4-7)** — divergence 0.92, word change 0.71, judge EQUIVALENT

> To decrease traffic build-up within the downtown district, a fresh strategy was sanctioned by the city council on Tuesday. Expanding lanes for bicycles, modifying the duration of traffic lights, and launching an on-demand shuttle bus pilot project are all components of this plan. That the city center will become more accessible and secure is the primary argument of advocates; however, detractors have voiced worries regarding construction's effect on neighborhood shops and the overall price tag.

### gemma4-12b · en · en-mx-08 chunk 0

**Source**

> Yes, you can change your subscription plan at any time from your account settings. If you upgrade, the new features become available immediately, and you will only be charged the difference for the remainder of the current billing period. If you downgrade, the change takes effect at the start of your next billing cycle, so you can continue to enjoy your current features until then. Please note that some data may become read-only if it exceeds the limits of the smaller plan.

**min ≥ 0.05 (E4-3)** — divergence 0.89, word change 0.69, judge EQUIVALENT

> Modification of your subscription plan is possible whenever you wish via the account settings. The remaining balance of your current billing period will only incur a charge for the price difference if you choose to upgrade, which grants instant access to new features. In the event that you opt for a downgrade, your current benefits remain active until the start of the subsequent billing cycle is reached. Be aware, however, that data might be restricted to read-only status should it surpass the capacity of the reduced plan.

**min ≥ 0.6** — divergence 0.89, word change 0.69, judge EQUIVALENT

> Modification of your subscription plan is possible whenever you wish via the account settings. The remaining balance of your current billing period will only incur a charge for the price difference if you choose to upgrade, which grants instant access to new features. In the event that you opt for a downgrade, your current benefits remain active until the start of the subsequent billing cycle is reached. Be aware, however, that data might be restricted to read-only status should it surpass the capacity of the reduced plan.

**max ≥ 0.2 (E4-7)** — divergence 0.90, word change 0.63, judge EQUIVALENT

> Modifying your subscription plan is possible whenever you wish by navigating to your account settings. When an upgrade occurs, immediate access to new features is granted, and the charge will only reflect the price difference for what remains of your existing billing period. In the event of a downgrade, your current features remain accessible until the next billing cycle begins, which is when the transition takes place. It should be noted, however, that certain data might be restricted to read-only status if it surpasses the capacity of the reduced plan.

### gemma4-12b · ru · ru-mx-03 chunk 0

**Source**

> В современном быстро меняющемся цифровом мире удалённая работа стала преобразующей силой, которая меняет подход организаций к своей деятельности. Она даёт сотрудникам беспрецедентную гибкость и избавляет от долгих поездок, однако вместе с тем создаёт и особые трудности, такие как поддержание сплочённости команды и предотвращение выгорания. В конечном счёте успех удалённой работы зависит от тонкого баланса между самостоятельностью и ответственностью.

**min ≥ 0.05 (E4-3)** — divergence 0.96, word change 0.77, judge CHANGED

> Подход организаций к их деятельности трансформируется под воздействием удалённой работы, которая превратилась в мощную силу в условиях нынешнего стремительно меняющегося цифрового мира. Сотрудникам она обеспечивает уникальную гибкость и освобождает от необходимости тратить время на поездки, но одновременно порождает специфические проблемы, включая риск выгорания и необходимость сохранения единства команды. Итоговый успех дистанционного формата тру slumped работы определяется тонким равновесием между ответственностью и самостоятельностью.

**min ≥ 0.6** — divergence 0.96, word change 0.77, judge CHANGED

> Подход организаций к их деятельности трансформируется под воздействием удалённой работы, которая превратилась в мощную силу в условиях нынешнего стремительно меняющегося цифрового мира. Сотрудникам она обеспечивает уникальную гибкость и освобождает от необходимости тратить время на поездки, но одновременно порождает специфические проблемы, включая риск выгорания и необходимость сохранения единства команды. Итоговый успех дистанционного формата тру slumped работы определяется тонким равновесием между ответственностью и самостоятельностью.

**max ≥ 0.2 (E4-7)** — divergence 0.98, word change 0.83, judge EQUIVALENT

> Организации полностью меняют свои методы деятельности благодаря удалённому формату работы, который в динамично развивающемся современном цифровом пространстве стал мощным фактором трансформации. Сотрудники получают возможность работать с небыwałой свободой и не тратят время на длительные поездки, но одновременно возникают специфические проблемы, включая предотвращение выгорания и сохранение единства коллектива. Баланс между ответственностью и автономностью в конечном итоге определяет эффективность дистанционного режима работы.

### gemma4-12b · ru · ru-mx-06 chunk 0

**Source**

> Пишу вам, чтобы выразить свою искреннюю заинтересованность в должности координатора по маркетингу в вашей компании. Обладая солидным опытом в цифровых кампаниях и страстью к сторителлингу, я уверена, что смогу стать ценным дополнением вашей команды. На предыдущем месте работы я успешно вела аккаунты в социальных сетях, взаимодействовала с разными отделами и помогла увеличить вовлечённость аудитории с помощью креативных и основанных на данных стратегий.

**min ≥ 0.05 (E4-3)** — divergence 0.95, word change 0.70, judge EQUIVALENT

> Вами я пользуюсь этим письмом для того, чтобы сообщить о своей серьезной заинтересованности в позиции координатора по маркетингу внутри вашей организации. Благодаря наличию значительного опыта реализации цифровых кампаний и любви к сторителлингу, у меня есть уверенность в возможности стать полезным участником вашего коллектива. На своем предыдущем рабочем месте я эффективно управляла профилями в социальных сетях, налаживала контакты с различными подразделениями и обеспечила рост вовлеченности пользователей через применение стратегий, базирующихся на данных и креативном подходе.

**min ≥ 0.6** — divergence 0.95, word change 0.70, judge EQUIVALENT

> Вами я пользуюсь этим письмом для того, чтобы сообщить о своей серьезной заинтересованности в позиции координатора по маркетингу внутри вашей организации. Благодаря наличию значительного опыта реализации цифровых кампаний и любви к сторителлингу, у меня есть уверенность в возможности стать полезным участником вашего коллектива. На своем предыдущем рабочем месте я эффективно управляла профилями в социальных сетях, налаживала контакты с различными подразделениями и обеспечила рост вовлеченности пользователей через применение стратегий, базирующихся на данных и креативном подходе.

**max ≥ 0.2 (E4-7)** — divergence 0.97, word change 0.79, judge EQUIVALENT

> Моей целью является подача заявки на позицию координатора по маркетингу в вашей организации, так как я искренне заинтересована в этой вакансии. Благодаря глубоким знаниям в области цифрового продвижения и любви к искусству сторителлинга, я убеждена в своей способности принести пользу коллективу. В рамках прошлой работы мне удавалось эффективно управлять профилями в соцсетях, координировать процессы с различными подразделениями и добиваться роста вовлеченности пользователей посредством внедрения инновационных стратегий, базирующихся на аналитике данных.

### gemma4-12b · ru · ru-mx-09 chunk 0

**Source**

> В целом результаты опроса свидетельствуют о высоком уровне удовлетворённости участников: большинство респондентов оценили семинар как полезный и хорошо организованный. Вместе с тем ряд слушателей отметил, что занятия проходили в спешке и что им не хватило времени на вопросы. В дальнейшем рекомендуется отводить больше времени на обсуждение и заранее предоставлять участникам дополнительные материалы.

**min ≥ 0.05 (E4-3)** — divergence 0.96, word change 0.76, judge EQUIVALENT

> Высокой степень удовлетворенности участников подтверждают результаты проведенного опроса: как полезным и качественно организованным большинство опрошенных сочли семинар. Однако некоторые слушатели указали на то, что в спешке проходили занятия, а времени на вопросы им оказалось недостаточно. Для будущих мероприятий целесообразно заранее передавать дополнительную информацию участникам и выделять больше времени для обсуждения тем.

**min ≥ 0.6** — divergence 0.96, word change 0.76, judge EQUIVALENT

> Высокой степень удовлетворенности участников подтверждают результаты проведенного опроса: как полезным и качественно организованным большинство опрошенных сочли семинар. Однако некоторые слушатели указали на то, что в спешке проходили занятия, а времени на вопросы им оказалось недостаточно. Для будущих мероприятий целесообразно заранее передавать дополнительную информацию участникам и выделять больше времени для обсуждения тем.

**max ≥ 0.2 (E4-7)** — divergence 0.97, word change 0.83, judge EQUIVALENT

> Свидетельствует о том, что значительная часть опрошенных осталась довольна организацией и полезностью семинара, общий уровень удовлетворенности участников в результатах исследования оказался высоким. Однако некоторые слушатели указали на недостаток времени для вопросов и высокую скорость проведения занятий. С целью улучшения рекомендуется участникам заранее передавать дополнительные материалы, а на обсуждение выделять больше времени в будущем.

### gemma4-12b · de · de-mx-01 chunk 0

**Source**

> Vielen Dank für Ihre Nachricht bezüglich der verspäteten Lieferung. Wir verstehen sehr gut, wie ärgerlich es ist, wenn eine Bestellung nicht rechtzeitig ankommt, insbesondere wenn Sie sie für ein bevorstehendes Projekt benötigen. Wir haben Ihre Bestellung überprüft und festgestellt, dass das Paket aufgrund eines unerwarteten Personalmangels in unserem regionalen Verteilzentrum aufgehalten wurde. Seien Sie versichert, dass es inzwischen versandt wurde und Sie innerhalb der nächsten zwei Werktage erreichen wird.

**min ≥ 0.05 (E4-3)** — divergence 0.88, word change 0.59, judge EQUIVALENT

> Herzlichen Dank für Ihre Mitteilung wegen der verspäteten Zustellung. Dass eine Bestellung nicht zum gewünschten Zeitpunkt eintrifft, ist besonders im Hinblick auf ein anstehendes Projekt sehr frustrierend, was wir vollkommen nachvollziehen können. Nach einer Prüfung Ihrer Bestellung haben wir festgestellt, dass das Paket wegen eines unvorhergesehenen Personalmangels in unserem regionalen Verteilzentrum liegen blieb. Dass es nun jedoch bereits versendet wurde und innerhalb der kommenden 2 Werktage bei Ihnen eintreffen wird, kann bestätigt werden.

**min ≥ 0.6** — divergence 0.88, word change 0.59, judge EQUIVALENT

> Herzlichen Dank für Ihre Mitteilung wegen der verspäteten Zustellung. Dass eine Bestellung nicht zum gewünschten Zeitpunkt eintrifft, ist besonders im Hinblick auf ein anstehendes Projekt sehr frustrierend, was wir vollkommen nachvollziehen können. Nach einer Prüfung Ihrer Bestellung haben wir festgestellt, dass das Paket wegen eines unvorhergesehenen Personalmangels in unserem regionalen Verteilzentrum liegen blieb. Dass es nun jedoch bereits versendet wurde und innerhalb der kommenden 2 Werktage bei Ihnen eintreffen wird, kann bestätigt werden.

**max ≥ 0.2 (E4-7)** — divergence 0.95, word change 0.74, judge EQUIVALENT

> Ihre Mitteilung zum verspäteten Lieferdatum haben wir erhalten und danken Ihnen dafür. Dass eine Bestellung nicht termingerecht eintrifft, ist besonders bei der Planung eines anstehenden Projekts sehr frustrierend, was wir vollkommen nachvollziehen können. Im Rahmen einer Prüfung Ihrer Bestellung wurde ersichtlich, dass das Paket wegen unvorhergesehener Engpässe bei den Mitarbeitern in unserem regionalen Verteilzentrum zurückgehalten wurde. Das Paket ist jedoch mittlerweile auf dem Weg zu Ihnen und wird die nächsten 2 Werktage nicht überschreiten.

### gemma4-12b · de · de-mx-04 chunk 0

**Source**

> Damit sich Ihre neue Zimmerpflanze wohlfühlt, stellen Sie sie an einen Ort mit hellem, indirektem Licht. Gießen Sie sie gründlich, sobald sich die oberste Erdschicht trocken anfühlt, und achten Sie darauf, dass der Topf Abzugslöcher hat, damit überschüssiges Wasser ablaufen kann. Während der Wachstumsphase düngen Sie einmal im Monat mit einem ausgewogenen Flüssigdünger. Wischen Sie außerdem alle paar Wochen die Blätter vorsichtig mit einem feuchten Tuch ab, um Staub zu entfernen.

**min ≥ 0.05 (E4-3)** — divergence 0.86, word change 0.59, judge EQUIVALENT

> Einen Standort mit hellem, indirektem Licht sollten Sie für das Wohlbefinden Ihrer neuen Zimmerpflanze wählen. Sobald die oberste Erdschicht trocken erscheint, ist ein gründliches Gießen notwendig; dabei muss gewährleistet sein, dass durch Abzugslöcher im Topf überschüssiges Wasser abfließen kann. Einmal monatlich sollte während der Wachstumsphase ein ausgewogener Flüssigdünger zur Düngung verwendet werden. Zudem empfiehlt es sich, die Blätter alle paar Wochen behutsam mit einem feuchten Tuch von Staub zu befreien.

**min ≥ 0.6** — divergence 0.86, word change 0.59, judge EQUIVALENT

> Einen Standort mit hellem, indirektem Licht sollten Sie für das Wohlbefinden Ihrer neuen Zimmerpflanze wählen. Sobald die oberste Erdschicht trocken erscheint, ist ein gründliches Gießen notwendig; dabei muss gewährleistet sein, dass durch Abzugslöcher im Topf überschüssiges Wasser abfließen kann. Einmal monatlich sollte während der Wachstumsphase ein ausgewogener Flüssigdünger zur Düngung verwendet werden. Zudem empfiehlt es sich, die Blätter alle paar Wochen behutsam mit einem feuchten Tuch von Staub zu befreien.

**max ≥ 0.2 (E4-7)** — divergence 0.88, word change 0.68, judge EQUIVALENT

> Positionieren Sie Ihre neue Zimmerpflanze für optimales Wohlbefinden an einem Standort, der durch helles, indirektes Licht geprägt ist. Sobald die oberste Erdschicht eine trockene Beschaffenheit aufweist, sollte ein gründliches Gießen erfolgen; hierbei ist sicherzustellen, dass überschüssiges Wasser durch Abzugslöcher im Topf abfließen kann. Einmal pro Monat wird während der Wachstumsphase ein ausgewogener Flüssigdünger zur Düngung verwendet. Um Staub zu beseitigen, empfiehlt es sich zudem, die Blätter alle paar Wochen behutsam mit einem feuchten Tuch abzuwischen.

### gemma4-12b · de · de-mx-07 chunk 0

**Source**

> Ich freue mich riesig, euch mitteilen zu können, dass ich offiziell ein neues Kapitel in meiner Karriere aufgeschlagen habe! Nach einer unglaublichen Zeit voller Wachstum und Lernen bin ich nun Teil eines Teams leidenschaftlicher Innovatoren, die wirklich etwas bewegen wollen. Ein herzliches Dankeschön an alle, die mich auf diesem Weg unterstützt haben. Auf neue Herausforderungen, bereichernde Zusammenarbeit und die spannenden Möglichkeiten, die vor uns liegen!

**min ≥ 0.05 (E4-3)** — divergence 0.91, word change 0.59, judge EQUIVALENT

> Es ist mir eine große Freude, euch verkünden zu dürfen, dass ich in meiner beruflichen Laufbahn ein neues Kapitel begonnen habe! Nach einer beeindruckenden Phase des Lernens und Wachstums schließe ich mich nun einem Team leidenschaftlicher Innovatoren an, die eine echte Veränderung bewirken möchten. Ein aufrichtiges Dankeschön geht an alle Personen, die mir auf diesem Pfad zur Seite standen. Ich blicke voller Vorfreude auf kommende Herausforderungen, eine bereichernde Kooperation und die spannenden Möglichkeiten, welche uns bevorstehen!

**min ≥ 0.6** — divergence 0.91, word change 0.59, judge EQUIVALENT

> Es ist mir eine große Freude, euch verkünden zu dürfen, dass ich in meiner beruflichen Laufbahn ein neues Kapitel begonnen habe! Nach einer beeindruckenden Phase des Lernens und Wachstums schließe ich mich nun einem Team leidenschaftlicher Innovatoren an, die eine echte Veränderung bewirken möchten. Ein aufrichtiges Dankeschön geht an alle Personen, die mir auf diesem Pfad zur Seite standen. Ich blicke voller Vorfreude auf kommende Herausforderungen, eine bereichernde Kooperation und die spannenden Möglichkeiten, welche uns bevorstehen!

**max ≥ 0.2 (E4-7)** — divergence 0.95, word change 0.71, judge EQUIVALENT

> Mit großer Freude verkündige ich euch heute den offiziellen Beginn eines neuen Lebensabschnitts in meiner beruflichen Laufbahn! Nach einer beeindruckenden Phase, die von persönlicher Entwicklung und Wissensgewinnung geprägt war, schließe ich mich nun einer Gruppe begeisterter Innovatoren an, deren Ziel es ist, echte Veränderungen zu bewirken. Mein aufrichtiger Dank gilt allen Personen, die mich auf diesem Weg begleitet haben. Ich blicke voller Vorfreude auf kommende Aufgaben, eine bereichernde Kooperation sowie die fesselnden Perspektiven, die uns bevorstehen!

### nemotron-bolt · en · en-mx-02 chunk 0

**Source**

> Elevate your morning routine with the AuroraBrew Pro, a sleek and intuitive coffee maker designed for those who refuse to compromise on flavor. Its precision temperature control ensures that every cup is brewed at the optimal point, unlocking the full aroma of your favorite beans. Whether you prefer a bold espresso or a smooth pour-over, the AuroraBrew Pro adapts seamlessly to your taste, making it the perfect companion for busy mornings and relaxed weekends alike.

**min ≥ 0.05 (E4-3)** — divergence 0.89, word change 0.82, judge EQUIVALENT

> For busy mornings and relaxed weekends equally, the AuroraBrew Pro becomes a fitting partner; it adapts without issue to your preference, be it a smooth pour-over or a strong espresso. Designed for individuals unwilling to sacrifice taste, this intuitive and streamlined coffee maker enhances how you begin the day. Optimal brewing is guaranteed by its exact temperature regulation, which brings out all the scent from your favored beans in every serving cup.

**min ≥ 0.6** — divergence 0.89, word change 0.82, judge EQUIVALENT

> For busy mornings and relaxed weekends equally, the AuroraBrew Pro becomes a fitting partner; it adapts without issue to your preference, be it a smooth pour-over or a strong espresso. Designed for individuals unwilling to sacrifice taste, this intuitive and streamlined coffee maker enhances how you begin the day. Optimal brewing is guaranteed by its exact temperature regulation, which brings out all the scent from your favored beans in every serving cup.

**max ≥ 0.2 (E4-7)** — divergence 0.94, word change 0.73, judge EQUIVALENT

> The AuroraBrew Pro, a coffee maker possessing an intuitive and streamlined design, permits you to improve your morning habits; it is intended for individuals unwilling to sacrifice taste. Optimal brewing point is guaranteed by its precise temperature regulation, thereby releasing the entire fragrance of preferred grounds in each serving. For either a mellow pour-over or a strong espresso preference, this AuroraBrew Pro adjusts smoothly to what suits you, rendering it an ideal partner during hurried mornings and calm weekends equally.

### nemotron-bolt · en · en-mx-05 chunk 0

**Source**

> The city council voted on Tuesday to approve a new plan aimed at reducing traffic congestion in the downtown area. The proposal includes expanding bike lanes, adjusting the timing of traffic signals, and introducing a pilot program for on-demand shuttle buses. Supporters argue that the measures will make the city center safer and more accessible, while critics have raised concerns about the cost and the potential impact on local businesses during construction.

**min ≥ 0.05 (E4-3)** — divergence 0.96, word change 0.72, judge EQUIVALENT

> Tuesday saw the city council's vote for sanctioning a fresh strategy designed to lessen traffic backup within the central district. This suggestion features bike lane enlargement, signal timing modifications, and starting a trial scheme for shuttle buses available upon request. Safer and more reachable will be the city core according to proponents; conversely, detractors voiced worries regarding expense and possible effects on neighborhood enterprises throughout building.

**min ≥ 0.6** — divergence 0.96, word change 0.72, judge EQUIVALENT

> Tuesday saw the city council's vote for sanctioning a fresh strategy designed to lessen traffic backup within the central district. This suggestion features bike lane enlargement, signal timing modifications, and starting a trial scheme for shuttle buses available upon request. Safer and more reachable will be the city core according to proponents; conversely, detractors voiced worries regarding expense and possible effects on neighborhood enterprises throughout building.

**max ≥ 0.2 (E4-7)** — divergence 0.98, word change 0.80, judge EQUIVALENT

> Approving a novel scheme to lessen traffic jams within the central district was decided by the city council on Tuesday. Expanding cycle paths, modifying signal timing, and launching a trial for requested shuttle vehicles are components of this suggestion. Safety and ease of entry for the municipal core will result from these actions, supporters contend; however, worry regarding expense and possible harm to neighborhood enterprises throughout building has surfaced from detractors.

### nemotron-bolt · en · en-mx-08 chunk 0

**Source**

> Yes, you can change your subscription plan at any time from your account settings. If you upgrade, the new features become available immediately, and you will only be charged the difference for the remainder of the current billing period. If you downgrade, the change takes effect at the start of your next billing cycle, so you can continue to enjoy your current features until then. Please note that some data may become read-only if it exceeds the limits of the smaller plan.

**min ≥ 0.05 (E4-3)** — divergence 0.91, word change 0.63, judge EQUIVALENT

> From your account settings, changing your subscription plan is possible anytime. Should you move up to a higher tier, the added capabilities appear right away; moreover, payment will cover solely the remaining amount for the present billing span. Conversely, if you decrease your level, the adjustment begins with the commencement of your subsequent billing period, meaning current features remain accessible until that point. Be aware that data might shift to read-only status if it surpasses the constraints of the less substantial plan.

**min ≥ 0.6** — divergence 0.91, word change 0.63, judge EQUIVALENT

> From your account settings, changing your subscription plan is possible anytime. Should you move up to a higher tier, the added capabilities appear right away; moreover, payment will cover solely the remaining amount for the present billing span. Conversely, if you decrease your level, the adjustment begins with the commencement of your subsequent billing period, meaning current features remain accessible until that point. Be aware that data might shift to read-only status if it surpasses the constraints of the less substantial plan.

**max ≥ 0.2 (E4-7)** — divergence 0.93, word change 0.66, judge EQUIVALENT

> At any point from your account settings, changing your subscription plan is possible. Should you move to a higher tier, the novel functionalities are accessible right away; moreover, only the remaining portion of the present billing duration will incur a charge for you. Conversely, should you lower your tier, that alteration begins when the subsequent billing period starts, meaning you keep using existing features until that moment. Remember some information might shift to read-only status if it surpasses the constraints of the lesser plan.

### nemotron-bolt · ru · ru-mx-03 chunk 0

**Source**

> В современном быстро меняющемся цифровом мире удалённая работа стала преобразующей силой, которая меняет подход организаций к своей деятельности. Она даёт сотрудникам беспрецедентную гибкость и избавляет от долгих поездок, однако вместе с тем создаёт и особые трудности, такие как поддержание сплочённости команды и предотвращение выгорания. В конечном счёте успех удалённой работы зависит от тонкого баланса между самостоятельностью и ответственностью.

**min ≥ 0.05 (E4-3)** — divergence 0.98, word change 0.79, judge EQUIVALENT

> Преобразующая мощь, меняющая отношение компаний к работе, обрела удалённый труд в сегодняшнем стремительно трансформирующемся цифровом пространстве. Сотрудникам она обеспечивает исключительную свободу и избавляет от продолжительных перемещений; тем не менее, возникают специфические проблемы, например, сохранение командного единства и избежание истощения. В итоге, достижение успеха в удалённом формате обусловлено деликатной гармонией между автономией и подотчётностью.

**min ≥ 0.6** — divergence 0.98, word change 0.79, judge EQUIVALENT

> Преобразующая мощь, меняющая отношение компаний к работе, обрела удалённый труд в сегодняшнем стремительно трансформирующемся цифровом пространстве. Сотрудникам она обеспечивает исключительную свободу и избавляет от продолжительных перемещений; тем не менее, возникают специфические проблемы, например, сохранение командного единства и избежание истощения. В итоге, достижение успеха в удалённом формате обусловлено деликатной гармонией между автономией и подотчётностью.

**max ≥ 0.2 (E4-7)** — divergence 0.98, word change 0.78, judge EQUIVALENT

> в эпоху стремительных перемен цифрового мира дистанционная занятость явилась силой, преобразующей, изменяя методы ведения дел компаниями. она обеспечивает работникам исключительную свободу и избавляет от продолжительных перемещений; тем не менее, это порождает определённые сложности, например сохранение единства коллектива и защита от истощения. в итоге достижение успеха дистанционной работы обусловлено тонким равновесием между автономией и подотчётностью.

### nemotron-bolt · ru · ru-mx-06 chunk 0

**Source**

> Пишу вам, чтобы выразить свою искреннюю заинтересованность в должности координатора по маркетингу в вашей компании. Обладая солидным опытом в цифровых кампаниях и страстью к сторителлингу, я уверена, что смогу стать ценным дополнением вашей команды. На предыдущем месте работы я успешно вела аккаунты в социальных сетях, взаимодействовала с разными отделами и помогла увеличить вовлечённость аудитории с помощью креативных и основанных на данных стратегий.

**min ≥ 0.05 (E4-3)** — divergence 0.76, word change 0.40, judge EQUIVALENT

> искренняя заинтересованность в должности координатора по маркетингу вашей компании хочу вам сообщить
> благодаря солидному опыту в цифровых кампаниях и любви к сторителлингу я уверена что стану полезным элементом вашей команды
> успешно вела аккаунты в социальных сетях на прежнем рабочем месте взаимодействовала с различными подразделениями и способствовала росту вовлечённости аудитории посредством креативных стратегий основанных на данных

**min ≥ 0.6** — divergence 0.76, word change 0.40, judge EQUIVALENT

> искренняя заинтересованность в должности координатора по маркетингу вашей компании хочу вам сообщить
> благодаря солидному опыту в цифровых кампаниях и любви к сторителлингу я уверена что стану полезным элементом вашей команды
> успешно вела аккаунты в социальных сетях на прежнем рабочем месте взаимодействовала с различными подразделениями и способствовала росту вовлечённости аудитории посредством креативных стратегий основанных на данных

**max ≥ 0.2 (E4-7)** — divergence 0.89, word change 0.57, judge EQUIVALENT

> Свою искреннюю заинтересованность в позиции координатора по маркетингу вашей организации хочу вам сообщить. Имея значительный опыт в цифровых кампаниях и увлечение сторителлингом, я полагаю, что стану полезным элементом коллектива. На прежнем рабочем месте мне удалось эффективно управлять аккаунтами в социальных сетях, сотрудничать с различными департаментами, а также поднять вовлечённость аудитории за счёт креативных и опирающихся на данные планов.

### nemotron-bolt · ru · ru-mx-09 chunk 0

**Source**

> В целом результаты опроса свидетельствуют о высоком уровне удовлетворённости участников: большинство респондентов оценили семинар как полезный и хорошо организованный. Вместе с тем ряд слушателей отметил, что занятия проходили в спешке и что им не хватило времени на вопросы. В дальнейшем рекомендуется отводить больше времени на обсуждение и заранее предоставлять участникам дополнительные материалы.

**min ≥ 0.05 (E4-3)** — divergence 0.89, word change 0.57, judge EQUIVALENT

> Большинство опрошенных участников выразили высокую степень довольства: семинар был оценён как полезный и хорошо структурированный. Однако некоторые слушатели заметили, что занятия шли в спешке, а времени на вопросы оказалось недостаточно. В будущем следует выделять больше времени для дискуссий и заблаговременно предоставлять участникам сопутствующие документы.

**min ≥ 0.6** — divergence 0.89, word change 0.57, judge EQUIVALENT

> Большинство опрошенных участников выразили высокую степень довольства: семинар был оценён как полезный и хорошо структурированный. Однако некоторые слушатели заметили, что занятия шли в спешке, а времени на вопросы оказалось недостаточно. В будущем следует выделять больше времени для дискуссий и заблаговременно предоставлять участникам сопутствующие документы.

**max ≥ 0.2 (E4-7)** — divergence 0.97, word change 0.71, judge EQUIVALENT

> Высокая степень удовлетворенности участников, в общем, отражена результатами опроса; большинство опрашиваемых семинар сочли полезным и слаженно организованным. Однако некоторые слушатели обратили внимание на то, что занятия шли поспешно, а времени для вопросов оказалось недостаточно. В будущем следует выделять больше часов для беседы и заблаговременно предоставлять участникам дополнительные ресурсы.

### nemotron-bolt · de · de-mx-01 chunk 0

**Source**

> Vielen Dank für Ihre Nachricht bezüglich der verspäteten Lieferung. Wir verstehen sehr gut, wie ärgerlich es ist, wenn eine Bestellung nicht rechtzeitig ankommt, insbesondere wenn Sie sie für ein bevorstehendes Projekt benötigen. Wir haben Ihre Bestellung überprüft und festgestellt, dass das Paket aufgrund eines unerwarteten Personalmangels in unserem regionalen Verteilzentrum aufgehalten wurde. Seien Sie versichert, dass es inzwischen versandt wurde und Sie innerhalb der nächsten zwei Werktage erreichen wird.

**min ≥ 0.05 (E4-3)** — divergence 0.88, word change 0.61, judge EQUIVALENT

> Hinsichtlich Ihrer Nachricht zur verspäteten Lieferung bedanken wir uns. Besonders wenn Sie eine Bestellung für ein bevorstehendes Projekt benötigen, ist es sehr verständlich, wie ärgerlich eine nicht rechtzeitige Ankunft ist. Unsere Prüfung ergab bei Ihrer Bestellung einen Halt im regionalen Verteilzentrum wegen eines unerwarteten Personalmangels. Gewissheit geben wir Ihnen: Es wurde nun versandt und Sie erhalten es innerhalb der nächsten 2 Werktage.

**min ≥ 0.6** — divergence 0.88, word change 0.61, judge EQUIVALENT

> Hinsichtlich Ihrer Nachricht zur verspäteten Lieferung bedanken wir uns. Besonders wenn Sie eine Bestellung für ein bevorstehendes Projekt benötigen, ist es sehr verständlich, wie ärgerlich eine nicht rechtzeitige Ankunft ist. Unsere Prüfung ergab bei Ihrer Bestellung einen Halt im regionalen Verteilzentrum wegen eines unerwarteten Personalmangels. Gewissheit geben wir Ihnen: Es wurde nun versandt und Sie erhalten es innerhalb der nächsten 2 Werktage.

**max ≥ 0.2 (E4-7)** — divergence 0.93, word change 0.63, judge EQUIVALENT

> Bezüglich der verspäteten Lieferung danken wir Ihnen für Ihre Nachricht. Ein nicht pünktlich eintretendes Bestellgut ist ärgerlich, besonders wenn es für ein kommendes Projekt notwendig ist – dies verstehen wir vollkommen. Unsere Prüfung ergab bei Ihrer Bestellung: Das Paket verharrte wegen eines unvorhergesehenen Personalengpasses im regionalen Verteilzentrum. Gewissheit erhalten Sie: Es wurde bereits versendet und in den nächsten 2 Werktagen ankommen.

### nemotron-bolt · de · de-mx-04 chunk 0

**Source**

> Damit sich Ihre neue Zimmerpflanze wohlfühlt, stellen Sie sie an einen Ort mit hellem, indirektem Licht. Gießen Sie sie gründlich, sobald sich die oberste Erdschicht trocken anfühlt, und achten Sie darauf, dass der Topf Abzugslöcher hat, damit überschüssiges Wasser ablaufen kann. Während der Wachstumsphase düngen Sie einmal im Monat mit einem ausgewogenen Flüssigdünger. Wischen Sie außerdem alle paar Wochen die Blätter vorsichtig mit einem feuchten Tuch ab, um Staub zu entfernen.

**min ≥ 0.05 (E4-3)** — divergence 0.81, word change 0.47, judge EQUIVALENT

> Ein Standort mit indirektem, hellem Licht ist für das Wohlbefinden Ihrer neuen Zimmerpflanze ratsam. Gründliches Gießen erfolgt, sobald die oberste Erdschicht trocken erscheint; zudem muss der Topf Abzugslöcher besitzen, damit überschüssiges Wasser entweichen kann. Einmal monatlich während der Wachstumsphase sollte mit einem ausgewogenen Flüssigdünger gedüngt werden. Mit einem feuchten Tuch wischen Sie ferner alle paar Wochen vorsichtig die Blätter ab, um Staub zu beseitigen.

**min ≥ 0.6** — divergence 0.81, word change 0.47, judge EQUIVALENT

> Ein Standort mit indirektem, hellem Licht ist für das Wohlbefinden Ihrer neuen Zimmerpflanze ratsam. Gründliches Gießen erfolgt, sobald die oberste Erdschicht trocken erscheint; zudem muss der Topf Abzugslöcher besitzen, damit überschüssiges Wasser entweichen kann. Einmal monatlich während der Wachstumsphase sollte mit einem ausgewogenen Flüssigdünger gedüngt werden. Mit einem feuchten Tuch wischen Sie ferner alle paar Wochen vorsichtig die Blätter ab, um Staub zu beseitigen.

**max ≥ 0.2 (E4-7)** — divergence 0.81, word change 0.55, judge EQUIVALENT

> An einen Ort mit hellem, indirektem Licht sollte Ihre neue Zimmerpflanze platziert werden, damit sie sich wohlfühlt. Sobald die oberste Erdschicht trocken ist, gießen Sie sie gründlich; zudem muss der Topf Abzugslöcher besitzen, sodass überschüssiges Wasser abfließen kann. Einmal monatlich erfolgt während der Wachstumsphase eine Düngung mittels eines ausgewogenen Flüssigdüngers. Darüber hinaus entfernen Sie Staub alle paar Wochen vorsichtig von den Blättern mithilfe eines feuchten Tuchs.

### nemotron-bolt · de · de-mx-07 chunk 0

**Source**

> Ich freue mich riesig, euch mitteilen zu können, dass ich offiziell ein neues Kapitel in meiner Karriere aufgeschlagen habe! Nach einer unglaublichen Zeit voller Wachstum und Lernen bin ich nun Teil eines Teams leidenschaftlicher Innovatoren, die wirklich etwas bewegen wollen. Ein herzliches Dankeschön an alle, die mich auf diesem Weg unterstützt haben. Auf neue Herausforderungen, bereichernde Zusammenarbeit und die spannenden Möglichkeiten, die vor uns liegen!

**min ≥ 0.05 (E4-3)** — divergence 0.84, word change 0.52, judge EQUIVALENT

> Ein neues Kapitel in meiner Karriere habe ich offiziell mit großer Freude bekannt geben! Nach einer unglaublichen Periode des Lernens und Wachsens bin ich nun Mitglied eines Teams engagierter Innovatoren, das tatsächlich Fortschritt erzielen möchte. Allen, die mich auf diesem Pfad beistehten, danke herzlich. Auf die kommenden Chancen, bereichernde Kooperationen und die fesselnden Perspektiven vor uns!

**min ≥ 0.6** — divergence 0.84, word change 0.52, judge EQUIVALENT

> Ein neues Kapitel in meiner Karriere habe ich offiziell mit großer Freude bekannt geben! Nach einer unglaublichen Periode des Lernens und Wachsens bin ich nun Mitglied eines Teams engagierter Innovatoren, das tatsächlich Fortschritt erzielen möchte. Allen, die mich auf diesem Pfad beistehten, danke herzlich. Auf die kommenden Chancen, bereichernde Kooperationen und die fesselnden Perspektiven vor uns!

**max ≥ 0.2 (E4-7)** — divergence 0.89, word change 0.62, judge EQUIVALENT

> Offiziell habe ich ein neues Kapitel in meiner Karriere begonnen, was mich sehr freut! Nach einer Zeit des Lernens und Wachsens, die unglaublich war, gehöre ich nun zu einem Team engagierter Innovatoren, welche tatsächlich Wirkung erzielen möchten. Dank gilt allen, wer mir auf diesem Pfad beisteht. Auf die spannenden Möglichkeiten, die uns erwarten, bereichernde Kooperation und neue Herausforderungen!

