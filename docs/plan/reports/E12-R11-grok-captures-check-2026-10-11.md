# E12-R11 — the owner's first captures from X, checked against their prompts

*2026-10-11. Task `wipemark-task-recon-grok-captures-check-2026-10-11`, branch
`recon/grok-captures-check` from `origin/recon/grok-fdncnn` at `2986012`. No Rust
changed, no decision taken. Every figure below is made by
`scripts/grok/captures-check.py check` (committed with this report); its tables,
the sheet of the corners and the α maps are under the ignored
`reports/grok-captures-check-2026-10-11/` and never enter git.*

## Для владельца

Пришло пять картинок, а не четыре: спичка, астронавт, медуза, кошачьи глаза и
маяк. Кошка скачана дважды, файлы побайтово одинаковые. Все пять на чёрном
фоне. **Ни одна не вышла того размера, что ждали промпты.**

| что просили | что пришло |
|---|---|
| три «альбомных» (16:9) промпта | спичка и астронавт — **портрет** 784×1168; медуза — альбом 1168×784 (это 3:2, а не 16:9) |
| два «портретных» (2:3) | оба 1152×1712 — пропорция та же, но крупнее, чем 784×1168 |

**16:9 в этой партии не было ни разу. 3:2 пришло только как маленькое
1168×784, а не 1728×1152.** При этом утренние картинки, тоже из X, были
1792×1008 (16:9), 1728×1152 (3:2) и 1600×1200 (4:3). Вывод одной строкой:
**пропорцию в X заказать нельзя — выбирается только альбом или портрет, размер X
решает сам; 16:9 и 3:2 он иногда отдаёт, но не по просьбе.** Два альбомных
промпта из трёх вернулись портретом. Либо переключатель ориентации остался на
«портрете», либо X его не послушал. Перед каждой генерацией стоит смотреть на
переключатель.

**Для карты знака эти картинки отличные.** Угол под знаком — ровный ноль, без
градиента и виньетки. Grok послушался: сцена сверху слева (у портретных —
сверху по центру, как и просили), в правом нижнем квадранте ничего нет. Знак
везде светлый, полупрозрачный белый. Прозрачность в толще штриха 0,36–0,37 —
ровно та же, что у тёмного знака на сером в первом разборе. Две картинки одного
размера дают одну и ту же карту до последнего пикселя: знак накладывается
каждый раз одинаково. На малых размерах Grok рисует знак чётче, чем выходит у
большой карты при уменьшении. Значит, на каждый размер нужна своя карта, и эти
снимки дают её для 784×1168, 1152×1712 и 1168×784.

grok.com здесь не поможет: видимый знак Grok ставит только в X, на grok.com
знака нет, только метаданные (ваши слова от 2026-10-11).

**Что ещё нужно** (промпты v2, имена `black-land-…`, `white-port-…`, иначе
файлы приходят с номерами X и сопоставлять их приходится по сюжету):

1. **Белые** — `white-land-01…03` и `white-port-01…03`. Это тёмный вариант знака
   на ровном фоне. Только пара «чёрный + белый» одного размера покажет, что у
   двух вариантов одна форма и одна прозрачность. Такой пары пока нет ни для
   одного размера.
2. **Ещё 2–3 чёрных альбомных** (`black-land-…`). Альбомный размер пока один,
   1168×784. Если X отдаст 1792×1008, 1728×1152 или 1600×1200, у каждого будет
   своя карта.
3. **Серые** — `gray-25-…`, `gray-50-…`, `gray-75-…`, как в v2 (7 штук). Они
   покажут, где Grok переключает чёрный знак на белый.

Каждый новый размер, который отдаст X, — это своя строка в профиле, так что
оставлять стоит все удачные снимки.

## 1. The files

