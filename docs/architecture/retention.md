# Retention — what is written, where, and what is kept

The Settings window's Retention page and `apps/wipemark-app/src/retention.rs`:
where a result goes, what happens to the file it came from, and whether
Wipemark keeps a copy of its own of what arrived with no file behind it.
This document is the survey the page was designed from, the rules it
settled on, and the one question it was written to answer — whether text
that arrived as Markdown or HTML is kept any differently.

Nothing is written yet. Cleaning is epic E1 and the batch queue is E4;
this page decides what they will do to a person's files when they
arrive, and its banner says so in every state it can be in.

## What the industry does

The product's nearest relatives are tools that take a file, remove
something from it, and hand back a file. Their answers to "what happens
to the original" fall into three shapes.

**A new file beside the old one, the original untouched.** `mat2`, the
Metadata Anonymisation Toolkit, writes `name.cleaned.ext` beside the
input and modifies the input only under an explicit `--inplace`. Windows'
built-in *Remove Properties and Personal Information* dialog defaults to
"Create a copy with all possible properties removed". Squoosh, TinyPNG
and every export-shaped tool produce a new file. The overview spec (§4.5,
§6.2) asks for exactly this: `*.cleaned.*` beside the input or in a
chosen folder, in-place only by explicit flag — and the CLI's `--out`
help already says "in-place needs an explicit flag, never a default".

**In place, with the original set aside.** ExifTool, the reference tool
for image metadata, overwrites the file and keeps `name.ext_original`
beside it unless told `-overwrite_original`; and it never overwrites an
existing `_original`, so cleaning a file twice cannot lose the first
original under the second's copy. `sed -i.bak` and `optipng -backup` are
the same shape with an opt-in.

**In place, no copy.** `jpegoptim`, `optipng` by default, ImageOptim
(whose FAQ says: it overwrites, keep your own backups), `sed -i`,
`perl -i`. Every one of these is a command-line tool or a batch
optimiser whose users are assumed to have version control or a backup
behind them.

**Non-destructive editors** — Lightroom, Apple Photos, Darktable — never
touch the original at all: edits are a recipe stored elsewhere, and a
file only exists when you export. Photos offers "Revert to Original"
because the original is always there.

For what a product keeps *of its own*, the precedents are about
history rather than files. Safari removes history items after a chosen
span ("one day … one year, manually"); Chrome keeps ninety days; Apple
Messages keeps for thirty days, a year or forever; screenshot tools
such as CleanShot keep captures in their own folder for a chosen number
of days. Password managers clear the clipboard after seconds. The
principle under all of it is the one privacy regulation calls *storage
limitation*: keep what was asked for, visibly, for a bounded time by
default.

## The rules

1. **The default is beside, and the file is never touched.** `mat2`'s
   default, the spec's, and the CLI's. `name.cleaned.ext`, with the
   infix before the *last* extension — `archive.tar.cleaned.gz` — and
   appended when there is no stem to put it after (`README.cleaned`,
   `.bashrc.cleaned`). A name that already carries the infix gets it
   again: `x.cleaned.md` → `x.cleaned.cleaned.md`, because collapsing
   it would make the result *the input*, and a destination called
   "beside" must never write over what it was handed.
   `a_result_beside_a_file_is_never_the_file_itself` is the gate. The
   CLI's `clean` writes beside by the same `with_infix`, which lives in
   `wipemark_intake::name` (with `RESULT_INFIX` and `ORIGINAL_INFIX`;
   `retention.rs` re-exports them) so the two applications cannot spell
   a result differently — and, as rule 3 says, the CLI reads none of the
   rows on this page.

2. **Replacing is never destructive.** The third destination replaces
   the file — a cleaned document with the same name is what a document
   with links pointing at it needs — but only after the original has
   been set aside as `name.original.ext`, and never over an original
   already there: the first original is the original. That is
   ExifTool's rule with a better name. An infix rather than
   `name.ext_original` or `name.ext.bak`, because both of those hide the
   extension, and a copy Finder cannot open is a copy nobody checks; and
   symmetrical with `cleaned`, so the two files beside each other read
   as a pair.

   "In place with no copy" — what a batch of five hundred files under
   version control wants — is deliberately **not a preference**. It is a
   per-run flag for the CLI and the batch queue to carry (E4, E5). A row
   that deletes originals is a landmine that goes off months after it
   was set.

3. **The CLI reads none of these rows.** `results.destination` is the
   window's preference and the batch queue's. A pre-commit hook that
   started replacing files because somebody clicked a radio button in a
   window is the failure "never a default" exists to prevent; the CLI's
   flags are the CLI's.

4. **A file is never copied into Wipemark's own folder.** The file is
   the original, and its result is written to disk. What can be kept is
   what has no file behind it — text pasted, an image dragged out of a
   browser, the text an agent hands the MCP server — because once the
   result has replaced it, `<data dir>/kept` is the only place it can
   be brought back from. The type says so: `Plan::File` has no field
   for keeping.

5. **Both switches are off, and the period is bounded, by default.** A
   product whose purpose is removing provenance must not quietly build
   an archive of marked originals in a folder the platform hides.
   Keeping is asked for, shown on the page — the banner names the
   folder and the span — and removed after a week unless a longer span
   or "until removed by hand" was chosen.
   `a_first_launch_writes_beside_the_file_and_keeps_nothing` is the gate.

6. **The format never decides whether a copy is kept.** See below.

7. **A kept copy is bytes as they arrived.** Markup included, never the
   text extracted from them — the same rule `wipemark-image` keeps for
   pixels. This half is E4's to implement; it is stated here so that it
   is implemented that way.

## Markdown and HTML — the question this page was written for

