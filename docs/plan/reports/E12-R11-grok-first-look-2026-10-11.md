# E12-R11, early — Grok's mark, a first look from the owner's pictures

*2026-10-11. Task `wipemark-task-recon-grok-first-look-2026-10-11` (F1–F5), branch
`recon/grok-first-look` from `feat/e0-e6-shell` at `918e314`. Checked at that
commit; no Rust changed. Every figure below is made by
`scripts/grok/first-look.py` (committed with this report) with the CLI
`scripts/grok/cli-with-profile.sh` builds; the pictures, maps, catalogue and crops
are under the ignored `reports/grok-first-look-2026-10-11/` and never enter git.*

## Для владельца

Знак Grok — перечёркнутый круг и слово «Grok» в правом нижнем углу. Его ширина
всегда около 8,3 % ширины картинки, а отступ от правого и нижнего края — около
10–12 пикселей при любом размере. Знак бывает двух цветов: на светлом фоне он
тёмный (полупрозрачный чёрный), на тёмном — светлый (полупрозрачный белый);
форма и прозрачность у обоих одни и те же, около 36 % в толще штриха, полностью
непрозрачных мест нет — значит, под знаком всё восстанавливается без
«дорисовывания». По серой картинке я снял точную карту тёмного знака, и с ней
программа находит знак на этой картинке, доказывает его и снимает почти без
следа. Но в нынешнем виде программа ищет только знаки светлее картинки, поэтому
тёмный вариант она пока не видит вовсе — это одно место в коде, его изменение
предложено отдельным решением (D540) и в этой работе не делалось. Светлый
вариант программа находит и доказывает по прозрачности, но отказывается снимать:
края букв у светлого знака чуть иные, чем у тёмного, а снять их по одной
картинке на пёстром фоне нельзя. На пёстрых тканях (красная, бежевая) проверка
краёв отказывает даже идеально наложенному знаку — это правило программы, оно
не ослаблялось. Чтобы сделать настоящий профиль, нужно несколько простых
картинок из X, перечисленных ниже.

**Что сделать (по схеме имён из руководства, `grok-captures/x/`):**

1. **Однотонные чёрные** — `black-16x9-01.jpg` … `-03` (1792×1008) и
   `black-2x3-01.jpg` … `-03` (784×1168): светлый вариант знака на ровном фоне —
   его собственная карта и края.
2. **Однотонные белые** — `white-16x9-01` … `-03` и `white-2x3-01` … `-03`:
   тёмный вариант на втором ровном фоне — тогда прозрачность и цвет знака
   разделяются, а не предполагаются.
3. **Серые** — `gray-25-…`, `gray-50-…`, `gray-75-…` по 2–3 штуки (хватит
   одного размера, 1792×1008): где Grok переключает тёмный знак на светлый —
   где-то между яркостью 70 и 160 из 255.
4. **Те же размеры, что уже есть**: 1600×1200 и 1728×1152 — по одной белой и
   одной чёрной (`white-4x3-01`, `black-4x3-01`, `white-3x2-01`,
   `black-3x2-01`): карта знака своего размера, а не пересчитанная.
5. **Одна и та же картинка двумя путями** — скачанная с grok.com
   (`web/white-16x9-01.png` или `.jpg`, как отдаёт сайт) и из X
   (`x/white-16x9-01.jpg`): пережимает ли X картинку и меняет ли размер; если
   сайт отдаёт PNG — это лучший материал.

Фото и текстуры пока не нужны: на них проверено, что уже есть.

## What the pictures are

Five, not three: the owner added `HUS9M6_W8AAkLdd.jpeg` and
`HUS9M6oX0AERoMb.jpeg` to `/workspace/grok-images/` the same morning (08:28);
both are used. All five are progressive JPEGs, 4:2:0, APP0 only, with one and
the same DQT 0 (`[2, 1, 1, 1, 1, 1, 2, 1]`, its first eight values as stored, in
zigzag order — the coordinator's "2,1,1,2,2,4,5,6" is the same table read in
another order), and X media ids as names:
the `x` source, very likely re-encoded by X.