| picture | scene (prompt) | sha256 | bytes | W×H | W/H | nearest |
|---|---|---|---|---|---|---|
| `HUW5egmXMAAbDzo` | the match (black, 16:9 / landscape) | `210daf15fc311bf6066c8846be1ac5fecd43252ae75ef2afaf3d3642413c59e1` | 37 584 | 784×1168 | 0.6712 | 2:3 (+0.7 %) |
| `HUW5Mw-WIAAPXqN (1)` | the astronaut (black, 16:9 / landscape) | `53070d1adf4bda15c976d6123057e9dc16b2ad6c7fa0775fc4063d801fcda713` | 19 388 | 784×1168 | 0.6712 | 2:3 (+0.7 %) |
| `HUW5_31W4AAH8IY` | the jellyfish (black, 16:9 / landscape) | `33d4accf38ccefe33fd92df6db2c7f177b8cde9ff8f7f3a34cb68696238bc9d8` | 52 368 | 1168×784 | 1.4898 | 3:2 (−0.7 %) |
| `HUW6njDWAAAGSxF` (and `… (1)`, the same bytes) | the cat's eyes (black, 2:3 / portrait) | `efd6b6529c932748820c16f68f2256406fbceec2f9e1ed9a44dc5b11bd2c6332` | 33 858 | 1152×1712 | 0.6729 | 2:3 (+0.9 %) |
| `HUW62vIXAAAI21T` | the lighthouse (black, 2:3 / portrait) | `c57874d36fd29f7fe89bd9c09e08d7ddcfed4ad142aeaeaed85367832789ffb6` | 29 346 | 1152×1712 | 0.6729 | 2:3 (+0.9 %) |

All five: **JPEG, progressive, 4:2:0, 10 scans, libjpeg quality 95** — the
quantisation tables equal, both of them, to the first five's (`fixtures/image/grok-x/`)
and to libjpeg's standard tables at q95 exactly. **One block before the image
data, APP0 JFIF; no EXIF, no XMP, no C2PA; nothing after the end-of-image
marker.** X's re-encode, as before. The small sizes (19–52 kB) are the flat
black.

Where they were: `/workspace/grok-images/x/` (there is no `/workspace/grok-captures/`),
landed 14:52–15:04 UTC, named by X's media ids. The prompts' files
(`wipemark-grok-generation-prompts-2026-10-11`, 15:07, aspect ratios; `-v2-`, 15:10,
landscape/portrait — the same scenes under both) were uploaded after the pictures,
so which one the owner followed is not known; the table carries both readings. The
scene ↔ prompt mapping is by eye and unambiguous.

## 2. The size against the one asked

| prompt | asked (v1) | asked (v2) | given |
|---|---|---|---|
| the match | 16:9, 1792×1008 | landscape | **784×1168, portrait** |
| the astronaut | 16:9, 1792×1008 | landscape | **784×1168, portrait** |
| the jellyfish | 16:9, 1792×1008 | landscape | 1168×784, landscape (3:2) |
| the cat's eyes | 2:3, 784×1168 | portrait | 1152×1712, portrait (2:3) |
| the lighthouse | 2:3, 784×1168 | portrait | 1152×1712, portrait (2:3) |

**In one line: Grok in X gives no ratio on request — in this batch no 16:9 at all,
3:2 only as 1168×784 (not 1728×1152), 2:3 at a size of its own (1152×1712); the
same channel gave 16:9, 3:2 and 4:3 this morning, so X can deliver them but they
cannot be asked for.** Sizes seen from X so far: landscape 1792×1008, 1728×1152,
1600×1200, 1168×784; portrait 784×1168, 1152×1712. 1168×784 is 784×1168 turned.

## 3. The mark and the corner under it

The template is the first look's (the flat grey `HUVfLiSW0AEtmX_`, 149×57 at
(1634, 940)); its law: width 0.08315·W, margins 9 / 11 px.

| picture | the law's place: x, y, w | NCC | the first look's row (same size) | found: x, y, w | NCC | found − law (x, y, w) | margins r, b | w/W | variant |
|---|---|---|---|---|---|---|---|---|---|
| match, astronaut 784×1168 | 709.81, 1132.06, 65.19 | +0.194 | `HUVfYqgWkAAOhiJ` (708.19, 1131.12, 64.56): NCC +0.971 | 708.12, 1131.12, 64.62 | 0.971 | −1.69, −0.94, −0.56 | 11.25, 12.15 | 0.0824 | **light** (logo 255) |
| jellyfish 1168×784 | 1061.88, 735.85, 97.12 | +0.663 | none | 1060.81, 735.50, 96.69 | 0.977 | −1.07, −0.35, −0.43 | 10.50, 11.51 | 0.0828 | **light** |
| cat, lighthouse 1152×1712 | 1047.21, 1664.36, 95.79 | +0.569 | none | 1045.81, 1663.69, 95.69 | 0.981 | −1.40, −0.67, −0.10 | 10.50, 11.71 | 0.0831 | **light** |

