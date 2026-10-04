# E11-3 — E11-1's test gaps

|                  |                                                                                                                       |
| ---------------- | --------------------------------------------------------------------------------------------------------------------- |
| Series           | `docs/plan/` — epic E11, images (phase 2); step 1 of the images series                                                 |
| Task             | Watchword FILE `wipemark-task-images-series-2026-10-04`, step 1                                                        |
| Depends on       | E11-1 (`crates/wipemark-image`), E11-2 (the CLI and MCP image surfaces), merged into `images/series`                   |
| Files touched    | `crates/wipemark-image/src/{lib,png,jpeg,webp,exif,json}.rs`, `crates/wipemark-image/tests/{gaps.rs,support/mod.rs}`, `apps/wipemark-cli/src/image.rs`, `apps/wipemark-app/src/mcp/image.rs`, the three catalogues, `docs/architecture/images.md`, this document, its report |
| Size             | ~half a day; pure code                                                                                                 |

## Goal

The host verification of E11-1 (D110) found four protections no test
guards. The code is believed correct for the first two; this step gives
each a test that goes red without it. The third is a decision the code
had not made. The fourth turns a guess the CLI already prints into a
fact the library reports.

## Ground rules

`CLAUDE.md` wins. One commit, `E11-3: E11-1's test gaps`, on
`images/series`. The library still returns values and never a sentence;
only applications localize; no epic number leaves the repository. In
this container tests are **compiled, not run**; every mutation is listed
in `docs/plan/reports/E11-3-mutate.py` for the host.

## True today

* `png.rs`, `classify`: `STRUCTURE.contains(&ty) || ty[0].is_ascii_uppercase()`
  — an unknown critical chunk is structure; no test feeds one.
* `jpeg.rs`, `settle`: a JPEG XT segment is C2PA when *any* segment of its
  box instance is labelled; every fixture's APP11 is one segment.
* `lib.rs`, `rebuild`: a removal before an `MPF` header is allowed and
  nothing else happens. The MP Index's first entry holds the primary
  picture's **Individual Image Size** (CIPA DC-007 §5.2.3.3), SOI to EOI,
  and every removal before the header shrinks the primary by the bytes
  removed. The field goes stale.
* `lib.rs`, `strip`: an EXIF block that names a generator is removed whole
  under the default scope, and its Orientation with it. E11-2's CLI says
  "may now show turned on its side" after *every* removed EXIF block,
  whether it carried an orientation or not.

## Decisions (I-numbers; proposed from D170 by the series report)

* **I1 — the MP Index is rewritten, not refused.** When blocks before the
  `MPF` header go, the primary's Individual Image Size becomes the old
  value minus the bytes removed — four bytes, in the index's own byte
  order. The secondary pictures' offsets are relative to the MP header and
  move with it, so nothing else in the index changes. The list of bytes
  this crate computes becomes: a WebP's RIFF size, two `VP8X` bits, and
  this one field. Refusing instead would refuse every phone photograph
  with an MPF whose EXIF named a generator, for a field most decoders
  ignore — and leaving it stale is a lie in the file. Three refusals stay
  `Unsupported::MultiPicture`: a removal after the header (as before), an
  index that cannot be read (its IFD or its entry outside the segment, a
  byte order that is neither `II` nor `MM`), and a size smaller than the
  bytes removed. An index with no MP Entry tag has no field to go stale,
  and a removal before it is allowed as before.
* **I2 — orientation is a reported fact.** `StripReport::orientation_removed:
  Option<u16>` is the EXIF Orientation (tag `0x0112`, IFD0) of the first
  **removed** EXIF block that carried one other than 1 — values 2 to 8;
  an out-of-range value is not an orientation and is not reported. The
  parsers read it where they classify an EXIF block (`exif.rs`, IFD0 only,
  bounded, never a panic), so a PNG raw `exif` profile is read after its
  hex is decoded. JSON: `"orientation_removed"`, a number or `null`,
  after `kept` and before the third shelf. Behaviour is unchanged.
* **I3 — the CLI says the fact, not the guess.** A new key,
  `cli-image-orientation-removed`, when `orientation_removed` is set, in
  either scope. `cli-image-exif-removed` and `cli-image-all-metadata` drop
  the "may now show turned on its side" clause they said for every EXIF
  block. MCP carries the JSON field and no sentence (nothing the server
  says comes from the catalogue).
* **I4 — a real Apple `CgBI` file stays refused.** Apple's chunk comes
  *before* `IHDR`, and the parser refuses that as `HeaderNotFirst`
  (unchanged; such a file is not a PNG a standard decoder reads either).
  The test feeds `CgBI` and an invented `ABCD` after `IHDR`, where the
  critical-chunk rule is the only thing that keeps them.

## Tests (each with the mutation that must turn it red)

| test | protection | mutation |
|---|---|---|
| `an_unknown_critical_chunk_is_structure` | `\|\| ty[0].is_ascii_uppercase()` | remove it |
| `a_c2pa_manifest_across_app11_segments_leaves_whole` | grouping by box instance | count only a segment that carries the label itself |
| `an_unlabelled_jumbf_of_another_instance_is_not_c2pa` | grouping is by *instance* | count every JUMBF as C2PA |
| `the_mp_index_follows_a_removal_before_it` | the rewrite | drop the patch |
| `the_mp_index_follows_a_removal_before_it` (little-endian half) | the index's byte order | always big-endian |
| `an_mp_index_that_cannot_be_read_refuses_a_removal` | the refusal | treat unreadable as absent |
| `a_removed_orientation_is_reported` | `orientation_removed` | always `None` |
| `orientation_one_is_not_a_rotation` | the `2..=8` filter | report any value |
| `the_json_form_of_a_strip_report_is_exact` | the key | not written |
| CLI `the_rotation_is_said_only_when_it_was_removed` | the sentence | condition `false` |

## Out of scope

Rewriting anything else in MPF; reading XMP `tiff:Orientation`; keeping
an orientation by editing inside an EXIF block (a block is kept or
removed whole — E11-1's rule).
