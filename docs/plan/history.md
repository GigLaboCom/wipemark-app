# What exists and how it got here

*Moved here verbatim from `CLAUDE.md` on 2026-10-12: the long summary of what the product does, and the epic order with every merge. The plan of record is [README.md](README.md); the first release is [v1-release.md](v1-release.md).*

## What exists

**Both layers exist, and the windows clean and rewrite.** Layer A — the
UCD 18.0.0 tables, the classifier and what it keeps, the scrubber, NFKC,
homoglyphs and the five guards, in `wipemark-core` — is real, and three
surfaces call it: `wipemark-cli inspect|clean|audit` (and `clean
--in-place`), the MCP tools `inspect`/`clean`, and the windows — Clean in
the main window's table and in the panel, and `--clean=<path>`, through
`apps/wipemark-app/src/clean.rs` (E7). Layer B is real too: the loop in
`wipemark-pipeline` rewrites a document with the model this machine holds
or with an endpoint, and three surfaces start it — `wipemark-cli rewrite`
and the MCP tool `rewrite` (E4-6a), and since E4-6b the main window's
**Rewrite** (a button on every row) and **Rewrite all** (its price asked
first) — every one of them an item of the one batch queue
(`wipemark-queue`) the application runs. Real around them: the workspace
and its four gates, the GPUI shell and its Settings window, preferences as
rows in SQLite, the API key in the OS credential store, the model
catalogue and its verifying downloader — which recognises a catalogue
file anywhere under the models folder and removes only what it downloaded
itself (D302, D350) — and models the person adds from a GGUF the
catalogue does not have, held to the sha256 they had when added (E8-1,
`docs/architecture/user-models.md`), the MCP server, the rule that decides who rewrites,
the panel that takes a drop, says what it found and cleans it, the report
with its three shelves, the Compare window, which saves an edited result
where the result lives — on Save, and by default as it is typed (E7-9,
D410–D419) — the menu-bar item on macOS and on Linux (D340),
the **document journal** — a row for every document somebody handed over,
whoever asked: a window, the panel, a launch flag, an agent, the command
line (`wipemark-store` schema 3, `apps/wipemark-app/src/journal.rs`,
D310–D326) — and the **Rewriting** section of Settings, where every
template and the pivot are edited by the rule MCP and the CLI use, checked
on a built-in sample and adapted into another language on a button
(`apps/wipemark-app/src/prompts.rs`, E4-6c, D330–D339). What the windows
still do not do, they say out loud: the panel cleans and says rewriting is
in the main window; there is no source editor with badges, no streamed
result and no Inspector (S7.2, S7.3, S7.5), and no Compare for pictures —
see `docs/architecture/retention.md` ("How the windows execute it"),
`docs/architecture/queue.md`, `docs/architecture/skeleton.md` and
`docs/architecture/layer-a.md` before assuming anything works. Of Layer B,
the local engine is llama.cpp in `wipemark-llama{,-sys}` and
`wipemark_engine::LocalEngine` behind `local-llama`, tested against a real
GGUF (Qwen3 4B, Gemma 4 12B, Qwen3.8 27B) — and, beside Qwen3.8 27B, its
DFlash2 draft, which proposes up to seven tokens a step for the model to
verify in one decode when it is downloaded and the Engine page's **Faster
decoding** row is on (E2-dflash2, `docs/architecture/local-engine.md`, "A
draft model"); the application **loads** the
chosen model, telling how far the load has got (D305), keeps it or lets it
go by the owner's policy (`EngineHost`), and **checks** that it writes;
see `docs/architecture/local-engine.md`. The endpoint is
`wipemark_engine::HttpEngine`, Ollama's native API or any
OpenAI-compatible server, streamed, tested against a fake server and a
live llama.cpp server — and the same **Check** asks it a fixed sentence;
see `docs/architecture/remote-engine.md`. What the pipeline feeds a model:
the preparation of a document — prose told from code and markup,
protected spans as `⟦n⟧`, chunks by paragraph with their context, the
document's language, and the way back byte for byte — in
`wipemark_pipeline::prepare` (`docs/architecture/pipeline.md`); and the
prompts — the shipped en/ru/de templates, the assembler that owns the
markers, the one rule an edited template is held to (`row::admit`, D330;
no invisible character in one, D369), the check and the adaptation
(`prompt::trial`) and the clean-up of an answer, in
`wipemark_pipeline::prompt` (`docs/architecture/prompts.md`). And the
loop: `wipemark_pipeline::start` runs a job — Layer A, candidates ×
rounds, the guards, the language check and the no-op floor, the
most-diverged winner, Layer A again — tested on `FakeEngine` and against
Qwen3 4B; see `docs/architecture/pipeline.md`, "The loop". The batch queue
— `wipemark-queue`: one document at a time, pause, cancel, resumable per
chunk, surviving `kill -9` — asks the application for an engine when each
item **starts**, holds while nothing is on duty (D311), and asks the person
again before a document goes somewhere other than where they agreed
(D361); the windows' clean does not go through it, one thing at a time on
a line of its own (D283).
Images exist too: `wipemark-image` reads and strips provenance metadata
from PNG, JPEG and WebP without touching a pixel; `wipemark-pixels` finds,
proves and takes off a generator's visible mark (Gemini's sparkle, V1 and
V2) over a decoded raster; `wipemark-picture` runs both on a picture file
with one writer; and `wipemark-cli inspect|clean|audit`, the MCP tools
`inspect_image`/`clean_image` and, since E7, the windows' Clean call them
— the windows at the MCP tool's default scope, AI provenance only. That
is the cleaning half of E12-8; Compare for pictures and the batch queue's
picture item are the rest of it, not started.
See `docs/architecture/images.md` and `docs/architecture/visible-marks.md`.


