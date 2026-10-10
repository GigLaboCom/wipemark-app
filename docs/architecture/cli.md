# The command line — `wipemark-cli`

The scriptable half of the product: the same crates as the window, no
window, and an exit code a pre-commit hook, a CI step or an agent can act
on. Every command runs; `rewrite` (E5-2) runs the pipeline on the running
application's engine when there is one, and on the local model otherwise
— see "`rewrite`" below.

`apps/wipemark-cli/src/`: `main.rs` is the argument surface and the exit
codes; `input.rs` reads and decodes a path or stdin through
`wipemark-intake`; `run.rs` is `inspect` and `clean`; `image.rs` the same
two on a picture (E11-2, "Images" below); `report.rs` the
human report; `audit.rs` the walk and its three renderings; `models.rs` the catalogue
and the downloader; `rewrite.rs` the rewrite flow and the command's own
engine; `app.rs` the road to the running application; `journal.rs` the
run's row in the application's document journal (E4-6b, "The journal"
below). Every write to disk — `-o`, beside the input, and
`--in-place` — is `wipemark_intake::inplace` (moved there from the CLI's
own `inplace.rs` in tails-1, behaviour unchanged, so the windows can share
it in E7). Layer A itself is `docs/architecture/layer-a.md`.

## Exit codes are the interface

`0` clean · `1` findings · `2` usage or a refusal · `3` partial —
*inconclusive is not clean*. Pinned in `main.rs` (`Exit`,
`exit_codes_are_pinned`). Never `0` for work that did not happen.

