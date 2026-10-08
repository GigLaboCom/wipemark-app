# Model downloads

How weights get onto the machine, how the product knows they are still
the weights it asked for, and how it decides which of them to offer.

Epic **E3 / S3.1**. The catalogue is real, the downloader is real, and
the Settings page that drives them is real. Nothing **loads** a model:
that is epic E2, and the banner at the top of the page says so in every
state it can be in.

* `crates/wipemark-models/src/manifest.rs` — the catalogue
* `crates/wipemark-models/src/host.rs` — what this machine can hold
* `crates/wipemark-models/src/store.rs` — the downloader
* `apps/wipemark-app/src/models.rs` — the page's vocabulary, as values
* `manifests/models.v1.json` — the two entries that ship

## What ships, and why those two

| entry | download | needs | license |
|---|---|---|---|
| `gemma-3-12b-it-qat-ud-q4` | 7.4 GB | ~9216 MB | gemma |
| `qwen3-4b-instruct-2507-ud-q4` | 2.5 GB | ~4608 MB | apache-2.0 |

Two, not one, and not six. The point of the pair is that a 12 GB
machine and a workstation are each offered something they can run,
rather than the same thing with a warning beside it — the smaller one is
chosen because the larger does not fit, never because it is better.

Both are quantization-aware or dynamic-quantized builds from the same
mirror, and both choices have a reason that is written down rather than
assumed:

* **QAT, not post-hoc quantization**, for the 12B. The damage is trained
  around rather than measured after. This is the lesson `heretic-mnemoria`
  records as its "QAT-only policy for stable chat models", and it is
  cheaper to inherit than to rediscover.
* **The Instruct build, not the hybrid**, for the 4B. A model that
  reasons before it answers burns tokens nobody reads:
  `docs/sdd/layer-b-rewrite-reference.md` records 9,894 completion
  tokens against 12 for the same rewrite. The same measurement is why
  the Engine page defaults `reasoning_effort` to `none`.
* **An ungated repository.** The downloader has no Hugging Face account
  and sends no `Authorization` header, so a repository gated behind an
  accepted licence cannot be fetched at all.
  `google/gemma-3-12b-it-qat-q4_0-gguf` is `gated: manual`; the unsloth
  mirror of the same weights is not.

`vendor: "open-llm"` is a statement about how a model is *served* —
locally, under the user's control. Gemma is Google's open-weight family,
and the entry's `notes` says so rather than letting the vendor field
imply a lineage it does not describe.

## Roles: the purpose classification

An entry declares a **list** of roles, not one task, because a model
serves a purpose and some models serve two. The five:

| role | what it is for | ships |
|---|---|---|
| `rewrite` | Layer B: rewriting the user's text | yes |
| `detect` | scoring a candidate — the selection loop's evaluator | no |
| `fill-mask` | the `mlm` tactic, recorded in the SDD as a deliberate gap | no |
| `embed` | the similarity floor that stops a no-op rewrite shipping | no |
| `pixel` | phase 2b image work (E11) | no |

Every one of them names something this repository already describes.
The choice is made **per role** — `models.rewrite` is a settings row,
`config::model_key` spells the rest — because "which model rewrites" and
"which model scores a rewrite" are different questions, and a machine
with three models downloaded has three answers to give.

Two gates hold the classification together:
`every_shipped_model_is_a_text_model` (v1 ships no `pixel` entry,
because there is no engine for one) and
`a_role_the_catalogue_serves_has_a_row` (a role with a catalogue entry
and no way to select it is a download with no purpose).

## The catalogue is data, and it is checked

Adding a model is a JSON edit. `Manifest::parse` refuses, rather than
normalises, every one of these — and each is a rule whose absence
produces a *silent* wrong answer:

| refused | what it would otherwise be |
|---|---|
| a duplicate id | two entries sharing one directory |
| an id that is not a plain name | a download outside the models directory |
| two files with one basename | a directory whose sha256 flips between two files |
| `hf://…` with no `@<commit>` | a checksum that fails months after the edit that caused it |
| a URL with userinfo | a credential written into a data file |
| `http://` to anywhere but loopback | weights fetched over a wire nothing protects |
| a weight file with no sha256 | a download nothing checks |