## Epic order and merges

The plan of record is `docs/plan/README.md`; E1 is split there into
seven self-sufficient implementer documents (`docs/plan/E1-1` …
`E1-7`) for the `implementer-xhigh` agent, and every decision taken
beyond the specs is a numbered row (D1 onwards) in its §4.

E0 skeleton (done) → **E1 `wipemark-core` Layer A** → E2 engines →
E3 models → E4 pipeline → E5 CLI → E6 GPUI shell → E7 workspace UI →
E8 models/engine UI → E9 licensing → E10 packaging; E11 images is
phase 2 and E12 visible marks phase 2b — E11-1…E11-3 and E12-1…E12-5
are done (the images series merged 2026-10-05); E12-6 (other vendors)
and E12-7 (the reconstructor) are not started; the E12-R series (the restoration measured against ground truth and made more precise, Grok as E12-6, `docs/plan/E12-R-recon.md`) is merged (below). **E7's windows clean is
done** (E7-1…E7-6, merged 2026-10-05 as `7621c9f`; follow-ups W1–W15
merged as `2f7ce56`; follow-ups X1–X14 merged as `78fd9e2`; follow-ups
Y1–Y9 merged as `bb73dc3`; follow-ups Z1–Z3 merged, squashed), and with it the half of E12-8 that cleans a picture from the
windows. **The windows rewrite too**: `integrate/2026-10-08`, merged into
`feat/e0-e6-shell` as `9b907bb` on 2026-10-08, carried the owner's fixes
F1–F6 and their second round (D300–D306), **E4-6b** — rewriting from the
windows through the batch queue, and the document journal (D310–D326) —
**E4-6c**, the Rewriting section of Settings (D330–D339), the **tray and
the shortcut on Linux** (D340–D347), the divergence research's bench
additions, and the fix round after one host verification (D350–D351,
D355–D364, D365–D369); gates once on `844aa01`, 1623/0/7, the app with
`local-llama` 628/0/2, CI green. Compare scrolls both panes together
(E7-7, D380–D387, merged `9e1eb95`). The queue's consent is checked
against the engine actually handed out (`fix/consent-and-lows`, D370–D375,
merged `72da17d`), and the consent and Compare-scrolling follow-ups are in
(E7-8, D390–D397, merged `021a5e8`). **E8-1, models the person adds** — a
GGUF the catalogue does not have, picked, named, given a purpose, held to
the sha256 it had when added, and Qwen3.8 27B and Gemma 4 12B in the
shipped catalogue (D400–D409, `docs/architecture/user-models.md`) — is
merged as `630d409` on 2026-10-08, with the host verification's two fixes
(`baca2eb`). **E7-9, saving an edited Compare result** — Save, ⌘S and
an autosave on by default, over the result's own file or into its row,
never the original (D410–D419, `docs/architecture/compare.md`) — is
merged as `c333d0b` on 2026-10-08, with the host verification's two
fixes (`cf7cadd`); its Lows and one owner question are open
(`docs/plan/README.md` §7 E7). What remains: of E7, the source editor
with its badges (S7.2), the streamed result
(S7.3) and the Inspector (S7.5); of E4, the keep-voice rule after a
four-model bench and a voice measure in the bench; the rest of E8 (E8-1,
models the person adds, is done, and so are its verification's M3 and
Lows); E9;
E10 (the Linux tray is done; Windows, packaging and the rest are not);
E12-6, E12-7, and of E12-8 Compare for pictures and the batch queue's
picture item. The open findings of E7-8's and E8-1's verifications — one
fact for consent, Compare's leftover asks, the added models' Lows and M3
(the command line writes the identity back) — are merged
(`fix/e7-8-e8-1-followups`, D430–D439, 2026-10-09); that verification's
Medium (tags that say speech refuse a text model) and Lows are in
`docs/plan/README.md` §7 E8. **E4-8, the bench sees the
author's voice** (D420–D429), is merged (2026-10-09); its four-model run is
the owner's (`bench/run-voice.sh`), and keep-voice ships only after it. **Compare's gutters face the middle** (E7-10,
D460–D467, merged 2026-10-09): the submodule on gpui-kit `next`
`d9b7c421`, where the owner's #3416 and #3417 are merged. **The
models-and-pipeline follow-ups** are merged too (D450–D455, 2026-10-09):
speech tags read in three kinds and `macOS-only` no identifier, both at a
default the owner may override, and E8-1's last Lows. **Compare's
follow-ups** are merged (D440–D449, 2026-10-10): an edit saved at quit, a
row cleaned since the window opened asked about, an edit mark that names
its result, one question at a time, "…, then edited" (D447) and Report… of
a journal row greyed (D448), both at a default the owner may switch.
**E7-11, Compare's panes wrapped and padded level** (D670–D679) is merged
(`cecdbce`, 2026-10-12, on the owner's word, before upstream): both panes
wrap or neither (`compare.wrap`, off by default until a wrapped resize fits
a frame, D677), the shorter side of every pair padded with gpui-kit's spacer
rows from the `glani/gpui-component` fork, and inside a changed passage
paragraphs paired by their words (D679). Its follow-up is merged too
(`627af2b`, 2026-10-13): Compare reads the panes' rows through gpui-kit's
`display_map()`, the shape upstream's maintainer gave #3465 (D674 amended),
and the submodule is at `50c120a8` on #3466, rebuilt on it — still the fork
until upstream merges both (D675;
`docs/plan/reports/E7-11-compare-spacer-rows-2026-10-12.md`).
**The 0.1.0 acceptance fixes of 2026-10-13** are merged:
- the macOS acceptance (`36d6220`): the bundle script (P3), a Finder drop
  arriving as its path (F21), and Rewrite all taking cleaned texts (D702);
- `fix/f20-f19` (D709–D714): Ollama asked with `"think": false`, an empty
  answer rejected as `empty`, a rewrite that rewrote nothing reads *Not
  rewritten* and writes nothing on every surface, and the three sentences
  that said the windows do not rewrite corrected, with a gate for a
  sentence that promises less;
- `fix/f1-f7` (D715–D719): a model download in the log, and SIGTERM,
  SIGINT and SIGHUP quitting the way Quit does.

**Stage two's first item, Redo a row** (D703–D708), is merged the same day
(`feat/redo-a-row`): *Clean again* and *Rewrite again* over the row's own
result, an edited result asked about first
(`docs/plan/reports/redo-a-row-2026-10-13.md`).
**The macOS acceptance of 2026-10-14** brought, all merged on `feat`:
- D720: a rewrite that rewrote nothing is cleaned by its row's Clean;
- D721: Compare's panes wrap by default;
- D722: the tray's panel item says Show or Hide as the panel is;
- `feat/panel-anchors` (D723–D727): the panel's nine anchors, flush on
  the visible edges, the Placement page to scale, a turned display keeping
  its anchor (`docs/plan/reports/panel-anchors-2026-10-14.md`);
- D728 (F24): AppKit window changes made once the GPUI update returns,
  `scripts/verify/f24/panel-toggle.sh` 4 errors in 6 toggles before, 0
  after;
- `fix/row-any-origin` (D729–D733; F25): a row's actions depend on what is
  on disk, never on where the row came from — a path the command line or
  an agent named is looked at like a recorded one, a redo from the table
  is the person's, every greyed item says why, and one file down every road
  gets one menu (`docs/plan/reports/row-any-origin-2026-10-14.md`);
- `fix/clean-after-not-rewritten-race`, from CI's macOS lane (gate run
  38630126228, a timeout in
  `a_row_whose_rewrite_rewrote_nothing_is_cleaned_by_its_clean`). It fixes
  two races at the moment a row says *Not rewritten* and the person presses
  Clean (D720):
  - a rewrite's end written late now lands only while the journal row
    still names its item (`Journal::update_item`; `update_open` checks the
    same);
  - a journal read from before a clean of this session no longer undoes
    it (`refreshed`).

  Red first, each guard deleted once:
  `a_rewrites_end_written_late_never_lands_on_the_rows_next_action`, then
  `a_clean_that_ended_is_not_undone_by_a_read_from_before_it`.