| command | exit codes | stdout | stderr |
|---|---|---|---|
| `inspect <path\|->` | 0 · 1 · 2 (missing, folder) · 3 (unreadable, not text, 8-bit, invalid) | the report | a note when the name lies; the refusal |
| `inspect --json` | as above | `InspectReport::to_json()` | as above |
| `clean <path>` (beside, `-o file`) | 0 · 1 (by the **input**) · 2 · 3 (also: the result could not be written) | the report | note, refusal |
| `clean … -o -`, `clean -` | as above | the cleaned text | the report, note, refusal |
| `clean --json` | as above | `{"report","written"}` or `{"report","text"}` | note, refusal |
| `clean --in-place [--no-original]` | 0 · 1 · 2 (stdin, a link, `-o`, an original already set aside, a replacement that could not be written) · 3 (unreadable input) | the report | note, refusal |
| `clean --in-place --json` | as above | `{"report","written":{"path","original"\|null}\|null}` | note, refusal |
| `inspect <picture>` | 0 (no block is AI provenance, no visible mark seen) · 1 (a block is, or a visible mark was seen) · 2 (TIFF, HEIC, AVIF: not in this version yet) · 3 (a picture this version could not read, or pixels that should have been examined and were not; 3 beats 1) | the report | a note when the name lies; the refusal |
| `inspect <picture> --json` | as above | `PictureInspection::to_json()` | as above |
| `clean <picture>` (beside, `-o file`, `--in-place`) | 0 (the input carried neither provenance nor a mark) · 1 (it did, and the result carries neither) · 2 (as `inspect`; a flag for text; an MPF index a removal would leave wrong; an animation to write back; every refusal of `clean` on a text) · **3 (could not be read; the result would still carry AI provenance metadata — nothing written; a visible mark left in the result — written with what could be done; pixels not examined; a restored picture that failed its own check — nothing written)** | the report | note, refusal |
| `clean <picture> -o -`, `clean -` | as above, and 2 when stdout is a terminal or with `--json` | the image's bytes | the report, note, refusal |
| `clean <picture> --json` | as above | `{"report":<PictureReport>,"written"}` | note, refusal |
| `audit <dir>` | 0 · 1 · 2 (not there, not a folder, `--json` with `--sarif`) · **3 when any file could not be read, findings or not** | files with findings, the summary, the unreadable files, the third shelf | refusal |
| `audit --json` / `--sarif` | as above | one JSON value / one SARIF 2.1.0 log | refusal |
| `models list [--json]` | 0 · 3 (the folder exists and cannot be read) | the catalogue and the folder | the folder's error |
| `models pull <id>` | 0 (present, or downloaded and verified) · 2 (unknown id, no room, mismatch, cancelled, any failure) | where the weights are | progress, refusal |
| `models verify <id>` | 0 (every byte hashed and matching — a model you added against the sha256 recorded when it was added) · **1** (absent or not matching) · 2 (unknown id) · 3 (could not be read) | the verdict | refusal |
| `models pull <id>` of a model you added | 2 (nothing to download) | — | refusal |
| `models rm <id>` | 0 (removed, or nothing to remove) · 2 (unknown id, a model you added, could not remove) | what was removed, and what the application will show | refusal |
| `models add <path> [--name] [--role] [--ctx]` | 0 (read in full and recorded) · 2 (not a GGUF, not a model that writes text, a name, purpose or context refused, no database or not this build's schema, the file could not be read) | `<id>: …` — the id, the file, the name | the chat-format verdict, progress, refusal |
| `models forget <id>` | 0 (the row removed; the file never) · 2 (unknown id, a catalogue id, no database or not this build's schema) | what was forgotten, and what the application will show | refusal |
| `rewrite <path\|->` | 0 (every paragraph rewritten, no Layer A findings in the input) · 1 (every paragraph rewritten, findings in the input — cleaned, D31) · 2 (no engine that may answer, a tactic not run here, a template that breaks a rule, a job that failed, was cancelled or lost its connection; nothing written) · **3 when any paragraph kept its cleaned original, findings or not** — and when the result could not be written | the report (the text with `-o -` or stdin) | price and progress on a terminal, who rewrote it, refusal |
| `rewrite --json` | as above | `{"report","written"\|"text","served_by"}` | who rewrote it, refusal |

Standard output carries one product; the human report moves to stderr
only when stdout carries the text (`run.rs`'s table). `--json` answers are
one line of ASCII (D29).

## `clean --in-place`

Retention rule 2 from the command line (`docs/architecture/retention.md`).
A **per-run flag**, never a preference, and the CLI reads none of the
Retention page's rows — a hook must not start replacing files because
somebody clicked a radio button in a window.

The order is the protection, and it is `wipemark_intake::inplace::replace`'s:

1. Read and clean as `clean` always does. A refusal to read is a refusal:
   nothing is touched.
2. **Nothing changed, nothing touched.** When the cleaned text is the
   text, there is no set-aside, no write, no new inode, the mtime where it
   was; `--json` says `"written": null`. The exit code is still core's:
   a homoglyph kept without `--aggressive` is a finding with no change and
   exits **1** over a file that was not touched — the one case where
   findings and "nothing changed" meet (`in_place_touches_nothing_when_nothing_changed`).
3. Unless `--no-original`: `name.original.ext` beside the file, spelled
   by `wipemark_intake::name::with_infix` like everything else. If
   anything already has that name — a dangling link included — the run
   **refuses with 2**, names it, and touches neither file: the first
   original is the original (ExifTool's rule). Otherwise the file is
   given that **second name by a hard link** — one directory, so one
   volume; the operating system refuses it when the name is taken, so no
   check comes before it (D284) — and the set-aside copy is the original
   byte for byte and inode for inode. A file system without hard links
   gets the old road: a check, then a rename.
4. The cleaned bytes, in the file's own encoding with its byte order mark
   (D11), go to a temporary file in the same folder, are `fsync`ed, take
   the original's permissions and are renamed over the path.
5. If step 4 fails after step 3, the second name is removed and the run
   exits 2 saying the file was not changed — the file never moved. On the
   rename road the original is renamed back; if that rename fails too, the
   refusal says where the original is now.

Step 3 used to be a check and a rename — two calls, so a file another
process created under that name between them would have been replaced.
The hard link is one call that cannot replace anything (D284, which
supersedes D81's accepted race). Only on a file system without hard links
is the check-then-rename still the road, and `wipemark_intake::inplace`
says so where the next caller will read it.

Two refusals the document did not list: **standard input** (`-`) has no
file to replace, and **a symbolic link** is refused — setting it aside,
by a hard link or a rename, would set aside the *link*, and the result
would replace the link and leave the file it points at unchanged, a run
that reports success over a document it did not touch. The windows' *In
place of the file* refuses it the same way (D287). A hard link
is not refused and cannot be: the replacement is a new inode, so the
other name keeps the old bytes, which is the same rule `-o` has always
kept ("never into an existing file").

## Images — `inspect`, `clean` and `audit` on a picture

E11-2; the plan is `docs/plan/E11-2-images-on-the-surfaces.md`, decisions
I1–I13 (proposed D130–D142). The library is `wipemark-image`
(`docs/architecture/images.md`).

**What is a picture.** A file — or standard input — whose head
`wipemark-intake` places as PNG, JPEG, WebP, TIFF, HEIC or AVIF **by its
bytes** (`input::read_any`, `picture_of`). A `holiday.txt` that is a PNG is
a picture, and the note on stderr says the name lied; a `photo.png` whose
bytes nothing places is not, and is refused as not text, as before. GIF,
BMP and SVG are not handed over (SVG is text and goes to Layer A).
`rewrite` reads through `input::read` and still refuses a picture.

**`inspect`** lists every metadata block — its chunk or segment and the key
inside it (`tEXt parameters`, `APP1 Exif`), its kind, its byte offset and
length — under "Would be removed" (AI provenance) and "Would be kept"
(everything else), and under each AI block the signals that made it so: a
C2PA manifest or a reference to one, an IPTC digital source type, a
generator's key or signature, with the field and the signature that
matched — **never the value**. A key read out of the file is spelled
`U+XXXX` past printable ASCII, there and in the JSON
(`wipemark_image::spell`). Then a line when there is colour information
(kept whatever is asked); then **Visible marks** (E12-5): every mark a
profile of the shipped catalogue proposed, with its profile id, vendor and
product — identifiers, never inside a sentence of their own (Q-V9) — its
rectangle, whether its row placed it or the search found it, and its
verdict with the numbers: "proved" with correlation, strength and edge
ratio, or "seen, not proved" with the reason and the number that failed;
or that none was found, or why the pixels were not examined (an animation;
a catalogue that did not load; pixels that do not decode). Then the line
that says what the pixels were examined for and that marks no eye sees are
not looked for, and the picture's third shelf — `invisible-pixel-marks`
first. No Unicode line: nothing in a picture was read as characters. Exit
**1** when a block is AI provenance or a visible mark was seen, **3** when
the pixels should have been examined and were not (3 beats 1), **0**
otherwise — camera EXIF is not a finding (D131).

**`clean`** strips with `Scope::AiProvenance`, or `Scope::AllMetadata`
under **`--all-metadata`** — every block but colour, camera data and EXIF
orientation included; the report says when a removed EXIF block carried the picture's rotation (`orientation_removed`, either scope). Colour
profiles are kept by both. The output goes where a text's does — beside
the input as `name.cleaned.ext`, `-o`, standard output, or `--in-place
[--no-original]` through `wipemark_intake::inplace` (nothing removed is
nothing touched: no set-aside, no write). The exit is the **input's**, as a
text's is: 1 when it carried AI provenance, 0 when not. `still_has_c2pa`
and `still_has_ai_metadata`, read off a second inspection of the output,
are one thing that can make it otherwise: either one true is exit **3**,
and the result is **not written** — a `.cleaned` file that still carries
provenance metadata is worse than none (D132). **A visible mark is
removed with no flag** (Q-V1, the owner's answer: "marks found are
removed") when it is proved — the picture is written back through
`wipemark_picture::clean` (PNG at its own colour type where it can be,
WebP lossless, JPEG re-encoded at quality 95) and the report says how;
the input carried a mark, so the exit is **1**. A mark seen and **not
proved**, or restored around opaque pixels, or in a picture this version
does not write back (a CMYK JPEG), is **left**: the result is written with
what could be done, the report says what is left, and the exit is **3**
— inconclusive is not clean. Pixels that should have been examined and
were not are 3 the same way.

**Refusals.**

| what | exit | said |
|---|---|---|
| TIFF, HEIC, AVIF | 2 | "`<path>`: TIFF images are not in this version yet." |
| bytes intake placed as a picture that the library does not open | 2 | "not an image this version opens" |
| a JPEG whose MPF index a removal would leave wrong | 2 | the MPF sentence; nothing written |
| `--aggressive` or `--nfkc` on a picture; `--all-metadata` on a text | 2 | which flag, and for what |
| a picture for a terminal on stdout; `--json` with the picture on stdout | 2 | write it with `-o` |
| a picture this version could not read (`Malformed`) | **3** | the defect (`image-defect-*`) and its byte offset; "not read is not clean" |
| a result that would still carry AI provenance | **3** | nothing written |
| an animated or unknown-critical picture whose pixels changed | 2 | the reframe sentence; nothing written |
| a restored picture that could not be written back, or failed its own check | **3** | nothing written |
| a visible mark left in the result | **3** | the result is written; "is still in the result" |

**`--json`.** `inspect`: `PictureInspection::to_json()` — E11's
`ImageReport` keys where they were,
`{"container","ai_metadata","c2pa","findings":[{"kind","chunk","key","offset","length","ai","c2pa","evidence":[…]}],`
then `"visible"` — `{"examined":true,"restorable","found":[{"profile","vendor","product","pass","rect","pixels","placed","row","kernel","ncc","verdict","refusal","scores","also_tried"}],"restored":[]}`
or `{"examined":false,"why":"animated"|"catalogue"|"decode"}` — and the
picture's `"not_established"`, `invisible-pixel-marks` first; one line of
ASCII. `clean`: `{"report":<PictureReport>,"written":…}` with E11's
`StripReport` keys
(`"container","still_has_ai_metadata","still_has_c2pa","removed","kept","orientation_removed"`),
then `"visible"` (with `restored`:
`{"profile","rect","changed","holes","clamped","outline","steps","step","chroma","outline_left","texture","texture_around","texture_left","noise","lossy","fitted","resampled","searched","exact","consistency_px","consistency_excluded"}`
— `wipemark_pixels::Restored`, field for field: the outline's share of
the contour, the faint band's step per channel (`steps`, R G B), in luma
(`step`) and in colour difference (`chroma`), the roughness the
restoration left and the same around the mark, why it is or is not
`exact`, and how far it is from the data (D305): the restored samples
blended back against the input, the 95th percentile in 8-bit levels
(`consistency_px`, about 0 for an exact inverse), and the samples left
out of it, clamped ones and holes (`consistency_excluded`). The last two
are a measure and decide no exit; `consistency_dct` joins them only on a
path that chooses a value inside a JPEG's intervals, and is absent
otherwise. After them, two keys **absent** — not `false`, not `null` —
wherever they would say nothing, so every report that says neither is
byte for byte what it was: `"smoothed":true` (D307, E12-R8) when, on a
lossy source, the restoration's roughness is under 0.8 of the picture's
around it — a patch flatter than its surroundings, a mark left like a
texture is (exit 3, and the human report says "The restored patch is
smoother than the picture around it — … so the mark counts as still in
the result", `cli-image-visible-smoothed`); and
`"interval":{"method":"dct"|"pixel"|"wiener","space":"ycbcr"|"rgb","sigma_base":[…],"text","iterations"}`
when the restored value was chosen inside a lossy codec's interval
(E12-R8) — which every restoration of a JPEG whose planes were read now
is, by DCT-POCS (D472, below)),
`"encoding"` (`{"kind":"unchanged"|"png"|"webp-lossless"|"jpeg",…}`),
**A 4:2:0 or 4:2:2 JPEG is proved and restored in its planes** (D471, the
owner, 2026-10-10 — the series' proposed D306, taken with the per-plane
interval), and two keys appear, each **absent** — not `null` — everywhere
else: a finding's `scores` gains `"planar":{"y","chroma"}` (the
out-of-range share's two terms; `out_of_range` is then the share of
pixels with either), and a restoration gains
`"planar":{"sampling":"4:2:0"|"4:2:2","max_alpha_dev_in_block","holes_chroma"}`,
or `"planar":"unavailable"` for a lossy three-component JPEG whose planes
could not be read and which was restored in RGB. A 4:4:4 JPEG, a PNG and
a WebP carry neither. See `docs/architecture/visible-marks.md`, "The
planar inverse (E12-R6)".
**Every restoration of a lossy JPEG whose planes were read is refined by
DCT-POCS** (D472, the owner, 2026-10-10): the restored value chosen inside
the file's own quantisation intervals. It gains `"interval"` (`"method":"dct"`)
and `"consistency_dct"`, and its `consistency_px` is no longer about 0 —
the value chosen is another point of the data, by design. A lossless
picture is never refined (S6) and its report is byte for byte what it
was; a lossy WebP, which has no intervals to read, keeps the inverse's
value and carries no `interval`. No flag and no environment variable
chooses the method (the `planar-preview` build and its
`WIPEMARK_INTERVAL` are gone). See `docs/architecture/visible-marks.md`,
"The value inside the interval (E12-R8)".
`"marks_left"`, `"not_established"`. The writers are the libraries'
(`wipemark-image`'s `json.rs`, `wipemark-pixels`'s report,
`wipemark-picture`'s splice).

## `audit <dir> [--json | --sarif]`

For a pre-commit hook or a CI step: every text file under a folder, read
exactly as `inspect` reads one (`input::read_any`) — and every PNG, JPEG
and WebP, whose metadata is inspected as `inspect` inspects one picture.

**What is walked.** `std::fs::read_dir`, recursively, sorted by name, no
depth limit. A hidden entry — a name starting with `.`, `.git` above all —
is skipped, counted and listed, never opened. A link to a **folder** is
not followed: a link back up the tree is a loop and a link out of it is
somebody else's tree, and not following is what makes the walk finite
without a visited set. A link to a **file** is read like the file. Only
regular files are opened — a FIFO would block the walk forever. **No
size limit**: the CLI has none for one file (`inspect` reads to the end),
so `audit` does not invent one; a file is refused on its first four
kilobytes when they are not text, so a large binary costs four kilobytes.
`.gitignore` is not read; a flag for it is a later addition if anyone
asks.

**Three outcomes per file.**

| status | what | `reason` (a format) |
|---|---|---|
| `scanned` | text, decoded, `wipemark_core::inspect` ran; or a picture, `wipemark_image::inspect` ran | `null` |
| `skipped` | never an error | `hidden`, `link-to-folder`, `not-a-file`, `not-text`, `empty`, `image-not-yet` (TIFF, HEIC, AVIF) |
| `unreadable` | a hole in the scan | `unreadable` (I/O, permission), `unnamed-encoding` (8-bit), `invalid` (a bad sequence in a file intake called text), `malformed-image` (a picture this version could not read), `missing` (gone mid-walk); a folder that could not be listed is listed as `dir/` |

**Exit.** `3` if anything was unreadable — **even when another file had
findings**: a hook must not read a scan with a hole in it as a complete
one, and inconclusive beats a finding — a picture that could not be read,
or whose pixels should have been examined and were not, is such a hole.
Else `1` if any report is suspicious or any picture carries AI provenance
or a visible mark (`inspect`'s rule exactly — soft hyphens alone do not
make a tree marked). Else `0`. A `<dir>` that is not there or is not a folder is
`2`.

**Human output (stdout).** One line per file whose report is suspicious —
`sub/c.md: 2 findings (zero-width character ×2)`, every class counted
across what `clean` would act on and what it would keep — then the
summary (`scanned · with findings · skipped · could not be read`), then
the unreadable files, each with the sentence `inspect` would have
refused with, then the Unicode version and the third shelf, once. Every
sentence from the catalogue. Never a file's text.

**`--json`.**

```json
{"version":1,"unicode":"18.0.0","root":"tree",
 "files":[{"path":"sub/c.md","status":"scanned","reason":null,"report":{…}},
          {"path":".git","status":"skipped","reason":"hidden","report":null}],
 "summary":{"scanned":3,"with_findings":1,"skipped":1,"unreadable":0}}
```

`report` is `InspectReport::to_json()` spliced in as it is — the very
bytes `inspect --json` prints, third shelf included; for a picture it is
`ImageReport::to_json()`, which has `container` where a text's has
`unicode_version`, and the human report's line for it is
`art/a.png: PNG, 2 blocks of AI provenance (generator parameters ×2)`
(`audit_json_carries_every_report_with_its_third_shelf` compares them).
Paths are relative to `<dir>` and `/`-separated; `root` is `<dir>` as
typed.

**`--sarif`.** SARIF 2.1.0, one run:

* `tool.driver.name` `wipemark`, `version` the crate's; no
  `informationUri`. `rules` are the `UnicodeClass` ids that occur
  (`zero-width`, `bidi-control`, …) with `shortDescription` the class's
  catalogue line.
* One `result` per **occurrence** of a finding: what `clean` would act on,
  plus what it would keep at `probable` or above (a homoglyph left in
  place without `--aggressive`) — the same rows that make `inspect` exit
  1. `level`: `confirmed` → `error`, `probable` → `warning`, anything
  lower → `note`. `message.text` is `U+200B ZERO WIDTH SPACE: zero-width
  character, confirmed`.
* `physicalLocation.artifactLocation` is the relative path,
  percent-encoded as a URI reference, with `uriBaseId: "SRCROOT"` for the
  consumer to resolve; `region` has `startLine`, `startColumn`,
  `endColumn` (exclusive), `charOffset`, `charLength` (1).
* `run.columnKind` is `unicodeCodePoints`, and lines, columns and
  `charOffset` count **code points** (`audit::locate`): a zero-width space
  after two CJK characters is column 3, not 7, and a UTF-16 file reads
  the same as its UTF-8 twin. Lines end at LF, CR, or CR LF counted once.
  A byte order mark at the very start is not counted — no editor shows it
  as a column.
* Unreadable files are `invocations[0].toolExecutionNotifications` at
  level `error`, each with its location, and
  `invocations[0].executionSuccessful` is `false` when there is one.
* **A picture** gives one `result` per signal of every block that is AI
  provenance, with `ruleId` `image-<signal>` (`image-c2pa-manifest`,
  `image-digital-source-type`, `image-generator-key`, …) — one rule per
  signal whatever generator it names, listed after the text's rules —
  `level: error` (the block is there and its signature matched), and a
  `region` of `byteOffset` and `byteLength`: a metadata block has no line.
  A **visible mark** (E12-5) is one `result` with `ruleId`
  `visible-<profile>` — one rule per profile seen, listed after the
  signals' — `level: error` when proved and `warning` when seen and not
  proved, no `region` (a byte range means nothing for a mark in the
  pixels), and `properties: {"rect":{x,y,width,height},"verdict","ncc"}`.
  A picture that could not be read is a notification like an unreadable
  text, and so is one whose pixels should have been examined and were not.
* `run.properties` carries the Unicode version and the third shelf's ids
  — with `invisible-pixel-marks` first when any picture was scanned — so
  the SARIF log is not the one report without them.
* **English, whatever the user's language.** SARIF is a format read by
  dashboards; the rule and message text are the `en-US` catalogue
  rendered with `PlainText`. A rule description that changed with the
  locale of the CI runner would be a rule nobody could match.

## `models list | pull <id> | verify <id> | rm <id> | add <path> | forget <id>`

The catalogue (`manifests/models.v1.json`, compiled in) and the
downloader of `wipemark-models`, without the window.

**Which folder, which model.** The application's two rows, through a
**read-only** open of `wipemark.db` that never creates one
(`the_cli_never_creates_a_database`): `models.dir` — an absolute path, or
`""` for `<data dir>/models`; a relative one reads as the default and is
left in the row — and `models.rewrite`, which counts only while the
catalogue has the id and it serves `rewrite`. The rules are
`apps/wipemark-app/src/config.rs`'s, restated because the CLI may not
depend on the application; the keys are formats. The CLI writes no row
but the models the person adds — below, and D404.

**Models you added (E8-1, [user-models.md](user-models.md)).** The rows
`models.user.<id>` are read with the others. `models.rewrite` also counts
when it names one of them that serves `rewrite`; a row naming one since
forgotten reads as nothing chosen.

**`list [--json]`.** Every entry: id, name, roles, size, the state on
this machine (`present` — every file matches, **wherever under the folder
it was found** (D302); `absent`; `partial` with how far, which `pull`
resumes; `mismatch` — a download of this product at its place that is no
longer the catalogue's), whether it is the model chosen for rewriting,
and `host::fit` for this machine, probed once (`None` is "unknown", never
"no"). A model whose file this product did not download — found
anywhere else, or at its own `<id>/<file>` place with no download's mark
(D302, amended) — adds "found at *path*" (`found_at` in `--json`, the
path below the folder, `/`-separated). Another tool's file at the
entry's own place with its name and not its contents leaves the entry
`absent` and adds "another tool's file of its name is at *path*, left as
it is" (`foreign_at`), and so does a `<file>.part` there that no
download of ours is known to have opened (D351) — it is not `partial`, and
the line says "a partial file Wipemark has no record of is at *path*"
(D375). A download of ours whose device and inode numbers a remount moved
is still ours (D375; `model-downloads.md`). Then the weight files in the folder the catalogue
did not put there, from `wipemark_models::scan::weights_under` — listed,
never verified, never loaded, and the heading says so; the catalogue's
files, wherever found, and a `foreign_at` file are not among them. A
state may re-hash a file whose size or mtime moved since its record
(`<data dir>/records`, D303 — never beside the weights); that is a CLI
with no window to freeze.

**`pull <id>`.** Present and verified — wherever it was found — says
so and where, exits 0, no network, and writes nothing. A file at the
entry's own place that this product did not download and that is not the
catalogue's is never downloaded over: `StoreError::Occupied`, exit 2,
the file untouched — and so is a `<file>.part` no download of ours opened
(D351), another tool's download in progress, which is never resumed into.
Occupied has its own sentence (`cli-models-pull-occupied`, A4): nothing
was fetched and the file is left as it is, so it says to move the file
away and run pull again rather than promise a resume a second pull would
be refused the same way
(`an_occupied_place_is_not_promised_a_resume`); a `.part` in the way is
said as a partial file Wipemark has no record of
(`cli-models-pull-occupied-part`, D375). A file whose download's
mark no longer names it (D350) is another tool's in the same way. A
records folder that will not take the `.part`'s mark refuses the pull
before anything is asked of the server, with no `.part` left (D375).
Otherwise `Downloads::fetch` on a thread of its own (which resumes a
`.part`), with the typed `StoreError` sent back so each failure has its
own sentence. Progress goes to **stderr**: one line redrawn at most twice
a second when stderr is a terminal (`IsTerminal`), a line at each quarter
(0/25/50/75/100 %) otherwise — a CI log is not a terminal. **Ctrl-C**
sets `Cancel`: the downloader flushes and keeps the `.part`, and the run
exits 2 with "cancelled; run pull again to resume"; a second Ctrl-C
leaves at once. That handler is the one new dependency, `ctrlc` — `std`
has no portable way to install a signal handler, and a hand-written one
would be the CLI's only `unsafe`. A checksum mismatch exits 2 with both
hashes and the `.part` already removed by the downloader (a resume point
that can only produce the same wrong file is not kept); no room exits 2
before a byte is fetched; anything else exits 2 with the downloader's
words and the `.part` kept. The log line records where the download
resumed from, how far it got, and how long it took.

**`verify <id>`.** `Downloads::rehash`: every file hashed in full,
whatever the record says (`<data dir>/records`, D303) — the record is a
cache of the last verify, enough
to notice a file that was replaced and not a byte changed in place, and
a command asked to verify is not asked to consult a cache
(`a_full_rehash_does_not_trust_the_stamp`). A match refreshes the record.
A file found elsewhere under the folder is verified where it is.
Exit **1** when the file does not match *or is not there* — the same
finding, the file on disk is not the file the catalogue promised — and 3
when it could not be read.

**`rm <id>`.** `Downloads::remove`; exit 0 whether or not there was
anything (and says which). It removes only what a download of this
product wrote — a file at its place carrying the download's mark under
`<data dir>/records` (D302, amended). A file it did not download — found
elsewhere, at its place without the mark (a download by a build from
before the mark included), or another tool's file of its name — is left,
and the answer is "*id* is at *path*, where Wipemark did not download it;
nothing was removed" — or, for a `.part`, "*id* has a partial file at
*path* that Wipemark has no record of; nothing was removed" (D375) (`models_rm_leaves_another_tools_file_at_the_entrys_place`,
and `scripts/verify/owner-fixes/rm-in-a-mirror.sh`, which says KEPT). Removing the chosen model says that the
application will show no model chosen until another is picked — and
leaves the row alone.

An id not in the catalogue is refused at 2 by name, with the ids it has,
for every subcommand that takes one — the catalogue's and, once somebody
has added a model, those too.

**`add <path> [--name <s>] [--role rewrite] [--ctx <n>]`** (U5). The
file's GGUF header is read — never a tensor — and what is not a model that
writes text is refused at 2 with the window's own one-line reason (a
projector, an adapter, no weights, an encoder or speech model, no chat
template, not a GGUF). The name is `--name`, or the file's own
`general.name`, or its name without `.gguf`; the purpose is `rewrite`, the
only one this version adds for; the context is `--ctx`, or the smaller of
the trained window and 8192, and must lie between 2048 (or the window) and
the window. Then the **database**: the rows are written through
`wipemark_store::RowsWriter`, which never creates and never migrates one —
no `wipemark.db` yet, or one at another schema than this build's, is exit 2
with a sentence that names the application (D404). The file is hashed in
full on a thread of its own, the progress on stderr as `pull`'s is, and the
row written; stdout says `<id>: <path> was added as <name>…` and that
Wipemark cannot vouch for what the model is, stderr the chat-format
verdict (supported, or why loading it would be refused). A file already
added is added **again** under its id (D405), and says so. Nothing is
written beside the file, and the file is never moved or copied.

**`forget <id>`.** The row of a model you added goes; the file never does
(Wipemark did not download it). A catalogue id is refused at 2 ("rm removes
a download of it"); forgetting the chosen model says what the application
will show and leaves `models.rewrite` alone, as `rm` does. `pull` and `rm`
of a model you added are refused at 2: there is nothing to download and
nothing Wipemark may delete.

**`list` and `verify` of a model you added.** `list` adds each one after
the catalogue's — `"source": "user"` in `--json` (the catalogue's entries
carry `"source": "catalogue"`), with `path`, `ctx`, `estimate_mb`,
`sha256` and `state` (`present`, `changed`, `missing`, `unreadable` with a
`reason`), the fit judged on the estimate; in prose, "added by you: *path*".
A model's state is looked at as the application looks at it — its record
trusted while its identity holds (D401) — and a moved identity whose bytes
a full read confirmed is **written back** by the command itself (D435): one
field of that model's row, through `RowsWriter::update` — never an insert,
never over a row re-added with other bytes meanwhile, never into a database
it did not find at this schema (D404). Before, the command left it to the
application, and every `list` and every `rewrite` read a touched 12 GB file
in full until the application scanned. Its file is not listed again among
the folder's strangers. `verify` reads its file in full against the sha256
recorded when it was added, and writes a moved identity back the same way:
0, 1 when it changed or is gone, 3 when it could not be read.

**The same file, another road** (D436). `add` of a file already added —
by its path, or reached through a link, `..` or a second hard link — adds
it again under its id and keeps the row's name and context unless `--name`
or `--ctx` is given. A pipe, a device or a folder is "not a regular file"
and never opened; a file that changed between its header and its hash, or
while it was hashed, is refused ("changed while it was read") and nothing
is written (D439). `rm` of a catalogue entry refuses, at 2, a file a model
you added names, saying which model and `models forget` (D439). The list
of the folder's strangers says any of them can be added with `models add`.

## Logs

Same file shape as the application's, under the stem `wipemark-cli`
(`docs/architecture/logging.md`). The stderr mirror is off unless
`WIPEMARK_LOG` is set, because stderr is a hook's contract. A log line
carries the command, counts, sizes, the exit code and an `io::ErrorKind`
— never a document's text, and never a path either: the input and the
audit root are `Elided` (`nothing_reaches_the_log_but_the_shape`).

## `rewrite`

E5-2, with the MCP tool it reaches (E4-6a; the plan document
`docs/plan/E4-6a-headless-rewrite.md`, decisions H1–H20).

```
wipemark-cli rewrite <path|-> [-o <out>|-o -|--in-place [--no-original]]
    [--tactic paraphrase|humanize|back_translate] [--intensity light|moderate|strong]
    [--candidates N] [--rounds N] [--format plain|markdown|html]
    [--aggressive] [--nfkc] [--prompts <file.json>] [--seed N] [--json]
    [--no-record]
```

**Two roads, never both.** When the application runs, its MCP server
leaves a beacon, `<data dir>/mcp.json` (`wipemark_models::Beacon`: pid,
port, a loopback address), and the command sends the document to its
`rewrite` tool — the one loaded model serves every surface (D52). The
beacon is trusted only when its address is loopback, its process runs and
the server there answers `initialize` as `wipemark`; otherwise the
command loads its own engine. Once the call is sent its answer stands:
a connection lost after that exits 2, and nothing is run again here.
Ctrl-C hangs up, and the application cancels the job.

**Its own engine is the local model only.** With no application, the
rows decide (read-only): `engine.serves` = `endpoint`, or an endpoint
that would answer first (`endpoint-first` with a provider set) or instead
(`machine-first` with the model missing), refuses and names the
application — the endpoint's road (`duty`, profiles, the default-deny of
`engine::refusal`, the key) lives in the application crate. Otherwise the
model `models.rewrite` names, verified whole, is loaded by `LocalEngine`
— in a build with `local-llama`; without it the refusal is "this build
has no local engine". Never `FakeEngine`. A model you added (E8-1) is
loaded the same way at its own context while its file is the one that was
added; a file that changed or is gone is refused at 2 by name, naming
`models verify` (`rewrite_refuses_an_added_model_whose_file_changed`). A
model whose chat format this build does not write is refused by name when
it is loaded (D407). The rows decide before any model file is looked at: a
refusal the endpoint's row makes reads nothing, where it used to read a
touched added model in full on the way to refusing
(`rewrite_reads_no_model_when_it_will_refuse`, D435); a look that does read
writes a moved identity back, as `list` does.

**Arguments.** `--tactic` takes every tactic's id and refuses
`structural` (a window's, behind a confirmation) and `code` (not built)
by name. `--candidates`/`--rounds` have **no default** — absent is "whoever
rewrites decides" (D61: 1 × up to 2 on a CPU, 2 × up to 2 on a GPU or an
endpoint) — and are 1 to 8. `--format` overrides what intake says
(Markdown and HTML by it, plain otherwise). `--seed` names the base seed;
without it every run gets a fresh one (D83), and the report says which.
`--prompts` is a JSON object of template rows (`prompts.<lang>.<tactic>.<step>.<role>`
to a template's text or D74's object), laid over the rows the application
saved; a template that breaks a rule exits 2 naming the row and the rule,
before anything is read or sent — and goes to the application with the
call when it is the application that rewrites.

**Output** is `clean`'s shape with its own name: beside the input as
**`name.rewritten.ext`** (В8, E4-6b — a format change: until E4-6b a
rewrite was written to the clean's `name.cleaned.ext`, so a clean and a
rewrite of one file overwrote each other), `-o`, stdout, or `--in-place`
through `wipemark_intake::inplace`. A result already at
`name.rewritten.ext` is **refused**, as the windows refuse one (D261,
D362): exit 2 before the input is read or anything is sent, with a sentence
naming `-o` (another file, or that one by name — `-o` is the person's word
and replaces) and `--in-place`; the existing file is left byte for byte.
The write itself is `inplace::write_new`, which refuses a file that
appeared while the model worked (exit 2, the same sentence). `clean` keeps
replacing its own `name.cleaned.ext`, which the parity table pins. The human
report says what Layer A found in the input, how many paragraphs were
rewritten and how many kept their cleaned original, that Layer B is
best-effort, the seed, where the result went, who rewrote it and the third
shelf. Price and progress go to stderr on a terminal only.

## The journal (E4-6b)

Every document has a status, whoever asked: the application's main window
lists one row per document from its `journal` table (`wipemark-store`,
schema 3), and the command line's runs are among them.

| run | recorded | by whom |
|---|---|---|
| `clean` (text: `clean`, picture: `clean_image`) | unless `--no-record` | this command |
| `inspect` | only with `--record` (a look changes nothing, В7) | this command |
| `rewrite`, served by this command | unless `--no-record` | this command |
| `rewrite`, served by the running application | unless `--no-record` | the application — the call carries `"record"` and, in the params' `_meta`, `wipemark/origin: cli` with the file's name, path and size; when the answer names its row (`result._meta["wipemark/journal"]`) and this command wrote a file, it tells that row where |
| `audit`, `models` | never | — |

A row holds metadata only (D312): origin `cli`, the action, done or
failed, the file's name, absolute path, kind, format, encoding and size,
the verdict (`nothing-found`, `cleaned`, `partly`, `not-cleaned`,
`failed`; `findings` for a look; `rewritten` for a rewrite) with its
counts, and where the result went (`file`, `caller` for standard output,
`nowhere`) — never the text. A run refused before its input was read
records nothing, and neither does a `rewrite` whose `--prompts` template
is refused — on this command's own road too, where the `too-long` rule is
asked after the read (D360): a template refused is not a document's
status, on any road. A path that is **not a regular file** — a FIFO,
`<(…)`, a device — or any name under `/dev` or `/proc` (`/dev/stdin` names
this process's descriptor, a file here and a terminal in the application)
is recorded as no file, with no name and no path (D356), and the call through the application names none in `_meta`:
the application would otherwise open it to read the row back, and an open
of a FIFO nobody writes to never returns.

**The one write, never a create or a migration (В5, D314).** Every other
open of `wipemark.db` here is read-only; the journal goes through
`wipemark_store::JournalWriter`, which opens an existing database
read-write and offers the journal and nothing else. No database — no
application has run on this machine — is no row and **nothing said**
(D315: a line on every hook run would be noise). A database that cannot
take the row — older than the journal (the application migrates it when it
next starts), written by a newer build, or not writable — is no row and
**one** line on stderr; the exit code and standard output are unchanged.

## Not here

* Reading `.gitignore` in `audit`.
* Writing any Retention row. The journal row above is the one write.
