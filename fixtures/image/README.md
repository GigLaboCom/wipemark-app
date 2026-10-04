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
stamped, at a twentieth of the bytes; the WebP is as it was.

| file | bytes | sha256 | from | what is asserted |
|---|---:|---|---|---|
| `crying-1025.png` | 558 672 | `ef0da91aa48a491e5a9017cf774e71506867f125ebfd9d832f8d22a5103a5bab` | `11_crying.png` (a re-saved copy: no C2PA, a flattened background) | the mark is proved at its row and restored; also as JPEG 90/95 and shrunk with its picture; over MCP and the CLI |
| `torch-1025.png` | 934 572 | `a81716a84e0117d5c895e09c0addef0740cdbce497a4972308d3756c0adc20e3` | `05_torch_and_cross.png` | the same; held out of the map's fit: no ghost, no outline, no square |
| `anchor-green-1025.png` | 1 053 895 | `48ffc44b91899766a738a07ec26e2ff053d69b05a4ccc5ea7e9093ca15b7b1f8` | `alt-anch/anchor-alternative.png` (C2PA intact) | the mark over a saturated green, the original at 0 in red and blue: proved and restored, the clamped channels counted (D240) |
| `victory-1025.png` | 1 068 039 | `f79a575baa38d91aa53026d9420ee68026e3418786b0ad348d8f87135ffeb2a9` | `19_victory.png` | held out of the map's fit: no ghost, no outline, no square |
| `crying-transparent-1025.png` | 623 170 | `6cbc54505c68415faf21eef20fbc8dcd30ab00709c44d96eb4093eb788e64f14` | `transparent/11_crying.png` | the mark in the colour channels under alpha 0: seen, refused as `Transparent`, left, exit 3 |
| `cut-out-confetti-256.webp` | 18 422 | `88b1aeaf710201fc9374c653ca689b41bfc33600eb8fd1dd11450f2718e8fe39` | `transparent_thumbs_256/19_victory.webp` | a cut-out sticker, its corner transparent, confetti in it: no finding, `inspect` exits 0 |

The whole set — the 21 originals with the mark, the two alternatives
and the transparent copies — is in Watchword as
`wipemark-gemini-stickers-2026-10-04` (a stored ZIP).