**The macOS acceptance of 2026-10-15** brought, all merged on `feat`:
- `fix/f29-cli-symlink` (D734; F29): a `wipemark-cli` linked into the PATH
  finds the bundle's llama.cpp backends, because the program's own
  directory is the link's target;
- `fix/f30-panel-no-activate` (D735–D739; F30): on macOS the panel comes
  and goes on its own, a non-activating panel that `afloat` puts in
  front. A minimised or closed main window stays where it was
  (`scripts/verify/f30/panel-activation.py`);
- `fix/f31-cli-metal-exit` (D740–D743, and D744 on `feat`; F31): no
  process reaches `exit` with a local model loaded. On Metal the CLI's
  exit went 134 → 0/1/2, and the application's quit during a rewrite
  134 → 0 (`scripts/verify/f31/`).
- `chore/gpui-kit-next` (D755–D757): gpui-kit merged #3465 (`779534e2`)
  and #3466 (`8abddf2a`), and the submodule is `longbridge/gpui-kit`
  `next` at `8abddf2a` again, `gpui-pre` still `=0.3.8`, `Cargo.lock`
  untouched. As merged, a pane's `display_map().version()` counts its
  spacers, and Compare's `Level` numbers each pane's rows past the move
  its own answer makes (D756);
- `fix/f28-caller-format` (D750–D751; F28): a caller's row looks as a
  window's row of the same thing does;
