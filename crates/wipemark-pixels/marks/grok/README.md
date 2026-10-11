# `grok/` — Grok's wordmark on X, measured from the owner's pictures

Grok puts a visible mark on the pictures it makes **in X** only (the owner,
2026-10-11): a slashed circle and "Grok" in the bottom-right corner, about
0.083 of the picture's width, in two colours — black over light pictures,
white over dark ones (`grok-x-dark`, `grok-x-light` in
`manifests/marks.v1.json`). The two colours have one opacity (α ≈ 0.37 at
the strokes' core, 0.41–0.43 at most, no hole) and **not one shape**: a
black and a white picture of one size (784×1168, 1168×784) agree at the core
and part on the soft edge, where the white mark is thinner (its map 0.90–0.93
of the black one's, NCC 0.98–0.99). So each profile has **a map per size of
its own colour** — twelve maps, six sizes (D572).

Every map is 16-bit `.wma`, **fitted** (`"fitted": true`, D245): one or two
pictures' measurement, never claimed exact, and a restoration with it is held
to the picture around the mark. All twelve were made by
`scripts/grok/fit-maps.py fit` on 2026-10-11 (its header says how), which also
wrote the numbers below into its ignored report
(`reports/grok-fit-maps-2026-10-11/fit.md`). The pictures are the owner's, read
in place, never in git beyond the five in `fixtures/image/grok-x/`:

```sh
python3 scripts/grok/fit-maps.py fit --flat '/workspace/grok-images/x/*.jpeg' \
    --drawn 1728x1152=fixtures/image/grok-x/HUS9M6oX0AERoMb.jpeg \
            1600x1200=fixtures/image/grok-x/HUVe9s_XkAAwxck.jpeg \
    --report reports/grok-fit-maps-2026-10-11 --date 2026-10-11
```

**How a map is made**, by what exists at its size:

* **measured** — the white mark off a flat black corner, `α = (I − B)/(255 −
  B)`; the black one off a flat white, `α = (B − I)/B`, or off the first
  look's flat grey, `1 − I/B` (`B` the ring's quadratic, luma); captures of
  one size and colour averaged;
* **drawn** (the black mark at a size with no white or grey picture) — the
  1792×1008 black map drawn by the area integral where the picture put the
  mark: on a black capture of 784×1168 and 1168×784 the drawn map agrees with
  the measured one at NCC 0.995–0.996;
* **derived** (the white mark at a size with no black picture) — the black
  map through the **colour curve**, the white α as a function of the black
  one sample for sample, measured on the two black-and-white pairs: made
  without one pair it gives that pair's white map to RMS 0.0099 and 0.0104,
  against 0.034–0.038 for the black map as it is. Through it the textured
  brown 1792×1008 picture, refused by the range with the black map (3.9 %),
  is proved at `k*` 0.98 and restored with no mark left.

| map | sha256 | size | row (picture, margin r, b) | how | from (sha256 of the picture) |
|---|---|---|---|---|---|
| `grok-x-dark-1792x1008.wma` | `b3e5cba49531304431f4db66e9a3b08ac0b88e10acb86f48f7f30e818e4ac5db` | 149×57 | 1792×1008, 9, 11 | measured on a flat grey (luma 201) — the first look's map | `HUVfLiSW0AEtmX_` (`8e63edab2dd6b86c…`, `fixtures/image/grok-x/`) |
| `grok-x-light-1792x1008.wma` | `39da3d3d190003b4aeb671fabafc52aebd89cb7c0dd8b6856f2ec874699f0548` | 149×57 | 1792×1008, 9, 11 | derived from the black map | as above |
| `grok-x-dark-1728x1152.wma` | `946d4c9718b687c7b21738c2016b0faf70d78b494fadcd68f8c51e1448166ad2` | 148×59 | 1728×1152, 7, 9 | **drawn** at (1575.125, 1086.000), width 143.688, where NCC (0.786) put the mark on a beige texture | `HUS9M6oX0AERoMb` (`e4da860583857cef…`, `fixtures/image/grok-x/`) |
| `grok-x-light-1728x1152.wma` | `92c03fc61986f4889fc035e8937b02e678b92916e7385c9b4eff6cccd165a7be` | 148×59 | 1728×1152, 7, 9 | derived from the drawn black map | as above |
| `grok-x-dark-1600x1200.wma` | `85c359121d4ff2818150ab6da2d1de6240b450d3736c9b3ad277aecb83e75bba` | 138×55 | 1600×1200, 7, 9 | **drawn** at (1457.312, 1138.062), width 133.125, where NCC (0.448) put it on a red fabric | `HUVe9s_XkAAwxck` (`7db4d175d5066499…`, `fixtures/image/grok-x/`) |
| `grok-x-light-1600x1200.wma` | `b688c3d1299d27d41885356fa3f2db16a60c49b631fdcbd444ea54cd97664fc3` | 138×55 | 1600×1200, 7, 9 | derived from the drawn black map | as above |
| `grok-x-dark-1168x784.wma` | `ce04771c6d9b7d9db2feac9747a43a040b90dae1351e3b2761d1f7301bd16acd` | 102×42 | 1168×784, 8, 9 | measured on flat white, two pictures (RMS 0.0012 apart) | `HUXGPYRXIAARnhd` (`a24209ea679db27a…`), `HUXGzYLXkAAKA1L` (`2876160f0cfa83e9…`) |
| `grok-x-light-1168x784.wma` | `bfc5def135fa26fb715c54add1d04326519fc47eb3bddff049ff7b1ffa35221e` | 102×42 | 1168×784, 8, 9 | measured on flat black | `HUW5_31W4AAH8IY` (`33d4accf38ccefe3…`) |
| `grok-x-dark-1152x1712.wma` | `4f6c1eb26ff055b22de6851d07999789d5e82e6496255c3d9dbd3a6caaabcd67` | 101×42 | 1152×1712, 8, 9 | **drawn** at (1045.812, 1663.688), width 95.688, where the black pictures put the mark | `HUW62vIXAAAI21T` (`c57874d36fd29f7f…`), `HUW6njDWAAAGSxF` (`efd6b6529c932748…`) |
| `grok-x-light-1152x1712.wma` | `9e4730dc3382920c5fbc998fe0caf9098e84100f5bc260e07a5be4fb5e3a96f4` | 101×42 | 1152×1712, 8, 9 | measured on flat black, two pictures (they agree to 0.00000) | as above |
| `grok-x-dark-784x1168.wma` | `0a06e77477c865d1b6a3afe28302f6ade5be57e7466b8f1b2b0411907c334ae5` | 69×29 | 784×1168, 9, 10 | measured on flat white, two pictures (RMS 0.0017 apart) | `HUXHBuXXAAAoPjs` (`d97471cdbf3de015…`), `HUXHYfZWQAAWTEQ` (`4d300e95164c8d16…`) |
| `grok-x-light-784x1168.wma` | `20f91e85fcd7564eec6bf702257ee406fae7516ab98ab92399cd18dcfdffc7bb` | 69×29 | 784×1168, 9, 10 | measured on flat black, two pictures (RMS 0.00006 apart) | `HUW5egmXMAAbDzo` (`210daf15fc311bf6…`), `HUW5Mw-WIAAPXqN` (`53070d1adf4bda15…`) |

The measured maps keep the samples the ring's noise does not explain
(negatives to 0; a sample under three times the ring's own α spread, at least
0.01, farther than two pixels from one over it, to 0) and two empty pixels
around the mark. The pictures from X are progressive 4:2:0 JPEGs at libjpeg's
quality 95; luma is stored at full resolution, which is what the maps are read
from.

**What a capture would replace**: a white (or grey) picture of 1152×1712 for
its drawn black map; a black one of 1792×1008, 1728×1152 and 1600×1200 for the
derived white maps; any flat picture of 1728×1152 and 1600×1200 for their
drawn black maps; and every new size X hands out, which has no row yet.
