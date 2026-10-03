# The command line — `wipemark-cli`

The scriptable half of the product: the same crates as the window, no
window, and an exit code a pre-commit hook, a CI step or an agent can act
on. Everything the CLI does before the pipeline exists is here; the one
command that still refuses is `rewrite` (it needs E4).

`apps/wipemark-cli/src/`: `main.rs` is the argument surface and the exit
codes; `input.rs` reads and decodes a path or stdin through
`wipemark-intake`; `run.rs` is `inspect` and `clean`; `report.rs` the
human report; `audit.rs` the walk and its three renderings; `models.rs` the catalogue
and the downloader. Every write to disk — `-o`, beside the input, and
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
| `audit <dir>` | 0 · 1 · 2 (not there, not a folder, `--json` with `--sarif`) · **3 when any file could not be read, findings or not** | files with findings, the summary, the unreadable files, the third shelf | refusal |
| `audit --json` / `--sarif` | as above | one JSON value / one SARIF 2.1.0 log | refusal |
| `models list [--json]` | 0 · 3 (the folder exists and cannot be read) | the catalogue and the folder | the folder's error |
| `models pull <id>` | 0 (present, or downloaded and verified) · 2 (unknown id, no room, mismatch, cancelled, any failure) | where the weights are | progress, refusal |
| `models verify <id>` | 0 (every byte hashed and matching) · **1** (absent or not matching) · 2 (unknown id) · 3 (could not be read) | the verdict | refusal |
| `models rm <id>` | 0 (removed, or nothing to remove) · 2 (unknown id, could not remove) | what was removed, and what the application will show | refusal |
| `rewrite` | 2, always, in this version | — | "not implemented yet" |

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
   **renamed** there — one directory, so one volume, so atomic, and the
   set-aside copy is the original byte for byte and inode for inode.
4. The cleaned bytes, in the file's own encoding with its byte order mark
   (D11), go to a temporary file in the same folder, are `fsync`ed, take
   the original's permissions and are renamed over the path.
5. If step 4 fails after step 3, the original is renamed back and the run
   exits 2 saying the file was not changed; if that rename fails too, the
   refusal says where the original is now.

The check in step 3 and the rename are two calls: a file created under
that name between them would be replaced. `std` has no no-clobber rename
(`renameat2(RENAME_NOREPLACE)` is Linux-only and needs `unsafe`), and a
race against another process naming a file `x.original.md` in the same
moment is accepted rather than hidden — and documented in
`wipemark_intake::inplace` itself, where the next caller will read it.

Two refusals the document did not list: **standard input** (`-`) has no
file to replace, and **a symbolic link** is refused — renaming it aside
would set aside the *link* and leave the file it points at unchanged, a
run that reports success over a document it did not touch. A hard link
is not refused and cannot be: the replacement is a new inode, so the
other name keeps the old bytes, which is the same rule `-o` has always
kept ("never into an existing file").

## `audit <dir> [--json | --sarif]`

For a pre-commit hook or a CI step: every text file under a folder, read
exactly as `inspect` reads one (`input::read`).

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
| `scanned` | text, decoded, `wipemark_core::inspect` ran | `null` |
| `skipped` | never an error | `hidden`, `link-to-folder`, `not-a-file`, `not-text`, `empty` |
| `unreadable` | a hole in the scan | `unreadable` (I/O, permission), `unnamed-encoding` (8-bit), `invalid` (a bad sequence in a file intake called text), `missing` (gone mid-walk); a folder that could not be listed is listed as `dir/` |

**Exit.** `3` if anything was unreadable — **even when another file had
findings**: a hook must not read a scan with a hole in it as a complete
one, and inconclusive beats a finding. Else `1` if any report is
suspicious (`inspect`'s rule exactly — soft hyphens alone do not make a
tree marked). Else `0`. A `<dir>` that is not there or is not a folder is
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
bytes `inspect --json` prints, third shelf included
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
* `run.properties` carries the Unicode version and the third shelf's ids,
  so the SARIF log is not the one report without them.
* **English, whatever the user's language.** SARIF is a format read by
  dashboards; the rule and message text are the `en-US` catalogue
  rendered with `PlainText`. A rule description that changed with the
  locale of the CI runner would be a rule nobody could match.

## `models list | pull <id> | verify <id> | rm <id>`

The catalogue (`manifests/models.v1.json`, compiled in) and the
downloader of `wipemark-models`, without the window.

**Which folder, which model.** The application's two rows, through a
**read-only** open of `wipemark.db` that never creates one
(`the_cli_never_creates_a_database`): `models.dir` — an absolute path, or
`""` for `<data dir>/models`; a relative one reads as the default and is
left in the row — and `models.rewrite`, which counts only while the
catalogue has the id and it serves `rewrite`. The rules are
`apps/wipemark-app/src/config.rs`'s, restated because the CLI may not
depend on the application; the keys are formats. The CLI writes no row,
ever.

**`list [--json]`.** Every entry: id, name, roles, size, the state on
this machine (`present` — every file matches; `absent`; `partial` with
how far, which `pull` resumes; `mismatch` — on disk and not the
catalogue's), whether it is the model chosen for rewriting, and
`host::fit` for this machine, probed once (`None` is "unknown", never
"no"). Then the weight files in the folder the catalogue did not put
there, from `wipemark_models::scan::weights_under` — listed, never
verified, never loaded, and the heading says so. A state may re-hash a
file whose size or mtime moved since its stamp; that is a CLI with no
window to freeze.

**`pull <id>`.** Present and verified: says so, exits 0, no network.
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
whatever the stamp says — the stamp is a cache of the last verify, enough
to notice a file that was replaced and not a byte changed in place, and
a command asked to verify is not asked to consult a cache
(`a_full_rehash_does_not_trust_the_stamp`). A match refreshes the stamp.
Exit **1** when the file does not match *or is not there* — the same
finding, the file on disk is not the file the catalogue promised — and 3
when it could not be read.

**`rm <id>`.** `Downloads::remove`; exit 0 whether or not there was
anything (and says which). Removing the chosen model says that the
application will show no model chosen until another is picked — and
leaves the row alone.

An id not in the catalogue is refused at 2 by name, with the ids it has,
for every subcommand that takes one.

## Logs

Same file shape as the application's, under the stem `wipemark-cli`
(`docs/architecture/logging.md`). The stderr mirror is off unless
`WIPEMARK_LOG` is set, because stderr is a hook's contract. A log line
carries the command, counts, sizes, the exit code and an `io::ErrorKind`
— never a document's text, and never a path either: the input and the
audit root are `Elided` (`nothing_reaches_the_log_but_the_shape`).

## Not here

* `rewrite` — the pipeline (E4), then E5-2; and the CLI's route to a
  running application's loaded model (D52). Its flags already say D61:
  `--candidates` and `--rounds` have **no default** — absent is "whoever
  rewrites decides" (1 × up to 2 for a model on this machine's CPU alone,
  2 × up to 2 on a GPU or an endpoint), a given count is kept as given,
  and `0` is refused by clap (`rewrite_counts_are_left_to_whoever_rewrites`).
  The help says so in every language; the echo prints `by-executor`.
* Reading `.gitignore` in `audit`.
* Writing any Retention row, or a history.