- `fix/f23-window-not-found` (D752–D754; F23): a window closes itself
  after the event that asked, on macOS by AppKit's close — Compare,
  Settings, the panel and the title bar;
- `fix/f27-compare-header` (D748–D749; F27): Compare's banner, the line
  under the panes and a refusal wrap inside the window;
- `fix/f26-engine-starting` (D745–D747; F26): until the first look at
  the models folder lands, every surface says the engine is starting,
  and the backends load beside the survey;
- `fix/compare-toolbar-overflow` (D758–D760, the owner's call): the
  result's strip hands the buttons that do not fit its pane, from the end,
  to a more menu whose lines are those buttons — the same press, greying,
  reason, shortcut and check.

The release notes draft is `docs/release/0.1.0.md` (P7).
**E4-9, template profiles** (D510–D516) is merged (2026-10-10): Keep voice
choosable, the shipped templates unchanged. The next free decision block is
**D531** (D490–D502 and D520–D530 are E12-R's, D503–D504 the draft's; D540–D549
were the Grok first look's, D550–D559 its FDnCNN step's, none taken; D570–D573 are
Grok on X's, merged 2026-10-11 — the next block after them is D580). **E2-dflash2** — DFlash2 speculative decoding for Qwen3.8 27B, its draft
as a catalogue entry tied to the model, the Engine page's **Faster
decoding** row, the command line and the bench's `--draft` (D480–D489) — is
merged (2026-10-10). On the owner's host (RTX 5070 Ti, Vulkan) the live
gate holds greedy text byte for byte in en, ru and de, and the draft makes
the model **slower** — 17.7 tokens a second against 33.6, acceptance 3.03 a
step (en ≈ 4.3, ru ≈ 1.3, de ≈ 1.6) — so the row is **off by default**
(D503) and every surface of a draft is hidden (D504,
`wipemark_models::DRAFTS_OFFERED`) until a draft is measured faster; why it
is slower — our port or DFlash2 on Vulkan — is open (`docs/plan/README.md`
§7 E2); `bench/run-dflash.sh` over the corpus is in
`crates/wipemark-pipeline/bench/results/dflash-2026-10-10/`. **The E12-R series** (`plan/recon-2026-10-08`, the restoration measured,
then made more precise) is merged (`recon/decided`, 2026-10-10): R1 and
R3–R8 are done, R12's stage 4a is done, and the tools of R2, R9 (behind
`blend-preview`), R10, R11 and R12's stage 4b are written; R3 reads
upstream zune-jpeg's raw output through the fork's getter
(`GigLaboCom/zune-image`, `raw-quantization-tables`, a git dependency,
`docs/architecture/zune-jpeg-pin.md`); the owner's D470–D472 are taken —
the search does not prove a mark its row refused by gain, a 4:2:0/4:2:2
JPEG is proved and restored in its planes, and DCT-POCS refines a lossy
JPEG's restoration, on the product's road (`planar-preview` removed).
Gemini's visible mark is gone from new generations (the owner,
2026-10-10: no more pictures will come), so `gemini-midtone` is dropped and
R4 §4.3–4.4 and R9 are not done (`docs/plan/E12-R2-corpora.md` §3); the
shipped profiles stay for the pictures already out. **The close-out** is
merged (`recon/closeout-grok-ready`, 2026-10-11, D520–D529): the Gemini line
closed (D520), R9 parked behind `blend-preview` (D521), the bounds at their
defaults (D522–D524) and the steps' smaller questions at theirs (D525); and Grok
is one folder away — the owner's capture guide
(`docs/plan/E12-R-grok-capture-guide.md`) and `scripts/grok/run.sh`, which takes
`grok-captures/<web|x|api|video>/` through stage 0 and R11's stages 1–3 to a
provisional `--catalogue` profile (D526–D529). What remains is the owner's: the
captures, the stage-0 table's hand columns, the ZIP upload, then the host runs
R11 and R12 owe (`docs/plan/reports/E12-R-closeout-2026-10-10.md`, K4). **D530** is merged too (`recon/search-gain-resampled`, 2026-10-11):
the search proves a resized mark only inside its width's gain band (`search.gain`
in the catalogue; absent is the product before), every real resized finding kept
(`docs/plan/reports/E12-R-search-gain-2026-10-11.md`). Grok's first look and its
FDnCNN step (`recon/grok-first-look`, `recon/grok-fdncnn`) are pushed, not merged. E1 and E3 parallelise in separate worktrees; E5 lands before
E6 and gives agents a usable product before the GUI exists.
