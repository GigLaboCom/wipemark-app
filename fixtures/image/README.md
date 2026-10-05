# Image fixtures

Four real files, byte-exact, for `crates/wipemark-image`. Everything
else the image tests need — a C2PA chunk in a PNG, an XMP packet with
an AI Digital Source Type, a Stable Diffusion `parameters` block, a
ComfyUI workflow — is **built** by `crates/wipemark-image/tests/support/`
around a tiny picture encoded at test time, so it is reviewable as code
rather than as a blob.

| file | bytes | sha256 | origin | what it carries | what is asserted |
|---|---:|---|---|---|---|
| `c2pa-jumbf.jpg` | 86 499 | `7c91641416c18319b823c292ae603c5354892ac365c05543519154c48c6a1f8a` | `sdk/tests/fixtures/no_alg.jpg` | a C2PA manifest store in APP11 (JPEG XT / JUMBF), JFIF, a camera EXIF | the manifest is found and removed by default; EXIF stays; the raster is unchanged. Its JUMBF is also what the tests embed as a PNG `caBX` and a WebP `C2PA` chunk |
| `xmp-provenance.jpg` | 97 591 | `9ac395ca04fc9d348acf6f81920f5e894d336341c3f764ca86c354eec6f7c2d6` | `sdk/tests/fixtures/no_manifest.jpg` | a progressive JPEG whose manifest was removed but whose XMP still says `dcterms:provenance="self#jumbf=/c2pa/…"`; Photoshop IPTC (APP13), an ICC profile, EXIF, APP14 `Adobe` | the XMP packet is a C2PA reference and goes by default; IPTC, ICC and EXIF stay |
| `xmp-provenance-url.png` | 4 104 | `8ffdf61c6497696b4a18f8e04b416c749f7ad67f4a1529862f50231db86d1411` | `sdk/tests/fixtures/libpng-test_with_url.png` | a palette PNG with an `iTXt` XMP packet pointing at a remote manifest (`dcterms:provenance="http://localhost:5000/…"`) | the packet goes by default; the file is small enough to cut at every offset |
| `exif-xmp.webp` | 25 020 | `769bf38d77496cc610ad0a0a5515f77e5c0ac185b64f99e4fa1ac4ede6dc290b` | `sdk/tests/fixtures/test_xmp.webp` | an extended lossy WebP: `VP8X`, `ICCP`, `EXIF`, an odd-length `XMP ` with its pad byte, Photoshop's `PSAI` | nothing is AI: the default strip returns it byte for byte; `AllMetadata` removes EXIF, XMP and PSAI, keeps ICCP and clears exactly the two `VP8X` bits |

**Origin.** The C2PA project's Rust SDK, `contentauth/c2pa-rs`, at commit
`e4f63a233c3b5bd6f5fcd308a0d0db95ab35eae4` (2026-10-04), directory
`sdk/tests/fixtures/`, fetched over `raw.githubusercontent.com` and
renamed here for what they carry, otherwise unchanged — the sha256
above is of the file as fetched.

**Licence.** `c2pa-rs` is distributed under the MIT licence and the
Apache License 2.0 (its `LICENSE-MIT` and `LICENSE-APACHE`, © 2020
Adobe); the repository carries no separate notice for its test
fixtures, so they are taken under the same terms. These files are test
inputs only: nothing ships them, and `NOTICE` (E10) need not list them.

No test reaches the network. Two rules from `../README.md` apply: the
files are byte-exact (`.gitattributes` marks them binary), and every
file here has a claim — `crates/wipemark-image/tests/signals.rs`,
`the_real_files_say_what_they_carry`, and every test that walks
`support::REAL`.

## `gemini/` — real visible marks

Six of the owner's own Gemini outputs (stickers made on 2026-04-24,
`heretic-videos/images/stickers/`), for `crates/wipemark-picture`
(`tests/real.rs`), the CLI (`tests/visible.rs`) and the MCP server.
The owner asked for them on 2026-10-04 in place of composites, which
supersedes Q-V8's "no vendor file is committed" for these. Each
2048 × 2048 picture is cut to its bottom-right **1025 × 1025** and
saved as an RGB PNG — the vendor's 96-pixel V1 mark is then exactly at
the large row (margin 64), so a row is tested on the pixels the vendor
stamped, at a twentieth of the bytes; the confetti WebP is as it was.
Seven JPEGs and a WebP were made here on 2026-10-05 by Pillow 12.3.0,
not handed out by the vendor: three JPEGs from the committed PNGs, colour
subsampled 4:2:0 as most JPEGs are; one from `torch-1025.png` at 4:4:4;
and three JPEGs and the lossy WebP from the 2048 originals cut to their
bottom-right **1040 × 1040** — the cut starts at
1008, 63 × 16, so the codec's 16-pixel blocks fall on the mark exactly
as in the 2048 file, and every figure measured on one is the other's
(D252). In a 1025 crop the mark sits at 865, a pixel off that grid.