| picture | size | sha256 | under the mark | variant |
|---|---|---|---|---|
| `HUVfLiSW0AEtmX_` | 1792×1008 | `8e63edab2dd6b86ca01d8cfca813342cc5c45c8d537c0c2b66ddcbd482b35e67` | flat grey, luma 201.0, σ_ring 1.0–1.2 | dark (the reference) |
| `HUVe9s_XkAAwxck` | 1600×1200 | `7db4d175d50664995dfd8e0bbf33793df89d908d6eceb4e4c3a5801d85e36c33` | red fabric + cream stripe, 160.2, σ 12–22 | dark |
| `HUVfYqgWkAAOhiJ` | 784×1168 | `1ce6fe267427ba34a55d187eec761163ac45125641294d3743aa87ed21d323d9` | blue, structured, 70.6, σ 3–24 | light |
| `HUS9M6_W8AAkLdd` | 1792×1008 | `7da20b6ccdb9764800a5c5778ae674e6e78972e9f231e79e5fdca1ff8fc8a7f3` | dark brown texture, 28.6, σ 35–42 | light |
| `HUS9M6oX0AERoMb` | 1728×1152 | `e4da860583857cef05ca6df9c7aa017d9f412e0f7639e5c2b4380f698bc145f7` | beige plaster texture, 182.6, σ 18 | dark |

## F1 — the figures

### Where the mark is, and how big

The template is the reference's depth `B_Y − I_Y` over its rectangle, the tight
box at 3.9 levels off the ring's quadratic grown by 2 px: **149×57 at (1634,
940)** (tight: 145×53 at (1636, 942), the coordinator's 146×55 by another
threshold). Located in each picture by the area integral at every whole-pixel
width, then refined to 1/16 px by NCC:

| picture | NCC | x | y | width | margin right | margin bottom | w/W |
|---|---|---|---|---|---|---|---|
| 1792 grey | 1.000 | 1634.00 | 940.00 | 149.00 | 9.00 | 11.00 | 0.0831 |
| 1600 red | 0.448 | 1457.31 | 1138.06 | 133.12 | 9.56 | 11.01 | 0.0832 |
| 784 blue | 0.627 | 708.19 | 1131.12 | 64.56 | 11.25 | 12.18 | 0.0824 |
| 1792 brown | 0.711 | 1634.00 | 940.00 | 149.00 | 9.00 | 11.00 | 0.0831 |
| 1728 beige | 0.786 | 1575.12 | 1086.00 | 143.69 | 9.19 | 11.03 | 0.0832 |

The coordinator's NCCs (0.494, 0.591) are reproduced to within the template's
definition (bilinear 146×55 there, area integral 149×57 here). The two 1792×1008
pictures — one dark variant, one light — put the mark on **the same pixel**: the
geometry is fixed per size.

