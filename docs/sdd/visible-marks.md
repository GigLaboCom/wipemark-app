# Visible marks: removing the logo a generator stamps on a picture

This document studies how AI image generators stamp a visible mark on a
picture and how one can be taken off. It also sets out the architecture
Wipemark uses to do that for any vendor, starting with Google's Gemini
sparkle. It is written for whoever works on epic **E12** next year. The
plan of record for the epic is
[`docs/plan/E12-visible-marks.md`](../plan/E12-visible-marks.md).

**Provenance of this survey.**

* Written 2026-10-04 on branch `e12/visible-marks` at `afe94cd`.
* The reference implementation is `allenk/GeminiWatermarkTool` ("GWT"),
  read at **`7c6a99f`** (2026-07-28, v0.3.2). It is MIT, © 2024 AllenK
  (Kwyshell). It was read from a shallow clone in the session scratchpad;
  nothing from it is vendored by this document.
* The vendor facts come from web research on the same date, with every
  source listed in §2.
* The measurements in §1.6 come from a throwaway Rust spike. Its code is
  not committed, but every number it produced is recorded here.

Words used throughout:

* A **mark** is the visible thing: a logo, a word, a sparkle.
* A **profile** is our description of one vendor's mark as data.
* **Restoring** a pixel means recovering the value it had before the
  mark, by arithmetic.
* **Reconstructing** a pixel means inventing a plausible value for it.
  The two are never filed on the same shelf.

---

## 0. The answer in one page

* **A semi-transparent overlay can be undone exactly.** A vendor that
  stamps a fixed overlay computes `I = α·L + (1−α)·O` for each pixel and
  channel:
  * `O` is the original value;
  * `L` is the logo's colour;
  * `α` is a fixed per-pixel opacity map.

  If `α` and `L` are known, the original comes back as
  `O = (I − α·L) / (1 − α)`. On a lossless file this is exact to ±1 level.
  The spike measured a maximum error of 1 on 24 composites of real
  photographs (§1.6). GWT is that equation plus a calibrated `α` for
  Gemini.
* **An opaque mark cannot be undone.** Where `α` reaches 1, the original
  is gone. Only reconstruction (inpainting) can fill it, and that is
  invention. We report it as invention.
* **What changes between vendors is data.** The opacity map, the logo
  colour, where the mark sits as a function of image size, and how sure a
  match must be are all values. A new vendor is a calibration run and one
  catalogue entry, never new code (§4).
* **Removing a visible mark removes nothing invisible.** SynthID is on
  Google's images. OpenAI announced it would add SynthID to its own images
  from 2026-05-19, according to press coverage. Meta and Adobe embed their
  own. All of these survive this pass. The report's third shelf says so
  every time (§6).
* **Evidence for vendors other than Google is thin.**
  * For OpenAI, a visible mark on ChatGPT images is **not established**:
    there are app-teardown strings and no announcement.
  * For Grok, it is **reported but unmeasured**.

  The foundation is built so that the owner's captures (§4.6) turn either
  into a profile in an afternoon.

## 1. GeminiWatermarkTool in depth (at `7c6a99f`)

### 1.1 The equation and its guards

`src/core/blend_modes.cpp:33-108`, `remove_watermark_alpha_blend`:

```
original = (watermarked − α·logo) / (1 − α)      per channel, in float
skip       α < 0.002                             (noise floor)
clamp      α ≤ 0.99                              (never divide by ~0)
clamp      result to [0, 255], then convertTo(CV_8UC3)   (rounds to nearest)
```

* **The logo is white.** `logo_value` defaults to 255.0 and is one
  scalar for all three channels (`watermark_engine.hpp:133-139`).
* **The space is the stored, encoded 8-bit values.** That is sRGB as
  stored, with no linearisation. The blend is undone in the same space
  Google evidently composites in: §1.6 shows a clean restoration of two
  real Gemini outputs under exactly this model.
* **The 0.99 clamp never fires for Gemini.** The largest `α` in any
  shipped Gemini map is 0.51 for V1 and 0.37 for V2 (§1.6). So nothing in
  Gemini's case relies on it.

  For a vendor with an opaque or near-opaque mark, the clamp would
  quietly produce a wrong value instead of admitting the pixel is gone.
  We replace it with a declared **opaque threshold** below which we
  divide and at or above which a pixel is a hole (D155).
* **The add path exists too** (`add_watermark_alpha_blend`, `:110-174`).
  GWT can *stamp* a mark. We do not ship stamping as a feature. In tests
  we use the same arithmetic to composite synthetic marks.

### 1.2 How the opacity maps were calibrated

GWT computes `α = max(R, G, B) / 255` of a stored capture
(`blend_modes.cpp:6-31`, `calculate_alpha_map`). That is exact if, and
only if, the capture is the mark composited over **pure black** with a
**white** logo. Then `I = α·255`.

The README and the author's Medium write-up (title in §9; Medium refused
our fetch, HTTP 403) describe captures on black and white backgrounds.
The code uses one capture per size.

**What this misses.** A logo that is not pure white, or has a tint,
cannot be calibrated from black alone. The capture on black is `α·L`, and
`α` and `L` are confounded.

The spike composited a slightly blue-white logo `(242, 246, 255)`.
Restoring it under GWT's "white logo, black capture" assumption left
errors up to **14 levels**. Solving from a **black and a white** capture
recovered `α` exactly and `L` within 1 level, and restored to ±1.

So our calibration takes at least two backgrounds per pixel and fits both
unknowns (§4.6). It adds a third, mid-grey background to test the blend
model. A blend in linear light rather than encoded values fits black and
white equally well and differs by about 12 levels on grey at `α = 0.5`.

### 1.3 The assets

`assets/embedded_assets.hpp` holds four PNG files as C arrays. We
extracted them in the spike.

| array | size | type | `α` max | pixels with `α ≥ 0.002` | sha256 of the PNG |
|---|---|---|---|---|---|
| `bg_48_png` (V1 small) | 48×48 | RGB, channel spread ≤ 3 | 0.506 | 2027 | `4afc99af…7f0f57` |
| `bg_96_png` (V1 large) | 96×96 | RGB, channel spread ≤ 5 | 0.514 | 9013 | `3e26f223…077911` |
| `bg_b_36_png` (V2 small) | 36×36 | 8-bit grey | 0.329 | 1256 | `a3e7d5ca…68cd31` |
| `bg_b_96_png` (V2 large) | 96×96 | 8-bit grey | 0.369 | 9115 | `3911f3b6…8c5552` |

They are MIT, © 2024 AllenK. The README asks redistributors to keep the
copyright notice and the full licence text and recommends a link back.
The masks have been reused by most derivative removers we found:

* GargantuaX/gemini-watermark-remover (MIT) carries AllenK's copyright
  line in its LICENSE.
* The Python ports credit GWT.

§7 says how we ship them.

### 1.4 Profiles V1 and V2: where the mark sits

`watermark_engine.cpp:36-118`.

| | V1 (Gemini before "3.5") | V2 (from Gemini "3.5", May 2026) |
|---|---|---|
| large (`W > 1024 && H > 1024`) | 96×96, margin 64 from right and bottom | 96×96, margin 192 |
| small (otherwise) | 48×48, margin 32 | **inferred:** see below |
| opacity | `α` max ≈ 0.51 | `α` max ≈ 0.37 |

How V2's small placement is inferred:

* The "canonical" long side is guessed from {2752, 2816, 2848}:
  * from twice the long side when the long side is over 1100 (the
    free-tier half-scale outputs, GWT issue #40);
  * otherwise from the short side, with thresholds at 566 and 550.
* The margin is then `round(192 × long/canonical)`.
* The logo size is `round(96 × long/canonical)`, kept at 36 when that is
  40 or less, and interpolated from the 96 map otherwise.

Three things matter here:

* **Placement is a function of the output size.** A vendor changes it
  without notice: V2 broke GWT in May 2026 (issue #34), and free-tier
  half-scale broke it again in July (#40).
* **GWT encodes placement as C++ branches.** Every change has been a
  release.
* **V1 and V2 are the same shape at different opacities.** The detector
  cannot tell them apart (§1.6). GWT asks the user (`--legacy`), and
  since v0.3.1 retries V1 when V2 "skips". That retry only works because
  the detector is lenient (§1.5).

### 1.5 The detector

`watermark_engine.cpp:358-542`, `detect_one_variant`. All three stages
work on greyscale `[0, 1]` at the formula position, ±3 px for V2 small.

| stage | what it computes | role |
|---|---|---|
| 1. spatial | `matchTemplate(TM_CCOEFF_NORMED)`, the NCC of the image region against the `α` map | below 0.25 exits early (`:452`); at 0.60 or above, the snap position is trusted (`:445`) |
| 2. gradient | NCC of Sobel magnitudes, image against `α` | weight 0.30 |
| 3. variance | `1 − σ(region)/σ(strip above)`, "a watermark dampens texture" | weight 0.20 |
| fusion | `0.5·s + 0.3·g + 0.2·v`; then `max(fused, s)` if s ≥ 0.30 (`:527`) | "detected" at 0.35 (`:534`), but the **CLI gate is 0.25** (`cli_app.cpp:869`) |

The threshold is low. In the spike, the NCC of GWT's own 96 map against
**unmarked** photographs reached:

* **0.26** at a fixed anchor (Monument Valley);
* **0.44** when searched over a 200 px corner and four scales (Rainbow
  lightbulb).

Real marks scored 0.83–0.999. GWT's safety therefore rests on three
things:

* the fused score;
* the observation that a wrongly removed "mark" changes few pixels by
  little (`α ≤ 0.5`);
* the user's backup.

We want something stronger before writing a file: a **second,
independent proof** that the hypothesis is right (§1.6, D154).

**Guided search, the "snap"** (`:687-931`). It works coarse to fine:

* scales 32–160 in steps of 4, the top five candidates, then ±10 in steps
  of 2;
* the score is NCC × `cbrt(scale/96)`, to counter NCC's preference for
  small templates;
* integer positions only.

It exists for resized and recompressed images, and the README is candid
that the maths is then no longer exact.

### 1.6 What the spike measured

The spike was ~400 lines of Rust over `png 0.18.1` and `zune-jpeg 0.5.15`,
both already in our `Cargo.lock`. It used GWT's four maps and six Ubuntu
wallpapers (`/usr/share/backgrounds`) as unmarked photographs. Four real
Gemini outputs came from a fixture folder of one of the owner's projects;
they were read only and are not copied anywhere.

**Exactness of the equation.** We composited each map onto each photo
with 8-bit rounding, then restored it.

| | result |
|---|---|
| max abs error, 24 composites × 3 channels | **1 level** (never 2) |
| mean abs error | 0.05–0.24 levels |
| samples clipped at 255 by compositing | 0 |
| error by `α` bucket (V1 large) | max 1 in every bucket 0.0–0.6; mean grows with `α` (0.02 → 0.47), as `0.5/(1−α)` predicts |

**The detector cannot tell V1 from V2.** On a real V1 mark (`hero-man.png`,
1984×544, 48 px at margin 32):

| | V1 | V2 |
|---|---|---|
| NCC | 0.988 | 0.985 |

Restoring with V1 leaves a clean region: residual NCC −0.09, and the
before/after crops were checked by eye. Restoring with V2 leaves the
sparkle at residual NCC 0.95.

**Residual NCC is not a reliable verifier.** On an unmarked photograph
whose content happens to correlate with the template, the "true"
restoration leaves residual NCC +0.26 (Monument Valley). On one that
anti-correlates it leaves −0.47 (Rainbow lightbulb).

**Edge energy along the mark's contour is.** Define

```
E(img) = Σ |∇ gray(img)| · |∇ α|   over the mark's region
```

That is the image's gradient wherever the mark has an edge. Then sweep a
gain `k` and restore with `k·α`, measuring the unclamped values. The
correct hypothesis is the one where `k* = argmin E` sits at 1 and
`E(k=1)/E(marked)` is small.

| case (V1 large map, synthetic, 6 photos) | `k*` | `E(1)/E₀` |
|---|---|---|
| true mark | 1.00 (5 of 6) | 0.004–0.10 |
| true mark + uniform ±3 noise (a JPEG stand-in) | 1.00 (5 of 6) | 0.025–0.16 |
| unmarked photo | 0.00 (5 of 6) | 1.0 or more |
| an **opaque white star** of the same shape (a look-alike) | 0.00 (all 6) | ≥ 1.0 (NCC 0.86–0.995!) |
| the mark at 0.72× opacity (the "other variant") | **0.72**, exactly (5 of 6) | 0.26–0.79 at `k = 1` |

The "1 of 6" in every row is the same photograph, Rainbow lightbulb.
There `k*` came out at 1.58–1.60 in every case, marked or not. The
likely cause is that the `k > 1` inverse leaves the range and the
spike's clamping erased the edges it was measuring. The verifier must
therefore measure the **unclamped** inverse and count out-of-range
values. Even as measured, the gain test refused that photograph in every
row. The decision fails *safe*: refused, never damaged.

On the real Gemini files, with the canonical 48 map at the canonical
position:

| file | `k*` | `E(1)/E₀` | out of range |
|---|---|---|---|
| `hero-man.png` | 1.00 | 0.067 | 0 |
| `hero-philosopher-1.png` | 1.00 | 0.034 | 0 |

With a **resampled** map (48 px from the 96 map) the same files gave
`k* = 1.04` and a ratio of 0.14–0.16. A canonical, calibrated map per
size is worth having; resampling is for search, not for claiming
exactness.

**Two real files carried two overlapping sparkles.** These were
`hero-philosopher.png` (1808×592) and `Gemini_Generated_Image_….png`
(1984×544). An earlier output had been fed back to Gemini, and the old
mark is now part of the generated content at a slightly different place
and size. This is GWT issue #20.

* Iterative detection found both: 48.5 and 44.5 px, 57 and 37 px.
* Subtracting both left visible outlines, because the baked-in one is
  resampled and is no longer a blend.
* The verifier refused all of them: `E(1)/E₀` of 0.62 and 1.1, `k*` far
  from 1.

This is the case a product must **report and not write**.

**Fitting a gain to zero the residual NCC** gave `k = 1.18` and
`k = 1.25` on those two files. It "succeeded" while leaving a ghost.
Never accept a gain-fitted restoration as exact.

### 1.7 The rest of GWT, and what we take

| part | essential? | ours |
|---|---|---|
| the reverse blend | **yes**: it is the method | kept, with a declared opaque threshold instead of a silent 0.99 clamp |
| calibrated `α` maps | **yes**: they are the data | Gemini's four, shipped as data with attribution (§7); later vendors calibrated by our own tool |
| white logo, one scalar | incidental | `L` per profile, per channel; a per-pixel colour map allowed |
| placement as C++ branches | incidental | a placement table + a bounded search, as data (§4.3) |
| 3-stage fused detector, gate 0.25 | incidental (heuristic weights) | NCC with sub-pixel registration to *propose*; edge-energy verification to *accept* (D154) |
| `--legacy` / automatic V1 retry | incidental | each opacity is its own profile, chosen by verification |
| snap (integer, cube-root bias) | useful | a search that uses integral images for speed, then sub-pixel; used only when no table row matches |
| NS / TELEA / "soft" inpaint of residuals | useful later | a declared *reconstructor*, off by default, best-effort shelf (E12-7) |
| FDnCNN via ncnn (embedded ~1.3 MB) | no | not ported. A learned model would be a catalogue entry with role `pixel` and needs a runtime decision (D166) |
| `cv::imread(IMREAD_COLOR)` | harmful | it applies EXIF orientation and drops alpha, ICC and every metadata block. We work on the **stored** raster and carry everything (§5) |
| JPEG out at quality 100, WebP at "101" (lossless) | harmful for JPEG | a JPEG gains a full generation of loss. We patch only the touched blocks (§5.2) |
| simple mode overwrites the input | harmful | our retention rules: a result beside the file, the original set aside (CLAUDE.md) |
| exit codes 0 processed / 1 skipped / 2 failure | different | ours: 0 / 1 / 2 / 3, with *inconclusive is not clean* (§8.1) |

The ecosystem around GWT:

* **`allenk/gwt-integrations`** (MIT, v0.3.1) is a Claude/Codex skill
  and a FastMCP server. The server wraps the GWT binary as a subprocess
  with a 120 s timeout, and its tools take **file paths only**
  (`remove_watermark(input_path, output_path, …)`). Our MCP server takes
  no paths (§8.2).
* **`allenk/VeoWatermarkRemover`** (v0.6.5, 2026-08-10) applies the same
  reverse blend per video frame. Its masks come from "golden frame
  differencing" over 10+ on/off pairs, and its strength is estimated per
  shot by least squares with up to five bisection rounds per frame. The
  repository holds a README and binaries, **no source and no LICENSE
  file**, so nothing can be taken from it. Video is out of scope (D167).

## 2. Visible marks by vendor (as of 2026-10-04)

Legend:

* **reversible** means a semi-transparent overlay at fixed opacity. That
  is reverse blending, exact on a lossless file.
* **opaque** means only reconstruction can remove it.
* Where the evidence is weak, the cell says so.

| vendor · product | visible mark? | tiers | look · place · size | reversible? | evidence |
|---|---|---|---|---|---|
| **Google** Gemini app / Nano Banana (2.5 Flash Image, 3 Pro Image) | **yes**, by default. Since **Aug 2026** the user can switch it off (Settings › Media watermark). Not where a law requires the label; the regional details conflict between sources | from Nov 2025: free and AI Pro carry it, Ultra and AI Studio do not; the Aug 2026 toggle supersedes for most accounts | white four-point sparkle, bottom-right. V1: 48 px at margin 32 / 96 at 64. V2 (May 2026): 36/96 at 192 scaled | **yes**: semi-transparent, `α` ≤ 0.51 (V1), ≤ 0.37 (V2). Measured | **strong**: GWT source and our measurements; Google's blog; [PCWorld](https://pcworld.com/article/3213409/google-now-lets-you-nix-visible-gemini-image-watermarks.html), [Gigazine](https://gigazine.net/gsc_news/en/20260817-google-visible-watermarks/), [TechRepublic](https://www.techrepublic.com/article/news-google-gemini-visible-watermarks-optional/) |
| Google Gemini API | mixed | a Jan 2026 forum post by Google staff says AI Studio shows the star unless a paid key is linked; an Apr 2026 user says it persisted | as the app | yes | forum only ([discuss.ai.google.dev](https://discuss.ai.google.dev/t/regression-forced-visible-star-watermark-breaks-gemini-nano-banana-pro-image-to-image-and-flow-frame-to-video-workflows/114193)) |
| Google Veo (video) | yes; the same toggle | | "Veo" wordmark (legacy); a diamond 48×48 or 44×44 at 720p (current) | yes, per frame | VeoWatermarkRemover README. **Video is out of scope** |
| **OpenAI** DALL·E 2 | yes (historical) | all | five coloured squares, bottom-right | opaque | [Wikipedia](https://en.wikipedia.org/wiki/DALL-E), community forum |
| OpenAI DALL·E 3 (ChatGPT, API) | **none found**. A Feb 2024 press claim of a visible "CR" mark top-left was never seen in outputs | | | — | weak ([Neowin](https://www.neowin.net/news/openai-adds-watermarks-to-ai-generated-images-using-dall-e-3/) said so; no samples) |
| OpenAI ChatGPT image generation (GPT-4o, gpt-image-1 and later) | **not established** | would be the free tier only | unknown | unknown | app teardowns only: `image-gen-watermark-for-free` (Apr 2025), "Save without watermark" (Jul 2025). **No announcement** ([TechRadar](https://www.techradar.com/computing/artificial-intelligence/chatgpt-free-users-look-away-now-openai-is-testing-watermarks-on-image-generation-that-could-render-the-feature-redundant-unless-you-pay), [Android Authority](https://www.androidauthority.com/chatgpt-save-without-watermark-apk-teardown-3578789)) |
| OpenAI, every image, from 2026-05-19 | no visible mark announced | ChatGPT, API, Codex | — | — | C2PA plus Google **SynthID** invisible mark, and a "Verify" preview ([PetaPixel](https://petapixel.com/2026/05/20/openai-gets-serious-about-detecting-fake-images/), [TheNextWeb](https://thenextweb.com/news/openai-c2pa-synthid-ai-image-detection-watermark)); the primary openai.com post was unreachable (403) |
| OpenAI Sora 2 (video) | yes: an animated logo that **moves** around the frame, later with the account handle | everyone but ChatGPT Pro | moving | not by a fixed map. Existing removers detect it and inpaint | secondary. The Sora app closed 2026-04-26 and the API 2026-09-24 ([The Decoder](https://the-decoder.com/openai-sets-two-stage-sora-shutdown-with-app-closing-april-2026-and-api-following-in-september/)) |
| **xAI** Grok Imagine / Aurora | **reported yes**: "a small logo, usually in a corner". xAI's FAQ is quoted as saying images and video carry a Grok watermark and that removing it breaks the AUP | consumer app; the API may be clean (one vendor page) | **unmeasured**: no size, place or opacity published | **unknown** | **thin**: secondary pages only; the official FAQ answered 403 ([elser.ai](https://www.elser.ai/blog/grok-ai-video-watermark); remover sites) |
| Meta AI (Imagine) | yes, plus an invisible watermark and IPTC metadata | | visible marker, lower-left per press; the exact text is not confirmed | unknown | [Meta newsroom, Feb 2024](https://about.fb.com/news/2024/02/labeling-ai-generated-images-on-facebook-instagram-and-threads/) and press |
| Microsoft Bing Image Creator | yes | | logo bottom-left, plus C2PA | unknown | secondary |
| Microsoft Designer / M365 | an optional watermark setting | | | | [Microsoft support](https://support.microsoft.com/en-US/Privacy/include-a-watermark-when-content-from-microsoft-365-is-ai-generated) |
| Midjourney | none | | | — | secondary; IPTC/EXIF only |
| Adobe Firefly | none visible | | | — | Content Credentials plus invisible TrustMark ([Adobe](https://news.adobe.com/news/2024/10/aca-announcement)) |
| ByteDance Doubao | yes | app | "豆包AI生成" text bottom-right; one tool measures 140×35 and 180×40 | **reported semi-transparent**: an open-source skill captures `α` on black and reverses it | an OSS skill README only |
| ByteDance Seedream / Jimeng | app yes; API `watermark` flag | | corner text | unknown | weak |
| Alibaba Tongyi Wanxiang (API) | optional | `watermark` parameter, default false | "AI生成", bottom-right | unknown | [Aliyun docs](https://help.aliyun.com/zh/model-studio/text-to-image-v2-api-reference) |
| Kling | free tier yes; Standard and above none | | logo bottom-right, about 145×34 on 720×1280 | treated as opaque by existing tools (STTN inpainting) | secondary and OSS |
| Hailuo / MiniMax | free web tier yes | | unknown | unknown | secondary |
| Ideogram, Stability | none visible found | | | — | secondary |
| Leonardo | "watermark on free tier" | | unknown | unknown | one review |

**China's labelling measures, in force since 2025-09-01.** These are the
CAC measures together with GB 45438-2025
([CAC](https://www.cac.gov.cn/2025-03/14/c_1743654684782215.htm),
[Harris Sliwoski](https://harris-sliwoski.com/chinalawblog/chinas-new-ai-labeling-rules-what-every-china-business-needs-to-know/)).

* Every image needs both:
  * an **explicit** label, a prominent text "in an appropriate position";
  * an **implicit** label, in metadata or a watermark.
* Art. 9 lets a provider deliver content without the explicit label at
  the user's request, with logs kept.
* Art. 10 forbids anyone to "maliciously delete, alter, forge or conceal"
  a label.

The often-quoted size rules (text ≥ 5 % of the short side and the like)
**could not be confirmed**. This is the reason ByteDance and Alibaba
products carry text marks. It is also an owner question (Q-V6).

**What the table means for the design:**

* **Gemini is the only vendor whose mark is measured, reversible and
  common.** It is the first profile.
* **OpenAI, on current evidence, puts no visible mark on images.** There
  may be nothing to remove. The owner's captures settle it (§4.6).
* **Grok reportedly has one, of unknown opacity.**
  * If its mark is semi-transparent and fixed, it is one calibration run.
  * If it is opaque, reverse blending cannot help, and the only honest
    tool is a reconstructor whose output is reported as invented (E12-7).
* **Text marks from the Chinese vendors are the same problem.**
  Calibration is per size class, because text is laid out per size.

## 3. Invisible marks: what is known, and what we can claim

| scheme | who | what is public |
|---|---|---|
| **SynthID-Image** | Google, on every Gemini, Imagen and Veo output. Since 2026-05-19, per press, also OpenAI's images | A post-hoc, model-independent learned encoder/decoder ([Gowal et al., arXiv 2510.09263](https://arxiv.org/abs/2510.09263)). Over 10 billion images marked. A detector portal has been in early access since May 2025, and checks in Lens/Search since May 2026. Turning off Gemini's *visible* mark leaves SynthID and C2PA in place, per Google |
| C2PA soft binding | the C2PA 2.1 standard | A registered watermark that can recover a stripped manifest through a lookup API. The algorithm list includes Digimarc, Adobe TrustMark and Microsoft InvisMark; **SynthID is not on it** ([list](https://github.com/c2pa-org/softbinding-algorithm-list)) |
| Adobe TrustMark | Firefly | Open source; the README says MIT |
| Meta | Meta AI | An invisible watermark (undisclosed). Open research models: Video Seal and Watermark Anything (MIT), Stable Signature (CC BY-NC) |
| xAI | Grok | No public statement on an invisible mark: **not established** |

**Published attacks.** UnMarker (IEEE S&P 2025) reported more than 50 %
removal against SynthID, which DeepMind disputes. Regeneration attacks
provably remove small-ℓ2 marks (NeurIPS 2024). The WAVES benchmark is
another reference. GWT's author ran black-box tests
(`report/synthid_research.md`): SynthID survived noise, resizing,
super-resolution, frequency attacks and camera re-capture. It fell only
to 1-bit binarisation, destruction below 25 dB PSNR, or deep diffusion
repaint that changes the picture.

**What this product can honestly say.**

| shelf | content |
|---|---|
| **verifiable** | "A visible mark matching profile *P* was found at *rect*" with its scores. On a lossless file, "the marked pixels were restored by inverting the blend, verified by the edge test, max error ±1 level". Metadata findings are E11's |
| **best-effort** | Restoration on a lossy file (exact maths over values the codec already disturbed, error ≤ codec error × `1/(1−α)`). Any **reconstructed** pixel, counted |
| **not established** | **Invisible marks in the pixels are neither searched for nor removed.** Plus core's three: vendor-detector evasion, human authorship, schemes this build does not implement |

Never "undetectable", "clean" or "AI-free" about a picture. A picture
whose visible mark and metadata are gone still carries SynthID if it ever
had it. Saying otherwise is the one lie this product exists not to tell.
The report keeps the sentence even when no mark was found (D156).

---

## 4. Architecture

### 4.1 Crates and edges

```
                 wipemark-core   (the third shelf's ids)
                  ↑     ↑     ↑
   wipemark-image  wipemark-pixels   wipemark-intake (unchanged)
   (containers;    (marks: profiles, catalogue, detect,
    never decodes;  verify, restore, calibrate; NO codec)
    + reframe)          ↑
         ↑              │
         └──── wipemark-picture ────┘   (a picture file: decode → pixels → encode → reframe;
                    ↑                    png, zune-jpeg, image-webp; our JPEG block patcher)
            app · cli · queue (later)
```

**`wipemark-pixels`** (new) holds the vendor-neutral maths. It works on a
decoded raster (`Raster`) and a catalogue of profiles.

* It finds, verifies, restores and calibrates.
* It depends on `wipemark-core` and on `serde`, `serde_json` and `sha2`
  for the catalogue and its asset checksums. All of these are in the lock.
* It has **no image codec**. A test that composites a synthetic mark into
  a 64×64 array needs no PNG.
* `wipemark-pixels` never depends on `wipemark-image`, and neither
  depends on the other's subject. The dependency script gets
  `"wipemark-pixels": {"wipemark-core"}`.

**`wipemark-picture`** (new) handles a picture *file*. It turns PNG, JPEG
and WebP bytes into a `Raster` of the **stored** samples, runs the pixels
pass, and encodes the changed raster back:

* PNG to PNG;
* lossless WebP to lossless WebP;
* JPEG by patching only the 8×8 blocks the mark touched (§5.2).

It then asks `wipemark-image` to **reframe**: the original's blocks
around the new image data, filtered by the metadata scope. It also holds
the one orchestration function every surface calls:
`clean(bytes, &PictureOptions) -> (bytes, PictureReport)`. Its edges are
`{wipemark-core, wipemark-image, wipemark-pixels}` plus `png`,
`zune-jpeg`, `image-webp` and `sha2`, all already in the lock.

**`wipemark-image`** keeps its promise that it never decodes a pixel. It
gains one container writer, `reframe(original, new_image, &StripOptions)`.
That writer takes the *structure* blocks from `new_image` (a minimal file
the encoder made) and everything else from `original`:

* rendering blocks always;
* metadata blocks unless the scope removes them.

`reframe` concatenates blocks it already parses. The gate is that, with
`new_image == original`, `reframe` equals `strip`. Container knowledge
stays in one crate.

**Why not one crate?** `wipemark-image`'s gate is "the decoded raster's
sha256 is identical before and after", and its value is that it never
decodes. Folding codecs into it would make the crate whose job is "pixels
untouched" the crate that changes pixels. And `wipemark-pixels` without
codecs is the one place where vendor-neutral maths can be tested at
array speed, as `wipemark-core` is for text.

**Why not put the codecs in `wipemark-pixels`?** Then the calibration
maths, the detector and every unit test would pull three decoders and a
JPEG writer, and a future surface that only wants to *detect* in a
raster it already has (a window's preview) would pay for all of them.

**Applications and the queue depend on `wipemark-picture`.**
`wipemark-queue` gains it when the batch takes images (E12-8). Its edge
list in the dependency script grows by one, and nothing else moves.

### 4.2 Data: a profile is a row, a mask is an asset

The catalogue is `manifests/marks.v1.json`, beside `models.v1.json`. It is
compiled in with `include_str!`, as the model catalogue is. Its assets
live in `crates/wipemark-pixels/marks/`, also compiled in, each pinned by
sha256 in the catalogue. A test re-hashes every asset, and a mismatch is
a red suite, never a silent load.

```jsonc
{
  "schema": 1,
  "profiles": [{
    "id": "gemini-sparkle-v2",          // a format: never renamed, never translated
    "vendor": "google", "product": "gemini", "mark": "sparkle",
    "observed": { "from": "2026-05", "until": null },
    "status": "stable",                 // stable | provisional (calibrated from few captures)
    "blend": { "model": "encoded", "logo": [255, 255, 255] },
                                        // "encoded" = linear over stored values; "linear-light" refused until needed
                                        // "logo_map": "<asset id>" for a multi-colour mark
    "opaque_above": 0.95,               // α at or above: a hole, never divided (D155)
    "alpha": [
      { "id": "gemini-v2-96", "asset": "gemini-v2-96.wma", "sha256": "…", "size": [96, 96] },
      { "id": "gemini-v2-36", "asset": "gemini-v2-36.wma", "sha256": "…", "size": [36, 36] }
    ],
    "placements": [                     // tried first, in order; exact rect per observed output size
      { "when": { "min_width": 1025, "min_height": 1025 },
        "corner": "bottom-right", "margin": [192, 192], "alpha": "gemini-v2-96" },
      { "when": { "width": 1024, "height": 1024 }, "rect": [x, y, 36, 36], "alpha": "gemini-v2-36" },
      { "when": { "width": 1376, "height": 768 },  "rect": [x, y, 48, 48], "alpha": "gemini-v2-96", "resample": true }
    ],
    "search": { "corner": "bottom-right", "within": [320, 320], "sizes": [24, 160], "alpha": "gemini-v2-96" },
    "detect": { "min_ncc": 0.70 },
    "verify": { "gain": 0.06, "edge_ratio": 0.30, "out_of_range": 0.01 },
    "source": { "from": "allenk/GeminiWatermarkTool", "commit": "7c6a99f", "licence": "MIT",
                "copyright": "Copyright (c) 2024 AllenK (Kwyshell)" }
  }]
}
```

**Each opacity variant is its own profile.** So `gemini-sparkle-v1` and
`gemini-sparkle-v2` are separate, and verification chooses between them
(§1.6).

**`placements` are exact rows**, keyed by output size or a size range,
with a corner and margin or an explicit rect. GWT's inferred V2 small
rule becomes one row per output size we have seen, generated once from
its formula and checked against GWT's own code paths. A vendor's new size
is a new row from one capture, never a new branch.

**`search` is the fallback**, for resized or cropped files and for a size
no row names. Anything it finds is verified like the rest, but it can
never claim "exact" with a resampled map unless the profile marks the
resample as calibrated. The status line says *searched*.

**The asset format `.wma`** ("Wipemark alpha") has a 4-byte magic `WMA1`,
then `u16 width`, `u16 height` and `u8 depth` (8 or 16), all
little-endian, then `width × height` samples, row-major. `α` is
`sample / (2^depth − 1)`. It needs no codec.

* GWT's maps are stored at depth 8 with `sample = max(R, G, B)`. That is
  exactly the `α` GWT computes, and a dev-test decodes the original PNG
  and proves byte equality.
* Calibrated maps are stored at depth 16, because a regression gives more
  than 8 bits.

### 4.3 The flow

Steps 1–5 are pure functions in `wipemark-pixels`. Steps 0 and 6–8 are in
`wipemark-picture`.

0. **Decode** to the stored raster.
   * No colour management, no EXIF rotation.
   * Alpha is kept untouched.
   * 16-bit samples stay 16-bit.
   * Palette and grey are expanded to RGB for the maths, and encoding
     goes back to the original's colour type when the new values allow
     it.
1. **Propose.** For each profile, try every `placements` row that applies
   to `(W, H)`: the NCC of the region's luma against the `α` map, then a
   sub-pixel refinement (±3 px in position, ±0.5 px in scale, steps of
   0.25). If no row applies, or none reaches `detect.min_ncc`, run the
   bounded `search`. It is coarse to fine, uses integral images for the
   NCC means and variances, and takes the top five candidates.
2. **Verify.** For each proposal at or above `min_ncc`, sweep the gain
   `k ∈ [0, 1.6]`. Compute the *unclamped* inverse with `k·α` and the
   edge energy along the map's contour. Accept only when all three hold:
   * `|k* − 1| ≤ verify.gain`;
   * `E(1)/E₀ ≤ verify.edge_ratio`;
   * the share of samples whose `k = 1` inverse falls outside the range
     by more than one level is at most `verify.out_of_range`.

   A proposal that fails is a `Finding` with `Verdict::Refused { why }`:
   which test failed, and the numbers.
3. **Choose.** Proposals that overlap compete. The verified one with the
   lowest edge ratio wins, which is how V1 beats V2 on a V1 file. If two
   profiles both verify, the canonical-placement one wins over the
   searched one.
4. **Restore.** Only a `Verified` value can be restored. It is a token
   that only step 2 constructs, so the type system keeps "write only what
   you proved" (D154).
   * Divide where `α < opaque_above`, round to nearest, clamp.
   * Pixels at or above `opaque_above` become **holes**, listed and not
     touched.
   * A profile with holes cannot report "exact".
5. **Again, once.** Repeat steps 1–4 on the restored raster, at most once
   per profile (D165). This finds a second, overlapping mark. A mark
   baked into regenerated content will fail verification, as measured,
   and is reported.
6. **Encode** only if something was restored (§5).
7. **Reframe** with the metadata scope (§5.3).
8. **Prove.**
   * Inspect the output again (E11's `still_has_*`).
   * Decode it and compare with the input raster: *every* sample outside
     the restored rectangles must be identical. For JPEG, "outside" means
     outside the patched blocks, widened by one MCU for chroma upsampling.
   * Re-run detection on the output. A verified mark must no longer
     verify.

A failure here is a bug, refused as a value, and no file is written.

**Threads.** Every call is synchronous and blocking, like
`wipemark-llama`. The caller owns the thread: the CLI's own thread, the
MCP server's request thread, or GPUI's background executor, never the
foreground ("Nothing blocks the GPUI thread"). The budget is under
200 ms for a 2752×1536 PNG with a placement hit, excluding decode and
encode, and under 2 s with a search. E12-1 records both.

### 4.4 Types (sketch, binding for E12-1)

```rust
pub struct Raster { pub width: u32, pub height: u32, pub layout: Layout, pub samples: Vec<u16> }
pub enum Layout { Rgb8, Rgba8, Rgb16, Rgba16 }      // E12-1 implements Rgb8/Rgba8
pub struct Catalogue { /* profiles, decoded α maps */ }
impl Catalogue { pub fn shipped() -> &'static Catalogue; pub fn parse(json: &str, assets: &dyn Fn(&str) -> Option<&[u8]>) -> Result<Self, CatalogueError> }

pub struct Finding { pub profile: ProfileId, pub rect: SubRect, pub placed: Placed /* Row(i) | Searched */,
                     pub ncc: f32, pub verdict: Verdict }
pub enum Verdict { Verified(Verified), Refused(Refusal) }
pub struct Verified { /* private: profile, rect, map — only verify() builds one */ pub gain: f32, pub edge_ratio: f32 }
pub enum Refusal { Gain { k: f32 }, Edges { ratio: f32 }, OutOfRange { share: f32 }, Opaque { holes: u32 } }

pub fn examine(r: &Raster, c: &Catalogue, o: &ExamineOptions) -> Examination;   // steps 1–3, 5 (dry)
pub fn restore(r: &mut Raster, v: &Verified) -> Restored;                        // step 4
pub struct Restored { pub rect: PixelRect, pub changed: u32, pub holes: u32, pub clamped: u32, pub exact: bool }

pub struct PixelReport { pub found: Vec<Finding>, pub restored: Vec<Restored>,
                         pub not_established: Vec<&'static str> }               // never empty (D156)
```

Errors are values. Malformed catalogue rows, a raster with a mark region
that is not opaque in the alpha channel, and a layout E12-1 does not
implement are each a named refusal, never a panic.

### 4.5 Honesty in the types

* `PixelReport::not_established` always carries the new id
  `invisible-pixel-marks` (D156) plus core's three. The id needs a
  translation in all three catalogues; the i18n gate that already guards
  the shelf enforces it.
* `Restored::exact` is true only when all of these hold:
  * the source raster was lossless (the picture crate says so);
  * the map was a canonical row, not a resample;
  * there are no holes;
  * nothing was clamped beyond rounding.

  Otherwise the restoration goes on the best-effort shelf.
* A reconstructed pixel (E12-7) is counted in `Reconstructed { pixels,
  method }` and can never set `exact`.
* Vendor and product names in a finding are **identifiers**, like a
  Unicode character's name. They are shown beside the profile id and
  never translated; the sentence around them is catalogue text. No logo,
  no vendor artwork and no trademark appears in the user interface beyond
  naming what was found.

### 4.6 Calibration: how a new vendor becomes data

The tool is `cargo run -p wipemark-picture --example calibrate --
<captures dir> --out <dir>`. It is a developer tool like the prompt
bench, not a product surface, so it has no catalogue strings (D162).

**Inputs.** A folder of original downloads, never screenshots and never
re-saved, plus a `captures.toml` saying for each file:

* vendor, product, tier;
* how it was made (date, app version, web/iOS/Android/API);
* its background class, `black | white | grey | content`;
* optionally, a *pair* id for "the same generation with the mark on and
  off".

**The method.**

1. **Locate.** Difference each background capture against a median of
   itself to find the mark's support, then confirm the corner and the
   margins across captures of one size.
2. **Estimate the background under the mark.** Generated "flat" images
   are not flat: they have noise and vignetting. Fit a low-order
   polynomial (quadratic) to a ring of pixels around the support, per
   channel, and evaluate it underneath.
3. **Regress per pixel.** `I = (1−α)·B + α·L` is linear in `B`, so
   across N captures `I = a·B + c` with `a = 1−α` and `c = α·L`.
   * Two backgrounds determine the pixel; more average out compression
     noise.
   * Report `R²` and the residual per pixel.
   * `L = c/α` where `α` is large enough; a single colour if the
     channels agree, a `logo_map` otherwise.
4. **Test the blend model.** Predict the grey captures under `encoded`
   and under `linear-light`, and keep the model that fits. If neither
   fits within 2 levels, refuse: the mark is not a fixed blend (it may be
   opaque, animated or content-dependent).
5. **Find the opaque part.** Pixels with `α ≥ opaque_above`. A profile
   whose holes are more than 5 % of its support is reported as "needs
   reconstruction". It may still ship as **detect-only**: inspect
   reports the mark, clean refuses to restore.
6. **Pairs**, if any. `I_marked = α·L + (1−α)·I_clean` is the same
   regression with `I_clean` as `B`, over many natural backgrounds. That
   is the best calibration there is (VeoWatermarkRemover's "golden
   frames"), possible wherever a vendor lets the same generation be
   downloaded with and without the mark. Gemini has allowed this since
   the Aug 2026 toggle, if the toggle applies to an existing generation.
   Q-V7 asks.
7. **Replay.**
   * Restore every capture and every `content` image with the new
     profile. Report `k*`, the edge ratio and the residual on the flat
     ones (expect ±2 levels on PNG).
   * Run the false-positive corpus (§9) with the new profile.

   The output is the catalogue row, the `.wma` assets and
   `calibration-report.md`. A profile ships only with that report
   committed beside it (`marks/<id>.report.md`).

**What the owner must capture.** The same list is in Q-V7, in product
terms.

First, establish whether a mark exists (cheap): for each product and
tier you have, **three ordinary generations**, downloaded with the
product's own download button.

| product | tiers to try |
|---|---|
| ChatGPT | free, Plus/Pro; web and the iOS/Android apps; the API if available |
| Grok Imagine | free, SuperGrok/Premium; X app, grok.com, iOS/Android |
| Gemini | free, AI Pro, Ultra if available; the toggle on and off |

If there is no visible mark, there is no profile, and the report says
that vendor was checked on that date.

If there is a mark, then for **each output size the product offers**:

| capture | how | count |
|---|---|---|
| on black | a prompt such as "a completely flat, uniform, pure black image with nothing in it, no texture, no vignette". Better: if the product edits uploads, upload a pure black PNG of that exact size and ask for it "unchanged" | 3 (5 if the product outputs JPEG) |
| on white | the same with pure white | 3 (5 for JPEG) |
| on mid-grey | the same with `#808080` | 3 |
| content | ordinary prompts with busy bottom corners (foliage, text, stripes) | 5 |
| pairs (if possible) | the same generation downloaded with the mark on and off | 5 pairs |

* Sizes: ChatGPT's square, portrait and landscape (1024×1024,
  1024×1536 and 1536×1024 as of the API docs), and whatever the app
  produces. For Grok, every aspect the app offers. Also any half-scale or
  "preview" size a free tier produces, a lesson from GWT #40.
* Per vendor and tier that is about **14–19 files per size**, about
  50–80 files per vendor.
* Never pass the files through a messenger, a cloud photo app or a
  screenshot: those recompress and strip, and the calibration would learn
  the messenger.
* Record the date and app version. A vendor's change is a new profile
  with `observed.from`, and the old one keeps working on old files.

### 4.7 Video

Out of scope (D167). Veo's and Sora's marks are per frame, Sora's moves,
and video means a demuxer and an encoder. `Kind::Media` stays "not this
product".

## 5. Output fidelity

### 5.1 What survives, by container

| input | pixels out | how | what else changes |
|---|---|---|---|
| PNG (8 or 16 bit, RGB/RGBA/grey/palette) | lossless; only restored samples differ | decode with `png`; encode with `png` at the same bit depth. Same colour type when the new values allow it: a palette PNG whose restored pixels are not in the palette becomes RGB(A), and the report says so | IDAT re-compressed, so the size may differ. Everything else kept by `reframe` |
| WebP lossless (VP8L) | lossless | `image-webp` decode and encode (lossless) | the `VP8L` chunk; `VP8X` flags kept or recomputed |
| WebP lossy (VP8) | see Q-V3 | no pure-Rust lossy VP8 *encoder* exists in the lock. The proposal is to write **lossless** WebP of the decoded and restored raster: bigger, never a second loss. The alternative is to refuse | the container becomes VP8L. Said in the report |
| JPEG (baseline or progressive) | untouched blocks **bit-identical in their quantised coefficients**, so their decoded pixels are unchanged | the block patch (§5.2) | the entropy-coded data is re-written. APPn segments, DQT and SOF are kept |
| TIFF, HEIC, AVIF, GIF, BMP | refused, "not in this version yet" | | |

Rules that hold for every container:

* **Never colour-managed.** The vendor blended stored values, so we undo
  stored values. An ICC profile, `gAMA`, `cHRM`, `sRGB` or `cICP` is
  carried as-is by `reframe` and never interpreted (E11's `Rendering` is
  never removed).
* **Never rotated.** EXIF orientation is metadata. The raster is the
  stored one, and placements are in the stored frame because that is the
  frame the vendor wrote. The orientation tag survives unless the scope
  is `AllMetadata`, as in E11.
* **Alpha channel untouched.** If the mark's region is not fully opaque
  in the alpha channel, the blend's meaning is unknown and the profile
  refuses (`Refusal::Transparent`).
* **One writer.** When no mark is restored, the output is E11's
  `strip(bytes)` byte for byte, with no decode-encode round trip at all.
  A clean that finds nothing must not re-compress a file.

### 5.2 JPEG: patch the blocks, never re-encode the picture

A vendor JPEG was blended **before** compression. Restoring decoded
pixels is therefore exact arithmetic over values the codec already moved.
The error is the codec's own error amplified by `1/(1−α)`, at most 2× for
Gemini V1 and 1.6× for V2.

What *we* add on top is the choice that matters.

**Full re-encode.** GWT does this at quality 100. Every pixel of the
picture takes a second generation of loss, the file grows, and the
quantisation tables change. A 4 MP photo would be changed everywhere to
remove 96×96 pixels in a corner. **Rejected** as a default.

**Block patch, recommended.**

1. Decode the entropy-coded data to **quantised DCT coefficients**:
   baseline and progressive, every component, at its own sampling.
2. Decode pixels as usual, and restore.
3. Re-transform only the blocks under the restored rectangle, widened to
   whole MCUs. That means RGB to YCbCr with the file's own transform
   (respecting APP14), chroma subsampling by the file's factors, forward
   DCT, and quantisation with the file's own DQT tables.
4. Replace just those blocks' coefficients.
5. Re-write the entropy-coded data as **baseline sequential** with
   optimised Huffman tables, keeping DQT, the sampling factors and all
   APPn segments.

Every other block keeps its quantised coefficients exactly, so any
decoder produces the same pixels there, except for a decoder's fancy
chroma upsampling within one MCU of the patch. The gate tolerates exactly
that band and nothing more.

A progressive JPEG comes out baseline, which is the one structural change
and is said in the report. Re-writing the same coefficients progressively
is possible but adds a scan-script writer for no pixel benefit.

**No crate in `Cargo.lock` reads or writes coefficients.** `zune-jpeg`
and `jpeg-decoder` decode to pixels only. The options:

| option | for | against |
|---|---|---|
| **our own coefficient codec** in `wipemark-picture` (~1.2 k lines: Huffman decode of baseline and progressive scans, including refinement; the baseline writer; optimised tables) | pure Rust, no `unsafe`, owned, tested against `zune-jpeg` on every fixture | the largest single piece of E12. Progressive refinement scans are fiddly |
| `dct-io` 0.1.1 (MIT/Apache-2.0, pure Rust, Sep 2026) | exists | **baseline only**; three weeks old; one author |
| `lepton_jpeg` (Microsoft, Apache-2.0, pure Rust) | bit-exact JPEG to coefficients and back, both modes, security-reviewed | its API is the Lepton container, not a coefficient editor; a large dependency |
| `mozjpeg` (`jpeg_read_coefficients`) | the reference behaviour | C, a C toolchain, `unsafe`; against the repository's pure-Rust direction for this subject |

The recommendation is our own codec (E12-4). Read `lepton_jpeg`'s
decoder as a reference for the progressive cases, without copying it. If
the owner prefers it, there is a fallback (Q-V2): an explicit, opt-in
`--reencode-jpeg` that does a full re-encode at the file's own
quantisation tables and says so on the best-effort shelf.

**Grok downloads JPEG (unconfirmed).** This step decides how good a Grok
result can be.

### 5.3 The metadata pass and the pixel pass: order

* Metadata is inspected on the **original** bytes, because that is what
  the user's file says.
* Pixels are decoded from the original and restored.
* The result is written once, by `reframe(original, new_image_data,
  scope)`. So the metadata pass is the reframe's filter, and there is no
  second writer and no intermediate file.

Why one pass:

* A C2PA manifest **hard-binds** the pixels. After a pixel change it no
  longer validates, and keeping it would hand the user a file that claims
  provenance and fails its own check.
* So when a mark is restored, the default scope is at least
  `AiProvenance`. A request to keep metadata while restoring pixels keeps
  everything *except* C2PA, and the report says why.

That is D159's one exception to "the metadata the user did not ask to
remove is kept".

## 6. Honesty and the report

* **Verifiable:**
  * profile id, vendor and product, the rectangle, and how it was placed
    (row or searched);
  * NCC, `k*`, the edge ratio;
  * pixels changed;
  * `exact` with "±1 level" when it holds, otherwise it is not here;
  * E11's metadata findings.
* **Best-effort:**
  * restoration over lossy input, with its error bound stated;
  * any reconstruction, with the pixel count and the method;
  * a lossy WebP written as lossless;
  * a progressive JPEG written as baseline.
* **Not established**, every time and in every language:
  * invisible marks in the pixels (SynthID-class), neither searched for
    nor removed;
  * core's three.
* **Refused findings are findings.** "A mark like *profile* was seen and
  not removed, because the edge test failed (ratio 0.62)" is exactly what
  the user of a re-generated picture needs, and it decides the exit code
  (§8.1).

## 7. Licence and attribution

* **What comes from GWT** (MIT, © 2024 AllenK (Kwyshell)):
  * the four maps, converted losslessly to `.wma`;
  * the placement numbers, as table rows.

  Ported **code** would be small (the reverse blend is one line of
  algebra), but we treat it as derived and give the file a provenance
  header. This is the D45 convention from `local-engine.md`: the source
  path, the commit `7c6a99f`, what was taken and what was changed.
* **`NOTICE` gets a section**, "GeminiWatermarkTool masks and method",
  with the copyright line, the full MIT text and a link to the
  repository. That is what MIT requires, plus the attribution the author
  asks for.
* **`crates/wipemark-pixels/marks/README.md`** names each asset's origin,
  the original PNG's sha256 (§1.3) and the conversion.
* **Calibrated profiles are our own work.** Their reports name the
  captures, never the user's files.
* **No vendor artwork is shipped as an icon or image**, and no vendor
  logo is drawn in a window. A finding names the vendor in plain text as
  an identifier.
* **The vendors' terms are the owner's call.** xAI's AUP reportedly
  forbids removing its mark. China's Art. 10 forbids "maliciously"
  removing labels. Google ships a switch for the user's own images.
  Whether to ship a profile for a vendor whose terms forbid removal, and
  any wording in the product about using it on one's own content, is
  Q-V6. A watermark removal can also touch rules on copyright-management
  information (e.g. US 17 USC §1202) and the EU AI Act's Art. 50 marking
  duties, which fall on providers. This document claims nothing legal and
  flags it for review.

## 8. Surfaces

### 8.1 CLI

This builds on E11-2, where `inspect`, `clean` and `audit` take an image.

| command | E12 adds | exit |
|---|---|---|
| `inspect photo.png` | a "visible marks" section: each finding, verified or refused, with numbers | **1** if any visible mark was found (verified or refused) or any AI metadata; **0** otherwise |
| `clean photo.png` | restores verified marks by default (Q-V1); `--keep-visible` opts out | **0** when the output has no AI metadata and no verified mark is left. **3** when a mark was found and not removed (refused, holes, `--keep-visible`): the output is written with what could be done and the report says what is left, because *inconclusive is not clean*. **2** for refusals: unsupported container, malformed input, a layout not implemented |
| `clean --reconstruct` | E12-7: fill holes and refused regions by the reconstructor; best-effort | 0 if nothing verifiable is left; never claims exact |
| `audit dir/` | images carry visible findings; SARIF `region.byteOffset` is meaningless for pixels, so the rectangle goes in `properties` | 3 beats 1, as for text |
| `--json` | `visible: [{profile, vendor, product, rect, placed, ncc, gain, edge_ratio, verdict, exact, holes}]` in ASCII | |

### 8.2 MCP

* `inspect_image` and `clean_image` (E11-2) carry the visible pass. There
  is no new tool, and there is no `path` argument.
* The current transport limit is a 1 MiB body. After base64 that is about
  768 KiB of picture, and a 2752×1536 Gemini PNG is 3–8 MB. So **most
  real images are refused with 413 today**.
* Q-V5 asks whether to raise the limit for the image tools, on loopback
  only, or to accept a path on loopback only. Until it is answered, the
  refusal names the limit.
* The MCP pane's banner says the tools look at visible marks and
  metadata, and that invisible marks remain.

### 8.3 Windows and the queue (with E7)

* The queue's image row gets a badge per finding (vendor, verified or
  refused). The hover card carries the restoration sentence. The Compare
  window for images shows two panes, with the restored rectangle outlined
  on both and a 4× loupe on it.
* The batch queue runs `wipemark_picture::clean` as an item kind beside
  the pipeline job. One picture is one unit: no per-chunk resume, a
  picture is seconds.
* All of it runs on the background executor (CLAUDE.md, "Nothing blocks
  the GPUI thread"). A 96 px search on a 4K picture is the kind of loop
  that freezes a window.

## 9. Tests and gates

* **Exactness.** Composite each shipped map onto procedural rasters with
  8-bit rounding (gradients, noise, checkerboards, a photograph-like
  1/f texture made in the test), restore, and require **max error ≤ 1**.
  Do the same at 16 bit. No real photograph is committed.
* **The opaque threshold.** A synthetic map with `α` up to 1.0: pixels at
  or above `opaque_above` are untouched and counted as holes, and
  `exact` is false.
* **Verification.**
  * An opaque look-alike star is refused.
  * The 0.72× opacity variant is told apart, with `k*` ≈ 0.72.
  * A dark flat corner refuses rather than damages.
  * Two overlapping marks at different scales: the second pass finds the
    second, and a resampled one is refused.
* **False positives.** A corpus of at least 2000 procedural negatives
  (textures, text glyphs, stars and diamonds drawn opaque or blurred,
  white corners, JPEG-like noise) runs with every shipped profile and
  must act on none. Its maximum scores are printed into the test output
  and recorded in reports. `WIPEMARK_FP_CORPUS=<dir>` adds real
  photographs locally (e.g. `/usr/share/backgrounds`), never in CI and
  never committed.
* **Real samples.** Real vendor files go in fixtures only with a clear
  licence: the owner's own generations, which the owner releases for
  this purpose (Q-V8), recorded in `fixtures/image/README.md` with hashes
  like E11's.
* **Containers** (E12-3/4):
  * Every sample outside the restored rectangles is identical after a
    round trip (JPEG: outside the patched MCUs + 1).
  * The `iCCP`/APP2 ICC profile is byte-identical.
  * The second inspection agrees.
  * `reframe(x, x) == strip(x)`.
* **Mutations.** Every protection is deleted once, and the suite must go
  red. The table is in the plan, per step.

## 10. Risks

| risk | what to do |
|---|---|
| A vendor changes its mark without notice (GWT broke twice in 2026) | A changed mark fails verification and **nothing is written**. The failure mode is "found, not removed", never damage. Keep `observed.from`; the owner re-captures |
| False positives on content that looks like a mark | Two proofs, the second independent of NCC. The corpus gate; thresholds recorded per profile |
| JPEG coefficient codec complexity | E12-4 alone. It can ship after PNG and WebP; until then a JPEG with a mark is reported and refused (exit 3) |
| MCP's 1 MiB limit makes the tools useless for real pictures | Q-V5 |
| Legal and terms (§7) | Q-V6 before any non-Google profile ships |
| Over-claiming | `not_established` carries `invisible-pixel-marks` always; the i18n gate guards it |
| Evidence for OpenAI and Grok is thin | No profile without captures. "Checked, none found" is a valid outcome |
| GWT's V2 maps are of unknown capture quality (pure grey, zero channel spread: averaged or synthetic) | Re-validate with the owner's Gemini 3.5 captures (E12-6); recalibrate if the edge ratio on real files exceeds V1's |

## 11. Sources

* GWT: <https://github.com/allenk/GeminiWatermarkTool> at `7c6a99f`
  (README, `src/core/*`, `src/cli/cli_app.cpp`, `report/synthid_research.md`,
  issues #20, #24, #29, #32, #34, #36, #40); A. Kuo, "Removing Gemini AI
  Watermarks: A Deep Dive into Reverse Alpha Blending",
  <https://allenkuo.medium.com/removing-gemini-ai-watermarks-a-deep-dive-into-reverse-alpha-blending-bbbd83af2a3f>
  (403 to our fetcher). The method is read from source.
* <https://github.com/allenk/gwt-integrations> (MIT),
  <https://github.com/allenk/VeoWatermarkRemover> (README only),
  <https://github.com/GargantuaX/gemini-watermark-remover> (MIT, credits
  AllenK's masks).
* SynthID: Gowal et al., arXiv 2510.09263; <https://blog.google/technology/ai/google-synthid-ai-content-detector/>;
  <https://blog.google/technology/ai/ai-image-verification-gemini-app/>;
  <https://9to5google.com/2026/05/19/google-is-adding-ai-detection-for-photos-videos-and-audio-to-search-and-chrome/>.
* Attacks: UnMarker <https://arxiv.org/html/2405.08363v2>;
  <https://arxiv.org/abs/2306.01953>; WAVES <https://arxiv.org/abs/2401.08573>.
* C2PA: the 2.1 specification; <https://github.com/c2pa-org/softbinding-algorithm-list>;
  TrustMark <https://github.com/adobe/trustmark>; Meta
  <https://github.com/facebookresearch/videoseal>,
  <https://github.com/facebookresearch/watermark-anything>.
* OpenAI: <https://openai.com/index/understanding-the-source-of-what-we-see-and-hear-online>,
  <https://openai.com/index/advancing-content-provenance> (unreachable;
  press as above), <https://the-decoder.com/openai-equips-dall-e-3-with-c2pa-metadata-to-increase-transparency-of-ai-images/>.
* JPEG coefficient I/O: <https://docs.rs/dct-io/latest/dct_io/>,
  <https://docs.rs/lepton_jpeg>.
* The vendor table's own links are in §2.