| file | bytes | sha256 | from | what is asserted |
|---|---:|---|---|---|
| `crying-1025.png` | 558 672 | `ef0da91aa48a491e5a9017cf774e71506867f125ebfd9d832f8d22a5103a5bab` | `11_crying.png` (a re-saved copy: no C2PA, a flattened background) | the mark is proved at its row and restored, and the outline it leaves on the flat background is said: the mark counts as left, `clean` exits 3 (D244) |
| `torch-1025.png` | 934 572 | `a81716a84e0117d5c895e09c0addef0740cdbce497a4972308d3756c0adc20e3` | `05_torch_and_cross.png` | proved at its row and restored, nothing left; also off its row (the search), as JPEG 90/95 and shrunk with its picture; over MCP and the CLI; held out of the map's fit: no ghost, no outline, no square |
| `anchor-green-1025.png` | 1 053 895 | `48ffc44b91899766a738a07ec26e2ff053d69b05a4ccc5ea7e9093ca15b7b1f8` | `alt-anch/anchor-alternative.png` (C2PA intact) | the mark over a saturated green, the original at 0 in red and blue: proved and restored, the clamped channels counted (D240) |
| `victory-1025.png` | 1 068 039 | `f79a575baa38d91aa53026d9420ee68026e3418786b0ad348d8f87135ffeb2a9` | `19_victory.png` | as `torch`, but for MCP and the CLI |
| `torch-1025-q95-420.jpg` | 139 059 | `4f14601af46d7a44bebb932548ad8d63aca10cf04804f382ab8d03fc583ab259` | `torch-1025.png`, saved by Pillow 12.3.0 at quality 95, `subsampling=2` (4:2:0, Pillow's default) | proved and restored; the colour fringe the inverse leaves — red +11, green −6.5, blue +6, luma +0.2 — is said (`chroma` 8.4, D247), and its texture (D250): the mark counts as left |
| `victory-1025-q95-420.jpg` | 169 016 | `ec3f7054fa42a22ab854a16936b1c647a7cc3845ee579b82ccec13e48786f119` | `victory-1025.png`, the same way | seen and refused out of range before it is restored (D240): 1.06 % of the samples, the mark a pixel off the codec's grid (D252); left, exit 3 |
| `victory-1040-q95-420.jpg` | 177 057 | `e001118524d8c47fceb03cb4689fb06308b8cd5600e3707d69b6b27b156df4b4` | `19_victory.png` cut to 1040, saved at 95, `subsampling=2` | the same picture with the mark on the grid, as in the 2048 file: 0.91 % out of range, restored, its fringe said (`chroma` 7.8) — refused or said by where the blocks fall, never clean (D252) |
| `thinking-1040-q95-420.jpg` | 183 957 | `2bfa389774885d105db9669669a968f378865b66e224a1742889abd241584204` | `09_thinking.png`, the same way | the lowest colour step of the 21 at 4:2:0 95, `chroma` 7.40: said by its colour alone, which holds `CHROMA_LEVELS` from above (D253) |
| `torch-1025-q95-444.jpg` | 182 354 | `f7bd31ce30f752b39026d6b041165df907ab660ef246e0f7b526e7ed0f480e5f` | `torch-1025.png`, saved at 95, `subsampling=0` (4:4:4) | restored with no outline — luma +0.2, colour 2.3 — and an 8 × 8 checker along the contour, roughness 9.0 against 3.3 around it: the texture said, left, exit 3 (D250) |
| `victory-1025-q98-420.jpg` | 292 005 | `e54180d481e71ef2a86abfc5a1ee54d620b4259318fe6b5f93f06d9ab985630e` | `victory-1025.png`, the same way at quality 98 | as `torch` at 95: the fringe said (`chroma` 8.4) |
| `fine-1040-q98-444.jpg` | 392 670 | `4e3f72bb53b548471675d5200405c31c8916ff609c15dc90b8e280e68f67395b` | `10_this_is_fine.png` cut to 1040, saved at 98, `subsampling=0` (4:4:4) | the highest roughness of the 22 at 4:4:4 98, 5.22 against 1.99 around it — barely found at ×6 — restored and **not** said: it holds `TEXTURE_LEVELS` from below, a bound of 5.0 says it (D250) |
| `scroll-1040-q90.webp` | 54 928 | `3404fd6e596e98b8837a4946c52197b564d4ddc142bcd16103784ebb63f9da29` | `04_tearing_scroll.png` cut to 1040, saved lossy (`lossless=False`) at quality 90, `method=6`, by Pillow's libwebp 1.6.0 | a lossy WebP is held as lossy (D251): restored, "stored with loss" said, its texture said (10.1 against 1.9) and its colour fringe (6.4); left, exit 3 — written back as lossless WebP |
| `crying-transparent-1025.png` | 623 170 | `6cbc54505c68415faf21eef20fbc8dcd30ab00709c44d96eb4093eb788e64f14` | `transparent/11_crying.png` | the mark in the colour channels under alpha 0: seen, refused as `Transparent`, left, exit 3 |
| `cut-out-confetti-256.webp` | 18 422 | `88b1aeaf710201fc9374c653ca689b41bfc33600eb8fd1dd11450f2718e8fe39` | `transparent_thumbs_256/19_victory.webp`, byte for byte — the same sha256; no mark, so not in the archive below | a cut-out sticker, its corner transparent, confetti in it: no finding, `inspect` exits 0 |

The whole set — the 21 originals with the mark, the two alternatives
and the transparent copies — is in Watchword as
`wipemark-gemini-stickers-2026-10-04` (a stored ZIP). The confetti
thumbnail carries no mark and is not in it: the fixture *is* the file,
and its sha256 above is the source's, read on 2026-10-04.