The two pictures of each size put the mark on the same pixel. The first look's row
for 784×1168, measured on a textured blue, is right to 0.07 px. **The law of
constant margins is 1–1.7 px off at these sizes. Over all ten pictures (the first
five and these), the margins fall linearly with the width.** Right margin =
13.01 − 2.210·W/1000 (worst 0.08 px). Bottom = 13.06 − 1.188·W/1000 (worst
0.16 px). Width = −0.99 + 0.08374·W (worst 0.20 px; ∝ W alone, worst 0.54 px).
A search for an unseen size should take these, not 9 / 11.

**The ring** (`scripts/corpus/ring.py`'s check: 8 px outside the mark's whole-pixel
rectangle, a flat group passing under σ 8 levels in every channel), and **the
corner** (luma over the box two mark-widths left and three mark-heights above the
mark to the edges, the mark left out):

| picture | ring mean R, G, B | ring σ | looks like | ring.py, group black | corner mean ± σ | slope x, y (levels/100 px) | drop | curvature | at 0 | flat |
|---|---|---|---|---|---|---|---|---|---|---|
| astronaut | 0.06 | 0.32 | black | pass | 0.00 ± 0.02 | +0.000, +0.002 | 0.00 | 0.00 | 100 % | **yes** |
| jellyfish | 0.02 | 0.20 | black | pass | 0.00 ± 0.00 | 0, 0 | 0.00 | 0.00 | 100 % | **yes** |
| match | 0.07 | 0.33 | black | pass | 0.00 ± 0.04 | +0.003, +0.005 | 0.01 | 0.01 | 100 % | **yes** |
| lighthouse | 0.02 | 0.19 | black | pass | 0.00 ± 0.06 | −0.000, −0.004 | 0.01 | 0.00 | 100 % | **yes** |
| cat | 0.02 | 0.19 | black | pass | 0.00 ± 0.04 | −0.001, −0.001 | 0.01 | 0.00 | 100 % | **yes** |

Every corner is pure 0 (every pixel of every corner box, the mark left out). There
is no gradient, no vignette and no glow. The ring's σ of 0.2–0.3 is JPEG's ringing
off the mark's edges.

## 4. The opacity, read directly

On black under a light mark: `α = (I − B)/(255 − B)`, `B` the ring's quadratic
(≈ 0). The first look's map, from the dark mark on flat grey at 1792×1008, has a
core median of 0.363 and a maximum of 0.410. *Drawn here* is that map drawn at
this picture's place and width by the area integral.

| picture | α max | α p99 | α core median (px) | R, G, B at the core | drawn here: core, max | NCC | slope: all, core | RMS after the slope | support at 0 |
|---|---|---|---|---|---|---|---|---|---|
| astronaut 784×1168 | 0.416 | 0.408 | **0.364** (165) | 0.364 ×3 | 0.345, 0.360 | 0.970 | 0.938, 1.050 | 0.053 | 11 % |
| match 784×1168 | 0.415 | 0.408 | **0.364** (165) | 0.364 ×3 | 0.345, 0.360 | 0.970 | 0.937, 1.050 | 0.053 | 11 % |
| jellyfish 1168×784 | 0.420 | 0.400 | **0.365** (379) | 0.365 ×3 | 0.351, 0.370 | 0.977 | 0.950, 1.032 | 0.050 | 10 % |
| lighthouse 1152×1712 | 0.424 | 0.395 | **0.361** (379) | 0.361 ×3 | 0.350, 0.370 | 0.980 | 0.958, 1.032 | 0.046 | 10 % |
| cat 1152×1712 | 0.424 | 0.395 | **0.361** (379) | 0.361 ×3 | 0.350, 0.370 | 0.980 | 0.958, 1.032 | 0.046 | 10 % |

* **The opacity agrees with the first look.** The core is 0.361–0.365 against
  0.363, and the three channels are equal (a white logo). That fits a logo of 255
  only if the two variants are one α. A logo of 245 would put the light core at
  0.379. Black alone gives only `α·L`, so `L` = 255 is supported here, not
  measured.
* **The shape at these sizes is not the 149-px map drawn down.** The maximum is
  0.415–0.424 against 0.360–0.370 for the drawn map. The core reads 3–5 % over the
  drawn map, while the whole support reads 4–6 % under it (NCC 0.970–0.980, RMS
  0.05). Grok renders the mark at the picture's size, sharper than an area-drawn
  reduction. This is the first look's "the 149-px map drawn down to 65 px is not
  what Grok drew at 65 px", now measured on a clean background, so each size wants
  its own map. These pictures give one each for 784×1168, 1152×1712 and 1168×784
  (`reports/grok-captures-check-2026-10-11/alpha/*-alpha-Y.png`, 16-bit).