Should text that arrived as Markdown or HTML be kept, and kept
differently from plain text?

**Whether** to keep is decided by one thing, and the format is not it:
does the original survive somewhere else once the result exists.

* A file on disk is its own original, whatever it contains (rule 4).
  A `.md` or `.html` file is left where it is; its result goes beside
  it; nothing is copied anywhere.
* Neither layer offers a way back. `CleanReport` says so in as many
  words — byte-exact reversibility "is not offered and never will be":
  the report carries counts and positions, and NFKC and exotic-space
  normalisation are one-way — and a rewrite is lossy by construction.
  So the layer does not decide either. What decides is whether the
  original is still somewhere: a file is, a paste is not.
* Text that reaches the panel by paste or drag is already plain: a
  browser puts both an HTML flavour and a plain-text flavour on the
  pasteboard, and `pasteboard.rs` takes the plain one on purpose
  ("plain text beats the markup around it"). The markup was never
  taken, so there is nothing markup-shaped to keep. A drag that carries
  *only* HTML arrives as bytes named `clipping.html` and is planned like
  any bytes — into the results folder, kept only if asked, and as the
  bytes it was.

So `Source::of` reads nothing but how the thing arrived — a Markdown
paste, an HTML paste and a plain one are one `Source::Text`, a PNG and
a DOCX on disk are one `Source::File` — and
`a_markdown_paste_and_a_plain_one_are_planned_alike` fails on the day
somebody adds a `match` on the format.

**What** a kept copy is, on the other hand, is exactly where the format
matters, and in the opposite direction from the intuition "it's only
text". An HTML original is where provenance hides —
`<meta name="generator">`, a comment, a `data-` attribute, a class
name — and a Markdown one keeps its structure in characters a rewrite
can break. A copy that had been flattened to text would be a copy of
the wrong thing: it would have lost the evidence the *verifiable* shelf
of the report rests on, and the structure that recovering from a bad
rewrite needs. Hence rule 7. The same holds for the *result*: a `.md`
in is a `.md` out, container preserved, which is E4's parser's job
(spec §4.1) and the same rule as "pixels are never re-encoded".

## The rows

| row | key | values | default |
|---|---|---|---|
| Where results go | `results.destination` | `beside` · `folder` · `replace` | `beside` |
| Results folder | `results.folder` | absolute path, or `""` for the platform's Downloads folder | `""` |
| Keep what you paste | `keep.originals` | `true` · `false` | `false` |
| Keep results | `keep.results` | `true` · `false` | `false` |
| For how long | `keep.for` | `1d` · `7d` · `30d` · `90d` · `forever` | `7d` |

Every row keeps the bargain every other preference keeps: a value this
build cannot use is read as the default, warned about, and left in the
row. The folder follows `models.dir`'s rule exactly — a relative path
reads as the default, because a results folder that followed the
launcher's working directory would scatter results across the disk.
`an_unusable_retention_row_falls_back_without_being_rewritten` walks all
five.

The results folder defaults to the platform's **Downloads** folder and
not to anything under the data directory: a user's documents do not
belong in a directory the platform hides, and Downloads is where every
browser lands things and where people already look. It is used in two
cases — when the destination is *the results folder*, and for a result
that has no file to sit beside whatever the destination says, such as
an image dragged out of a browser. `Layout::downloads_dir` asks the
platform; `Homes::discover` falls back to the home directory and then
the temporary directory, the ladder the models folder climbs.

The period is five fixed spans and not a number of days: a dropdown can
tick one of five and cannot tick `14d`, and a preference the control
cannot show is one the user cannot see. The ids are the number of days
with a `d` on them, so whoever removes old copies reads the span off the
row without a second table, and nothing spells zero.

## The plan

`retention::plan` is a pure function over a `Source`, the rows and the
two folders:

```rust
pub fn plan(source: &Source, retention: &Retention, homes: &Homes) -> Plan
```

`Plan::File(Written)` for a file — beside, into the folder, or over it
with the set-aside name; `Plan::EachFileIn(folder)` for a dropped
folder, whose files the batch plans one by one; `Plan::Loose { result,
kept }` for text and bytes, where `kept` is present only when a switch
is on. Nothing in it touches the disk: whether `name.original.ext`
already exists is exactly the check it cannot make, which is why
`Written::Over` states the rule for the batch that will.

The panel is the plan's first reader. Under each thing dropped on it,
beside what it turned out to be, is a sentence saying what would happen
to it — "Its result would go beside it, as `report.cleaned.docx`; the
file itself would not be touched." — which is the one place today where
the page's choices can be seen against a real thing.
`every_plan_reads_as_a_sentence` is the gate.

## What is left open

* **The folder itself.** `<data dir>/kept` is named by `Layout::kept_dir`
  and by the banner, and nothing writes to it. E4 writes copies there as
  bytes as they arrived (rule 7), one directory per job, and removes
  them on the period — on launch and once a day while running, the way
  the log rotates.
* **Setting aside.** The filesystem half of rule 2 — rename the file to
  `name.original.ext`, refusing with *already exists* rather than
  overwriting — is a ten-line function with a test on a scratch
  directory, and it lands with the first caller that can rename a file.
* **The per-run "no copy" flag** for the CLI (`--in-place`, and
  `--no-original` or similar beside it) and for the batch queue.
* **History.** Spec §6.3's job history — hashes, actions, engines,
  outcomes, never text — is the third thing the product will keep, and
  its span is a natural sixth row here. It is not a copy, so it is not
  on the page until the history table exists (E4 / E6);
  `every_persisted_preference_has_a_row` will insist on the row the day
  the key is added.
* **"Remove everything now."** A button that empties the kept folder
  belongs beside the period once there is anything in it.