`sha256` and `size_bytes` are **read off Hugging Face**, never
estimated — `manifests/README.md` has the two commands. The `lfs.oid` in
the tree API is the file's sha256, and the `x-linked-etag` on the 302
confirms it; the etag on the 200 that follows is the CDN's own and is a
different value.

## The download

Blocking, on a thread the caller owns, reporting through a `flume`
channel — the shape every long operation in this product has, because
the GPUI executor and tokio cannot await each other's futures.

```text
<models folder>/<id>/<file>            the weights, where a download puts them
<models folder>/<id>/<file>.part       a download in progress
<models folder>/<id>/meta.json         what was fetched, and when — written
                                       by a download, never by a verify
<models folder>/…/<file>               a catalogue file found anywhere else
                                       under the folder (D302)
<data dir>/records/<key>-<file>        size:mtime and sha256 at the last
                                       hash, keyed by the file's path (D303)
<data dir>/records/<key>-<file>.downloaded       a download's mark, with the
                                       identity of the file it marks (D302, D350)
<data dir>/records/<key>-<file>.part.downloaded  the same for the `.part` a
                                       download opened (D351)
```

`<models folder>` is `<data dir>/models` unless the `models.dir` row
names somewhere else — see [The folder](#the-folder-and-what-else-is-in-it)
below.

What is checked, and when:

* **Before the first byte** — that the URL is one this build will
  request at all, and that the volume has room for what is left plus
  half a gigabyte. A download that fills the disk takes the database and
  the log down with it. A volume that cannot be read is an *unknown*,
  not a refusal: the download runs and fails on write if there really is
  no room.
* **During** — nothing. The bytes go to a `.part`, which is not the name
  anything reads.
* **After** — sha256 over the finished `.part`, and only then the rename
  into place. A crash can never leave a file that passes verification
  without having earned it. A mismatch deletes the `.part` too: keeping
  it would keep a resume point that can only ever produce the same wrong
  file again.
* **On every later look** — the size and mtime recorded at the last
  hash, beside the sha256 the file had then. They match and the recorded
  hash is the answer without re-reading seven gigabytes; they do not and
  it is hashed again. The record is not a security check — the sha256
  is — and anything that moves the file re-hashes it.

### Records under the data directory, never beside the weights (D303)

Until 2026-10-07 a verify left `.<file>.ok-<sha256>` beside the file,
and a fetch of an entry that was already whole wrote `meta.json` (with
`fetched_at_unix`) beside weights it had not fetched — both seen on the
owner's model mirror, a folder shared with another program. Now:

* What a verify learned is a **record** under `<data dir>/records/`
  (`Layout::records_dir`), one per weight file, named
  `<first 32 hex of sha256(path)>-<file name>` — the path is the folder
  made absolute (`canonicalize`) joined with the file name, so a file
  reached as `/var/…` and `/private/var/…` is one record. It holds
  `size:mtime`, the sha256 the file *had*, and the path. Recording the
  hash rather than "it matched" is what lets a file with a catalogue
  file's name and size and another hash be read once rather than on
  every look, and lets a manifest that changes an expected hash be
  compared against what the file is rather than a stale "ok".
* A record that cannot be written is a warning: the next look hashes
  again, and the verify still answers.
* `fetch` of an entry already whole — at its place or found elsewhere —
  downloads nothing and writes nothing. `meta.json` is written only
  after a download, in the entry's own `<models>/<id>/`.
* A stamp the old layout left beside a file is not read and not
  removed (it is hidden, so the walk never lists it; `remove` deletes
  the one beside a download of ours). The first look after the move
  hashes each file once and records it
  (`a_stamp_beside_the_file_is_not_needed`).

A folder held read-only verifies and loads with nothing written in it
(`a_read_only_folder_verifies_with_nothing_written_in_it`, which also
holds that the record saves the second look its hash).

### Found wherever it is (D302)

A catalogue model is not only where a download puts it. When a file of
an entry is not at `<models>/<id>/<file>`, the walk of the folder
(`scan::weights_under`, eight levels) is searched for it: every file of
its **name and size** is a candidate, its **sha256** decides, and the
first that matches in path order is used. A same-name file of another
size is not even hashed; one of the right size and another hash is not
used, stays under "Also in this folder", and is read once (its record
remembers the hash it had). An entry with no sha256 is never recognised
elsewhere — a name and a size are not a confirmation.

A file found that way is **the user's**:

* it is `Present`, and `Downloads::weights_path` hands it to the engine;
* the card says where it was found and offers no button — no Download,
  because nothing needs fetching, and no Remove
  (`Availability::Found`, `settings-models-found-at`;
  `a_model_found_elsewhere_offers_no_remove`);
* `remove` deletes only what a download of this product wrote — a file
  at its place carrying the download's mark (amended below) — and never a
  found file, nor anything else a person put in `<models>/<id>/`
  (`delete_leaves_a_file_found_elsewhere_alone`,
  `a_file_at_its_place_that_no_download_wrote_is_never_removed`);
  `wipemark-cli models rm` says where the file is and that nothing was
  removed (`cli-models-rm-found`), and `models list` adds "found at"
  (and `found_at` in `--json`).

One walk serves the whole page: `Downloads::survey` walks once and
locates every entry in it, and the same listing, less every file of
every entry wherever it was found, is "Also in this folder".
`Downloads::locate` (and `state`, `weights_path`) walk only when a file
is not at its place.

**Amended after the host verification (H1, 2026-10-07).** The first
version took a file at `<models>/<id>/<file>` as the product's own,
whoever put it there — and the owner's mirror is laid out exactly
`<id>/<file>`, so its Qwen card read Installed with a Remove that deleted
12 GB another tool had put there (`scripts/verify/owner-fixes/rm-in-a-mirror.sh`
said DELETED). Now **a download leaves a mark a look never writes**: when
`fetch_one` renames a verified `.part` into place it writes
`<data dir>/records/<key>-<file>.downloaded` beside the file's record
(`Downloads::mark`), and only a file with that mark is the product's:

* a file at its place **with** the mark is a download of ours — Installed
  with Remove, Damaged with Remove when it no longer matches, and the
  only file `remove` deletes or `fetch_one` replaces;
* a file at its place **without** the mark is another tool's: when its
  sha256 is the catalogue's it is used where it is and the card says
  "Found at …" with no button, like one found elsewhere; when it is not,
  the entry stays absent, the card says "A file at … has this model's name
  but not its contents … move it away to download this model here"
  (`Availability::Foreign`, `settings-models-foreign`) and offers
  **nothing** — no Download, which would have to write over it, and no
  Remove; `fetch` refuses with `StoreError::Occupied`, the file untouched;
* `remove` deletes, at the entry's place, only marked files, their old
  stamp, the `.part` (the download's own working name) and — only when a
  marked file went — `meta.json`; the directory only if that empties it.

**A download made by a build from before the mark has none**, so it now
reads as "Found at …" and cannot be removed from the page; that is the
safe side. To have it removable again, delete it by hand and download it
anew. A mark that cannot be written is a warning: the file then reads as
another tool's, the same safe side.

**A mark names the file, not the place (D350, after the host
verification of 2026-10-08, A1).** The first mark held a time and a path
and nothing about the file, and was never dropped: a download deleted by
hand, or replaced at the same path by another tool — a `models.dir` that
is the owner's mirror, laid out `<id>/<file>` — left a mark that made the
new file "ours", so Remove deleted it, a fetch removed it and downloaded
over it, and the card read Damaged with Remove
(`scripts/verify/owner-fixes/rm-through-a-link.sh`, case `stale`, said
DELETED). Now the mark is

```text
wipemark download mark 1
<identity>
<fetched at, unix seconds>
<path>
```

and the identity is what the file was when it was marked, read without
following a link: `size:mtime_ns:dev:ino` on Unix, `size:mtime_ns:birth_ns`
elsewhere. A file at the place is a download of ours only while it has
that identity; anything else — written over in place (the mtime moves),
renamed over (a new inode), a symbolic link put there (never a regular
file), or a mark in the old shape with no identity — reads as another
tool's file (Found when it matches, Foreign when it does not; never
removed, never downloaded over), and the mark is dropped, as it is when
its file is found absent. A mark that names another file can never be
right again. The cost is on the safe side: a download of ours whose mtime
something else touched is "Found at …" from then on, not Installed with
Remove (`a_mark_names_the_file_and_not_the_place`,
`a_mark_whose_file_is_gone_is_dropped`,
`a_link_at_a_marked_place_is_not_the_download`).

**A `.part` is ours only when a download opened it (D351, A2).** Remove
deleted `<id>/<file>.part` unconditionally, a look read any `.part` as a
resumable download, Resume appended HTTP bytes into it, and a mismatch
deleted it — so another tool's download in progress under the same name
(case `dirlink`: `<models>/<id>` a link into a mirror) was lost. Now a
download creates its `.part` with `create_new` and marks it at once,
before the first byte, with an identity that survives appending:
`dev:ino:birth_ns` on Unix (`-` where the file system keeps no birth
time), `birth_ns` elsewhere. Only a marked `.part` is a resume point, is
truncated when a server ignores the range, and is removed (by Remove, or
on a mismatch); its mark goes when it is renamed into place or thrown
away. An unmarked `.part` is another tool's: the entry is not Partial,
the card says it is there as it would another tool's file
(`Located::mismatched`, `Availability::Foreign`), Remove leaves it, and a
fetch refuses with `StoreError::Occupied` before any request
(`a_part_nobody_marked_is_never_resumed_or_removed`,
`a_download_marks_its_own_part`). A `.part` left by a build before D351
has no mark and reads the same way: move it away, and the download
starts over.

Loading a GGUF that is in no catalogue — the user's own model,
unverified — is an owner question, not this rule.

### Resume, and the 200 that ruins it

An interrupted download leaves its `.part` and the next attempt sends
`Range: bytes=<n>-`. A server that honours it answers **206** and the
tail; one that does not answers **200** and the whole file. Appending
the second to a `.part` produces a file of exactly the right length and
entirely the wrong contents — which is what the sha256 is there to
catch, and what nobody notices until a model loads as noise. So the
offset goes back to zero unless the status really was 206.
`a_server_that_ignores_a_range_does_not_corrupt_the_file` is the gate,
and deleting the check turns it red with that exact hash mismatch.

Verified against the real thing, not only a mock: a download of the
shipped Qwen entry was cancelled at 27 MB, resumed to 45 MB, and the
bytes on disk compared byte-for-byte against an independently fetched
`curl --range 0-45039770`. `a_real_file_comes_down_from_hugging_face_and_verifies`
is the `--ignored` test that keeps the whole path — `hf://` resolution,
the 302 to the CDN, TLS through the operating system's trust store, the
sha256 — runnable on demand without putting the network in CI.

### Cancelling keeps the bytes

Stopping a download leaves the `.part` alone and the page says so.
Deleting six gigabytes because somebody closed a window is not a
kindness, and the tone of that message is a warning only when something
actually failed — `stopping_a_download_is_not_reported_as_a_fault`.

### No credentials, so redirects are safe

There are none. The catalogue holds public weights, the downloader sends
no `Authorization` header, and a URL carrying userinfo is refused before
it reaches the store. That is what makes following Hugging Face's
redirect to its CDN safe: the rule this product inherited from upstream
— never let an `Authorization` header follow a 3xx — is satisfied by
never having one to leak. Adding a Hugging Face token later would mean
adding the redirect rule with it, not instead of it.

## What this machine can hold

`Host::probe` reads total RAM through `sysinfo` (cross-platform), treats
Apple silicon as unified memory by `cfg`, and asks `nvidia-smi` for
video memory on Linux and Windows. Everything downstream is a **pure
function over the probed value**, so the policy is testable on a machine
that is neither of the ones it describes.

`None` always means *unknown*, never zero:

* There is no portable way to ask a graphics card its size without
  linking a vendor driver, and this crate forbids unsafe code. A
  discrete AMD or Intel GPU therefore reads as unknown. That is a
  **deliberate gap**, recorded rather than papered over: the wrong
  answer would be a model refused on a machine that could run it.
* `nvidia-smi` is a process spawn, not a `dlopen`. A desktop application
  opening its Settings window can afford one where a server accepting
  requests could not — and it is given three seconds before it is
  treated as absent, because a driver in a bad state leaves it hanging
  and a Settings window that never opens is the worse failure.
* `an_unmeasured_gpu_is_not_a_small_gpu` is the gate.

`fit` answers on RAM alone, because RAM decides whether a model can run
at all — a machine with enough of it can always fall back to the CPU.
Video memory decides how *fast*, which this module does not claim to
predict. Four answers: `Fits`, `Tight` (it would fit and little else
would — offered, with the number said out loud, rather than hidden),
`TooBig { short_by_mb }`, and `Unknown`, which is never rendered as
"no".

The two thresholds are not round numbers, and that is the point:

| threshold | value | why not the round one |
|---|---|---|
| `CONSTRAINED_VRAM_MAX_MB` | 13000 | a 12 GB 4070 reports ~12282, a 3060 reports 12288, the next tier reports ~16376 |
| `CONSTRAINED_UNIFIED_RAM_MAX_MB` | 17000 | a 16 GB Mac reports exactly 16384; the next configuration is 18432 |

Both, with the reasoning, come from `mnemoria-lvkb`'s
`models::recommend`. `the_thresholds_separate_the_real_configurations`
asserts them against the sizes real hardware reports.

## The page

`Settings → Models`, or `--settings=models`.

A banner (what this machine reports, which folder and what is in it,
what the last download did, and that nothing loads a file yet), the row
that chooses the folder, the row that chooses the rewrite model, one
card per catalogue entry, and — when the walk turned any up — the model
files the catalogue did not put there.

Each card offers **exactly one** action, because a card with a Download
*and* a Remove has to explain which one applies:

| on disk | button |
|---|---|
| nothing | Download |
| a `.part` | Resume — never "start over", which would throw the gigabytes away |
| running | Stop |
| present and matching | Remove |
| present and not matching | Remove, under a sentence saying what is wrong |

Nothing is repaired silently. A file whose bytes are not the catalogue's
bytes is named, and the user decides.

One download at a time: the free-space check that let it start was made
for one file, and every other card's button is disabled while it runs.

The selector lists **only models that are on this machine**. Choosing
one that has not been downloaded is choosing a file that does not exist
— a preference that reads as done and fails at the first request.
`a_model_that_is_not_on_this_machine_is_not_offered` is the gate, and
removing the chosen model clears the choice rather than leaving a row
naming a file that is gone.

### The folder, and what else is in it

Where the weights live is one row, `models.dir` — an absolute path, or
`""` for the platform's `<data dir>/models`. One row and not one per
model, because the question it answers is "which disk": a laptop with a
small internal drive and a large external one has one answer for every
model. The row is read generously and refused strictly, the way every
other row is: a relative path would follow the working directory, which
for an application launched from the Dock is `/`, so it reads as the
default and is **left in the row**
(`a_models_folder_that_is_not_absolute_falls_back_without_rewriting_the_row`).

On the page it is a field with two buttons under it — the platform's
folder picker (`App::prompt_for_paths`, no new dependency) and
"Default" — and the field is the setting: both buttons write *into* it,
the way the address presets do. It commits on **Enter and on blur, not
on change**, unlike every other field in the window, because a folder
is read recursively the moment it is chosen and `/Users/` is an
absolute path on the way to `/Users/me/models`: committing per keystroke
would walk a home directory once per letter typed. `~` is expanded,
an empty field puts the default back, and anything else is put back to
what the field showed (`models::folder_typed`). Whether the folder
*exists* is not the field's question — on a fresh install the default
one does not either — and the banner answers it: "does not exist yet;
the first download creates it" is the ordinary state and is not painted
as a fault, while a folder that exists and cannot be read is
(`an_unreadable_folder_is_a_warning_and_a_missing_one_is_not`).

The folder **cannot move while a download runs**: the bytes are landing
in the old folder through a client the download thread holds, and a
page showing the new folder over a bar filling the old one would be
lying about where the file will be. The controls are disabled for that
span with a sentence saying why, and `Preferences::select_models_dir`
refuses independently. Moving it forgets everything the last scan said
and asks for a new one, and **keeps the chosen rewrite model**: the
choice names a catalogue entry, not a path, so if the new folder has
the same entry downloaded it is still right, and if it does not, `duty`
says so and switching back restores it. Clearing it would make trying a
second folder cost the choice made in the first.

The scan is **recursive**. `wipemark_models::scan::weights_under` walks
the folder to eight levels, listing every regular file with a weight
extension (`Format::extension` — `gguf`, `onnx`) that is not hidden and
not reached through a linked *directory* (a link to a parent is a
loop; a linked *file* is listed, because sharing one seven-gigabyte
file between two tools is what a symlink is for). It reads no byte of
any file: recognition is by extension, and the list says so. The
catalogue's own downloads are found by the same walk and subtracted
(`Folder::from_listing`), whatever state they are in — a damaged
download is still the catalogue's and belongs on its card, not in the
list of strangers. What is left is **listed and nothing more**, under
"Also in this folder": where under the folder, how big, and the
sentence that nothing here can verify a file the catalogue has no
checksum for and nothing loads one yet. There is no button, because
there is nothing this build can do with one, and a card with no action
looks broken. The banner carries the folder and the count in every
state once the scan has answered
(`the_banner_names_the_folder_once_it_has_been_read`), so an empty
folder is described rather than silently missing a section.

### A bar while bytes move (D306)

The owner's 2026-10-07 session: a download said only "3.1 GB of 12.0
GB", and the look at a 12 GB file already on the disk said "Checking what
is already here…" for minutes with nothing moving. Now a card draws
gpui-component's own progress bar (`models::bar`, one place for the
Models page and the walk-through's model step) whenever bytes move:

* **while a download runs**, and **while one waits to be resumed**, at
  its fraction, with "*done* of *total*" kept in the line above it;
* **while a file of the entry is hashed** — a look at a file whose record
  moved, a verify, the check of a download that just finished (its `.part`
  read under its working name) — as `Availability::Checking`, "Checking
  *done* of *total* against the catalogue…", with no button: nothing can
  be pressed until the file is read, and the state wins over every other
  (a download being checked is no longer downloading; a model on the disk
  is not known to be whole until it is read).

The hash's progress is the store's: `Downloads::watch_hashes` takes a
`flume` sender of `Hashing` — `Progress { path, done_bytes, total_bytes }`
from zero, at most one per 120 ms (`REPORT_EVERY`, the download's own
pace) and at the whole file, then `Done { path }` however the hash ended.
`Preferences` attaches a listener to every `Downloads` it builds (at
startup and when the folder moves), keeps the file being hashed and how
far, and `Preferences::checking_for(entry)` is what a card reads. With
D304 there is one scan at a time, so one file is read and one bar moves.
Gates: `store::tests::a_hash_tells_how_far_it_has_got`,
`models::tests::a_card_has_a_bar_while_bytes_move_and_none_when_installed`,
`models::tests::the_bar_is_painted_mid_download_and_not_when_installed`
(painted, with a height, and not over an installed model),
`settings::tests::a_hash_in_progress_reaches_the_card` (a real 3 MB look
through the folder row's store: the card's bar went 0 → 100 and is gone).

### Nothing blocks the window

The probe may spawn a process and the scan may re-hash gigabytes, so
both run on the background executor and both happen when the Settings
window opens — not at startup, for the same reason the API key is not
read at startup.

**One scan at a time (D304).** `Preferences::look_at_models` is asked
by the main window, by Settings opening and after every change on disk;
on 2026-10-07 three scans ran at once over one twelve-gigabyte file
(three descriptors on one `.gguf`). Now a scan asked while one runs
starts nothing and sets `rescan`; when the running one lands its answer
is set aside — it may describe a disk that has since changed — and one
more scan runs, after it, never beside it. Whatever the first one hashed
is a record by then, so the second reads the record instead of the file
(`two_scans_asked_back_to_back_hash_a_file_once`, over
`Downloads::hashes`, the count of full hashes a store has made). A
`wipemark-cli models verify` in another process is not joined: it is
the one command asked to hash in full.

## What is deliberately not here

* **Loading a model.** E2. The banner says so in every state
  (`the_models_banner_always_says_the_weights_are_not_used_yet`).
* **A signed remote manifest.** The embedded copy is trusted because it
  is compiled in; a mirrored one needs the Ed25519 check that lands with
  E9. Every parse rule is already written as though the manifest were
  hostile, because the day that becomes true is not the day to start.
* **Video memory on a non-NVIDIA discrete GPU.** See above — a recorded
  gap, not an oversight.
* **More than one download at a time**, a bandwidth limit, and a
  scheduled or automatic download. None of them is needed to get the
  first model onto a machine, and each would need a surface of its own.

## From a downloaded file to a running model

`Downloads::weights_path` hands back the `.gguf` an engine opens, and
that is the whole of the handover: this crate describes and fetches a
file, and knows nothing about loading one. Which role that file serves,
and whether it serves it at all rather than an HTTP endpoint doing so,
is `apps/wipemark-app/src/duty.rs` — see [who-rewrites.md](who-rewrites.md).
A model is put on duty only when it is **whole**: `State::Present` and a
weights path together, because a path under any other state names a
`.part`, and a `.part` is a file no runtime can open.

## Which model, and when it becomes the one that runs

The selector on the Models page lists only what is on this machine, so
on a fresh install it has one row: *None*. That leaves two questions the
selector cannot answer, and `apps/wipemark-app/src/models.rs` answers
both as pure functions over values.

**Which one should I get?** `recommended` asks the catalogue —
`host::default_for_role`, which picks the best-rated stable entry this
machine has comfortable room for, then a tight one, then the smallest.
"Comfortable room" is stricter on a machine `Host::is_constrained`
names: a 16 GB Mac *runs* the 12B — 9216 MB is under three quarters of
16384 — and is still pointed at the 4B, because on that machine the
entry has to claim no more than half the pool the model competes for.
That is the mnemoria rule the thresholds above were taken for, applied
at last; `a_constrained_machine_is_offered_the_small_model` is the
gate, and [setup.md](setup.md) is where the recommendation is put in
front of a first-time user. It shows as a badge on the card. It answers `None` before the machine
has been probed, because a recommendation made against an unread machine
is a guess wearing a badge, and `None` once anything is chosen, because
the tick on the card is then the answer and a second badge beside it
would be the page arguing with itself.

**What happens when I get it?** `adopted`: the first model to finish
downloading takes the role it serves. The expensive half of choosing a
model is fetching it, and a user who has waited out two and a half
gigabytes and is then told nothing is on duty has been asked the same
question twice — the second time in a dropdown three rows above the
button they just pressed.

Only the first, and that is the load-bearing half. A later download is a
comparison, not a replacement: somebody who has chosen a rewriter and
then fetches another to try it has not asked for the switch, and a
selection that moved on its own is one they would have to notice before
they could undo it. The mirror was already here — `remove_model` clears
the choice when the chosen model is deleted, because a preference naming
a file that is not there is worse than no preference.

Neither function writes anything. `recommended` is a suggestion and
`adopted` returns an entry; `Preferences::adopt` is the one place that
turns the second into a settings row.