* **10–11 % of the support reads 0.** These are the faint fringe, α under half a
  level (≈ 0.002), which an 8-bit black cannot show.

**Pairs of one size**, α against α pixel for pixel:

| size | a | b | NCC | RMS | max \|Δ\| | core a, b |
|---|---|---|---|---|---|---|
| 784×1168 | astronaut | match | 1.0000 | 0.0001 | 0.0002 | 0.364, 0.364 |
| 1152×1712 | lighthouse | cat | 1.0000 | 0.0000 | 0.0000 | 0.361, 0.361 |

The mark is the same to the last level in two different generations of one size.
The pairs are both black, though. **There is no black-and-white pair of one size**,
so whether the dark and the light variants share one shape and one opacity *at one
size* is still open. The two variants are compared only across sizes (above).

## 5. Did Grok follow the prompt

The scene is luma more than 12 levels off the corner's level, the mark left out.

| picture | scene px (share) | box x0, y0, x1, y1 (of W, H) | centroid | cell | in the bottom-right quarter | in the lower half |
|---|---|---|---|---|---|---|
| astronaut | 13 523 (1.5 %) | 0.09, 0.06, 0.35, 0.33 | 0.18, 0.17 | upper left | 0 | 0 |
| jellyfish | 37 927 (4.1 %) | 0.03, 0.01, 0.38, 0.54 | 0.27, 0.29 | upper left | 0 | 286 |
| match | 28 328 (3.1 %) | 0.00, 0.01, 0.47, 0.42 | 0.17, 0.25 | upper left | 0 | 0 |
| lighthouse | 26 962 (1.4 %) | 0.27, 0.00, 0.55, 0.19 | 0.43, 0.07 | top centre | 0 | 0 |
| cat | 13 579 (0.7 %) | 0.15, 0.03, 0.85, 0.21 | 0.50, 0.07 | top centre | 0 | 0 |

Yes. The landscape prompts asked for the upper left and got it. The portrait
prompts asked for "the top of the frame" and got the top centre. Nothing is in the
bottom-right quarter of any picture. The jellyfish's tentacle tips reach 0.54 H, on
the left. The sheet of the corners for the owner is
`reports/grok-captures-check-2026-10-11/corners-x4.png`. Each row shows the
original | luma minus the ring's quadratic ×10 around mid grey, ×4. The pictures
small, with the scene's box and the bottom-right quarter, are in `pictures.png`.

## How it was run

```sh
python3 scripts/grok/captures-check.py selftest
python3 scripts/grok/captures-check.py check '/workspace/grok-images/x/*.jpeg' \
    --asked HUW5egmXMAAbDzo=16:9/land,black HUW5Mw-WIAAPXqN=16:9/land,black \
            HUW5_31W4AAH8IY=16:9/land,black HUW6njDWAAAGSxF=2:3/port,black \
            HUW62vIXAAAI21T=2:3/port,black --date 2026-10-11
```

`--reference` and `--known` default to the first look's grey picture and the five
fixtures. numpy 2.5.3 and Pillow 12.3.0 from the user site, Python 3.13. The run
takes 3 s. The selftest covers:

* nearest ratios;
* libjpeg q95 and q80 read back;
* EXIF, XMP and C2PA named;
* a drawn mark found within 0.3 px;
* both variants;
* black and white rings flat;
* a gradient caught by the corner and not by the ring;
* α from black and from white within 0.02 of 0.36;
* the black-and-white pair agreeing;
* the scene's cell;
* a disc in the corner seen;
* a duplicate read once.

## Deviations

* **Five pictures, not four** (the fifth, the lighthouse, landed at 15:00). All five
  are checked; one of them was downloaded twice and is read once.
* **The pictures were not in `/workspace/grok-captures/x/`** but in
  `/workspace/grok-images/x/`, named by X's ids. The prompt behind each was read off
  its scene.
* `scripts/grok/run.sh`'s stage 0 was not run: it wants the guide's layout and
  names and a release build. `captures-check.py` reads the same facts off the
  bytes, and more of them (every block, both tables whole, libjpeg's quality).
* Nothing was built; no CLI, no product run. The product on these pictures is the
  next task's (`wipemark-task-grok-x-mark-2026-10-11`).