**Laws** (through the reference; worst picture's distance from the law):

| width ∝ | CV | worst, px |
|---|---|---|
| **W** | **0.0039** | **0.62** |
| longer side | 0.144 | 32.6 |
| √(W·H) | 0.164 | 41.5 |
| shorter side | 0.201 | 51.3 |
| constant | 0.252 | 84.4 |
| H | 0.290 | 108.1 |

Width = **0.0832 × W** of the template's box (0.0809 × W of the tight box). The
margins are **constant in pixels** (right worst 2.25 px off, bottom 1.18 px; ∝ W
or ∝ the mark's width: 7.3–7.4 px off), 9/11 px to the template's box,
11/13 px to the tight box; the 784×1168 picture is 1.6–2.25 px off that — the
law is not exact at the smallest size (or that structured background moved the
NCC); the rows below therefore take each size's measured place.

### Two variants, one shape

At the strokes' core against the ring's quadratic (per channel R, G, B):

| picture | bg luma | dark fit a | dark RMS | light fit a | light RMS | variant |
|---|---|---|---|---|---|---|
| 1792 grey | 201.0 | 0.365, 0.365, 0.364 | 2.2 | — | 2.2 (a negative) | dark |
| 1600 red | 160.2 | 0.315, 0.342, 0.370 | 9.8–23.9 | negative | 22–32 | dark |
| 784 blue | 70.6 | negative | 18.5–20.7 | 0.315, 0.357, 0.411 | 10.8–22.8 | light |
| 1792 brown | 28.6 | negative | 32–42 | 0.307, 0.340, 0.334 | 9.1–13.2 | light |
| 1728 beige | 182.6 | 0.349, 0.359, 0.366 | 12.4–12.6 | negative | 12.5 | dark |

Dark on backgrounds of luma 160–201, light on 29–71: Grok chooses the variant by
the background, the switch somewhere between 71 and 160 (not measurable here).
The free one-colour fit `I − B = a·(L − B)` gives L of −286, −61, 372, 124 and
−86 — meaningless where `B` barely varies under the core (one flat background
fixes only `a·(L − B)`), and the table carries `B spread` to say so.

**The dark map** (`α = 1 − I/B` on the reference): core median **0.363**, max
0.410, the three channels alike (core 0.364 / 0.363 / 0.362, spread 0.002 where
α > 0.1 — a black logo dims them alike, which on one grey says nothing about L's
grey level). Noise: α over the ring sd 0.0052 (p99 of |α| 0.018), over the band
past it 0.0094; σ_ring 1.0–1.2 levels — JPEG noise and the slight gradient of
that corner. **No hole**: nothing reaches 0.95; the maximum is 0.41.

**The light variant**: `(I − B)/(255 − B)`, rough — relative to the dark map drawn
at the same place, NCC 0.821 (blue) and 0.736 (brown), slope 0.868 and 0.854.
That slope is biased low (`B` on dark texture, place errors); **the product's gain
sweep says otherwise and better**: on the brown 1792 picture, at the dark map's
own α with logo 255, `k* = 1.000` and the contour ratio 0.249. Dark textured
pictures with the dark map: slope 0.955 (red) and 1.018 (beige). So: **one shape,
one opacity (≈0.36 at the core), two colours** — black on light backgrounds,
white on dark. The provisional light profile carries the dark map unscaled
(`--light-scale 1.0`; `auto` would give 0.86 from the biased slopes). Where the
variants differ is the soft edge (F3).

## F2 — the provisional profile

`reports/grok-first-look-2026-10-11/catalogue/marks.json`, two profiles in the
shipped schema, `status: provisional`, observed from 2026-10, vendor `xai`,
product `grok`: **`grok-x-dark`** (logo `[0, 0, 0]`) and **`grok-x-light`** (logo
`[255, 255, 255]`), both the reference's luma α (negatives to 0, samples under
0.0157 — 3σ of the ring's α — farther than 2 px from one over it to 0), 16-bit
`.wma`, pinned by sha256, `opaque_above` 0.95, `min_ncc` 0.70, verify 0.06 / 0.30
/ 0.01 — the shipped profiles' limits, nothing loosened. Rows, one per size seen,
each its own map at a corner margin:

| size | map | margin | how |
|---|---|---|---|
| 1792×1008 | `…-149x57` | 9, 11 | the reference's own map |
| 1728×1152 | `…-144x55` | 9, 11 | drawn at (1575.125, 1086.000), width 143.688 |
| 1600×1200 | `…-134x51` | 9, 11 | drawn at (1457.312, 1138.062), width 133.125 |
| 784×1168 | `…-65x25` | 11, 12 | drawn at (708.188, 1131.125), width 64.562 |

plus a search (bottom-right 400×200, widths 44–238, the 149 map). Why each size
has a map of its own: the catalogue takes a resampled `rect` only at the map's
exact aspect (`catalogue.rs`), which 0.083·W never gives in whole pixels — so
the reference map is drawn by the area integral at the sub-pixel place and the
offset baked into a map one pixel larger. Rows at the measured place
(`--rows measured`, the default) rather than the laws' (`--rows law`): two
pictures of one size put the mark on the same pixel, and the law missed 784×1168
by 1.6 px. `calibrate` could not be used: it needs black, white and grey captures
of one size; there is one flat picture. `regress.py` was not run: no Rust moved.

## F3 — the product on the pictures

The CLI takes no `--catalogue` (the coordinator's assumption): the product reads
only its compiled-in catalogue (D526), and only `recon_bench` and `measure_clean`
take the file. So `scripts/grok/cli-with-profile.sh` builds a scratch
`wipemark-cli` from `HEAD`'s tracked tree with the two profiles appended to the
manifest and their maps beside the shipped ones — the product as it would be
with these rows shipped, in a copy that never touches the working tree. Then
`inspect --json` and `clean -o --json` per picture:

| picture | inspect / clean exit | what the product did |
|---|---|---|
| 1792 grey (dark) | 0 / 0 | **nothing proposed**: the dark mark correlates negatively with its map, and the proposal's NCC is signed (`propose.rs`) — a mark darker than the picture is never a proposal |
| 1600 red (dark) | 0 / 0 | nothing proposed, the same reason |
| 784 blue (light) | 1 / 3 | `grok-x-light` row 0, NCC 0.650, **refused by edges** 0.382 (k* 1.06, out of range 1.1 %); mark left, file written unchanged in pixels |
| 1792 brown (light) | 1 / 3 | `grok-x-light` row 3, NCC 0.763, **k* = 1.000**, edges 0.249 (passes), **refused out of range** 3.87 % (Y plane; chroma 0); mark left |
| 1728 beige (dark) | 0 / 0 | nothing proposed, the same reason as the grey |

With the laws' rows (`reports/grok-first-look-2026-10-11-law/`) the blue is not
proposed at all (its row 1.6 px off, the search's NCC under 0.70); the rest is
the same.

**Which check refuses, and why** — two measurements beside the product, both
PNGs (lossless: no planes, no DCT-POCS, no texture check):

* *Inverted* (`--invert-dark`): `(1 − α)·B` inverted is exactly `α·255 + (1 −
  α)·(255 − B)`, so the light profile examines the dark mark there.
* *Control* (`--control`): the row's map blended exactly over the picture's own
  texture 1.5 mark widths to the left (pasted over the mark).

| picture | inverted | control (a perfect blend of the map) |
|---|---|---|
| 1792 grey | **verified** (row, NCC 0.999, k* 1.000, edges 0.015, out of range 0); restored, outline 0.003, steps 0.29 levels, consistency 0.33 | verified, edges 0.015 |
| 1600 red | refused by edges 0.445 (k* 1.06) | **refused by edges** 0.517 |
| 784 blue | — (light) | **verified**, edges 0.054 |
| 1792 brown | — (light) | **verified**, edges 0.171 |
| 1728 beige | refused by gain 1.08 (edges 0.544) | **refused by edges** 0.531 |

Reading it:

1. **The dark map is good on its own picture**: once the sign is out of the way
   the reference is proved and restored with no outline said (the crop
   `HUVfLiSW0AEtmX_-inverted-before-after-x4.png`: a faint dotted trace at ×4
   amplification, 0.29 levels). What stops the dark variant everywhere is a
   **rule of the code, not the map**: the proposal looks only for marks lighter
   than the picture. Proposed below as D540; not built here.
2. **The light 1792 is refused by the map's edge**: the 3.9 % out-of-range
   samples are on the strokes' soft edge (α 0.04–0.31, median 0.14), 3.5 levels
   past the 8-level allowance at the median — the light mark's edge is fainter
   than the edge the dark mark left on grey (JPEG ringing has the opposite sign
   on the two, and Grok may antialias the white glyphs otherwise). The control
   on the same texture proves, so it is the map, not the rule; a flat black
   capture gives the light variant's own edge.
3. **The 784 blue is refused by the map at that size**: the control proves, the
   real mark does not (edges 0.382) — the 149-px map drawn down to 65 px is not
   what Grok drew at 65 px (it likely renders the vector logo per size). A flat
   capture at 784×1168 gives that size its own map.
4. **The red and beige textures are past the edge rule whatever the map**: a
   perfect blend of the map is refused there (0.52–0.53 against 0.30). That is
   D154's edge check on a high-contrast texture, as it should be; not loosened.

Crops (before | after | the result's departure from its ring's quadratic ×4,
all ×4 nearest), for the coordinator, not in git:
`reports/grok-first-look-2026-10-11/product/*-before-after-x4.png`, with
`-inverted-` and `-control-` beside them.

## Decisions

None taken. **Proposed, D540 — a mark darker than the picture is proposed.** The
first proof's NCC takes the logo's side: for a profile whose logo is darker than
the picture around the mark (Grok's `[0, 0, 0]` variant), the correlation of
luma with α is negative, and the proposal compares `−ncc` with `min_ncc` (or
correlates with `α·(L − mean)`'s sign). Measured: with the sign out of the way
(inversion, exact for the encoded blend) the dark map is proved at k* 1.000,
edges 0.015, and restored with outline 0.003 on its own picture; on the two dark
textures the edges refuse it as they refuse a perfect blend. No other check
assumes a light logo as far as this run shows (the gain sweep, the range and the
restoration take `L` as given). Building it is a change to `wipemark-pixels`
with its own red tests (a dark composite proposed and proved; a dark look-alike
still refused), level A and level B — another task.

## Questions

* Q-R5 (xAI's policy on removing Grok's mark) stands; nothing here says Grok is
  supported.
* Is X the only road the owner gets these from? DQT, progressive, APP0 only are
  X's re-encode; F4 item 5 answers what grok.com hands out.

## Deviations

* Five pictures, not three (two added by the owner the same morning).
* The CLI has no `--catalogue`; F3 used a scratch CLI built with the profile
  compiled in (`scripts/grok/cli-with-profile.sh`, new) — also what
  `run.sh --cli-with-profile` waits for.
* Rows at each size's measured place, not `(W − 12 − w, H − 12 − h)`: the margin
  is constant but not to the pixel at 784×1168.
* `calibrate` not used (no black/white/grey set); the map is written by
  `first-look.py` and says so in its header.
* Pillow decodes for the maps (libjpeg upsampling, not zune-jpeg's planes, R11
  §4.2): a first look, as the header says.

## Files

* `scripts/grok/first-look.py` — `look` (F1, F2) and `product` (F3, with
  `--invert-dark` and `--control`).
* `scripts/grok/cli-with-profile.sh` — the scratch CLI with a `--catalogue`
  file's profiles compiled in.

Reproduce:

```sh
G=/workspace/grok-images
P="$G/HUVfLiSW0AEtmX_.jpeg $G/HUVe9s_XkAAwxck.jpeg $G/HUVfYqgWkAAOhiJ.jpeg $G/HUS9M6_W8AAkLdd.jpeg $G/HUS9M6oX0AERoMb.jpeg"
python3 scripts/grok/first-look.py look $P --date 2026-10-11
scripts/grok/cli-with-profile.sh reports/grok-first-look-2026-10-11/catalogue/marks.json \
    --out reports/grok-first-look-2026-10-11/cli
python3 scripts/grok/first-look.py product $P --cli reports/grok-first-look-2026-10-11/cli/wipemark-cli \
    --date 2026-10-11 --invert-dark --control
# the laws' rows: add --rows law --out reports/grok-first-look-2026-10-11-law to the first and the same --out to the others
```
