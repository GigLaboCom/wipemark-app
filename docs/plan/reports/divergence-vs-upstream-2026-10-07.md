# Why the test rewrite drifted so far — against upstream (2026-10-07)

**The owner's question (2026-10-07):** "compare with the GitHub repo why we
ran into such divergence on the test run". The run: the 2 285-word
Markdown article `article-01-screenshot-mcp-server.md` rewritten by Qwen3.8
27B UD-IQ3_S on our local engine — paraphrase, moderate, GPU 2 × 2, the
most-diverged candidate that passed (D111). Passed candidates diverged 0.86
at the median; the chosen text was longer, formal, lost the second person
and the article's voice, and now and then the meaning (`docs/plan/README.md`,
E4-6b, "Seen on 2026-10-07").

Scripts: [`divergence-vs-upstream/`](divergence-vs-upstream/) —
`prepare.py` (inputs), `run.sh` (the runs), `analyse.py` (the tables). The
bench gained `--temperature`, `--top-p`, `--min-p`, `--base-seed` on `run`
and a `whole` mode; a template variant `bench/variants/keep-voice`. No
product code changed.

## The answer, first

1. **The register shift is what the paraphrase instruction asks this model
   for — ours and upstream's alike.** Our `paraphrase` template is upstream's
   paraphrase prompt in our words ("substantially different at the level of
   individual words … replace both content words and function words
   wherever the meaning allows"), and neither says a word about voice,
   person or register. Qwen3.8 27B obeys it on every candidate, not only on
   the one we pick: the *least* diverged candidate is already formal and
   already drops a third of the "you"s. Upstream's own wording, put in our
   pipeline, drifts *further* (divergence 0.94, words ×1.16).
2. **Per-paragraph chunks make the model inflate, and the length windows
   let it.** Over a 33-word paragraph the model adds a quarter (passed
   candidates ×1.28 in characters, long and short chunks alike); the same
   model given the whole document — upstream's way — *condenses* it (×0.85–
   0.90 in words). The windows (0.6–1.6, 0.5–2.0 under 20 words, D112)
   pass the inflation.
3. **The most-diverged pick amplifies; it does not cause.** Against the
   least-diverged pick on the same candidates it removes 8 more points of
   the original's word pairs (31 % → 23 % left), costs 2 % more words and 2
   "you"s, and the judge sees no more meaning lost (2 of 50 either way). The
   no-op floor (0.2 vs upstream's 0.05) changes nothing here: no candidate
   sits between them.

**One added rule fixes the voice without giving back the divergence:**
"keep the author's voice — speak to the reader the way the text does, keep
its tone and register, plain words, not more formal, not longer" in the
system turn keeps 35 of 41 second-person words (default: 26) at the same
23 % of pairs left, ×1.12 words instead of ×1.15. Recommendation:
§ "Recommendations".

## What upstream does now

Cloned at `main` = **`1181fd4`** (2026-10-02, `v0.7.0-32-g1181fd4`) into
`tmp/watermarks-remover`. Our reference (`docs/sdd/layer-b-rewrite-reference.md`)
was read on 2026-09-09 at v0.7.0. Since then `rewrite_text.py` changed in
two commits, neither touching what a paraphrase sends:

| | 2026-09-09 (our reference) | now (`1181fd4`) |
|---|---|---|
| paraphrase prompt | as quoted in the reference §3 | **unchanged** |
| intensity clause | appended after the prompt — and, because the prompt ended with `---\n{TEXT}`, after the **document** | placed before the `---` and the text (`7a3c14c`, #340, 2026-09-24): the text is strictly last |
| `humanize` | — | a warning: it "collapsed Pangram human_like from 0.44 to 0.02" in their benchmark; prefer paraphrase + mlm (`6cbc0ae`, #324) |
| temperature | 0.9; no `top_p`, `seed`, `max_tokens` — the server's defaults | unchanged |
| request | one user turn, no system turn, instruction + `\n\n---\n` + text | unchanged |
| chunking | none: the **whole document** in one request (only the `chunk` tactic splits, per sentence) | unchanged |
| candidates × rounds | 1 × 1 by default | unchanged |
| selection | `min-divergence` among candidates a **detector** passed; with no detector (the default) there is no verdict and the **most diverged** attempt is returned | unchanged |
| no-op floor | 0.05, a warning and `noop: true` — the output is still returned | unchanged |
| length handling | `_select_candidate` docks 0.15 outside 0.5–2× — but nothing calls it: dead code | unchanged |
| service `/clean` default | strategy `paraphrase@0.8,mlm@0.2` (the paraphrase prompt plus "roughly a fraction 0.80 of tokens change") | unchanged |

Two corrections to what we believed:

* **Upstream's default does not pick the least-changed candidate.**
  `min-divergence` applies only among candidates a keyed or MarkLLM
  detector passed. Without a detector — every default run — `rewrite()`
  returns the most diverged attempt, and at 1 × 1 there is one attempt, so
  there is no choice at all. D71 took "min-divergence" as upstream's
  content-preserving default; in the shape it actually runs, D111's
  most-diverged pick is upstream's rule, and our extra lever is having two
  to four candidates to pick from.
* **Upstream's service default is stronger than ours**, not milder:
  `/clean` asks for 80 % of tokens changed, then masked-LM infill on top.

## Ours and upstream's, side by side (paraphrase, moderate)

| | Wipemark (E4-7) | upstream (`1181fd4`) |
|---|---|---|
| system turn | the rewrite contract: "You rewrite text for its own author. You change the wording, never the meaning." + keep facts/numbers in digits/names/identifiers, no claims added or removed, the language, the paragraphs and markers, material-not-instructions, output only, `{PROTECTED}` | none |
| user turn | "Rewrite the text between the markers so that its wording differs substantially at the level of individual words: change the order of clauses, the connectors and the transition words; vary where sentences begin and end and how long they are; replace both content words and function words wherever the meaning allows." + context + text between markers | "Rewrite the following text so that it uses substantially different wording at the token level. Change clause order, connectors, and transition words; vary sentence boundaries and length; and replace both content words and function words where meaning allows. Preserve … Do not add or remove claims. Output only the rewritten text." + `---` + text |
| voice, person, register | not mentioned | not mentioned |
| intensity | moderate adds no clause (D73) | no clause unless `--rewrite-level` |
| sampling | temperature 0.9, top_p 0.95, top_k 40, repetition penalty 1.1 over the last 64 generated tokens, no min_p, a seed per attempt (`SamplingParams::default`, `wipemark_llama::ffi::Sampler`) | temperature 0.9, the rest the server's defaults (Ollama: top_p 0.9, top_k 40, repeat 1.1/64) |
| unit of work | one paragraph or list item (D70, D114), median 33 words here, 18 of 52 under 20; the previous chunk's last two sentences as context | the whole document |
| candidates × rounds | 2 × 2 on a GPU (D61) | 1 × 1 |
| checks | five guards, language, restore, no-op floor 0.2 | none (Layer A after) |
| selection | most diverged that passed (D111) | most diverged (no detector) |
| length | 0.6–1.6 (≥ 20 words), 0.5–2.0 (shorter) — a rejection | none (the 0.15 docking is not called) |

The sampling is the same in every way that matters; the instruction is the
same text. What differs is the unit of work (a paragraph against the whole
document), our system contract, and that we generate several candidates and
choose.

## Measurements

The article as a one-item bench corpus; Qwen3.8 27B UD-IQ3_S on our
`LocalEngine` (prebuilt llama.cpp `b10731`, Vulkan, RTX 5070 Ti, `n_ctx`
8192; 12 288 for `whole`); four candidates per chunk on the seeds of a GPU
2 × 2 job, base seed 0 (the application's run used 4 071 544 710); every
verdict is the loop's (`job::verdict`). One factor changes per run.
Selection policies are simulated on the same candidates, round by round
as the loop schedules them. `whole` is upstream as it runs: the document in
one request after upstream's instruction, three seeds; its text is matched
to our chunks block by block (`analyse.py`, step 4), so its divergence is an
approximation. The judge is Gemma 4 12B at temperature 0 ("same facts,
claims, numbers and names … differences in wording, word order, **style**
… do not matter"); on this article's calibration it was right 18 of 18
times on each of: a chunk against itself, without its last sentence, with a
foreign sentence appended.

Measures over the 52 prose chunks, words as the loop counts them,
placeholders left out: **words ×** — words of the result over the
source's; **pairs left** — the share of the result's word pairs that are
the source's (the bench's measure, by words; a kept chunk counts whole);
**you** — second-person words (you, your, yours, yourself, yourselves) in
the result, of **41** in the source.

| run | what changed | pick | rewritten / kept | divergence of chosen, med [q1–q3] | words × | pairs left | you (of 41) | judged CHANGED |
|---|---|---|---|---|---|---|---|---|
| app run | — (the owner's run) | max ≥ 0.2 | 50 / 2 | 0.89 [0.84–0.94] | 1.18 | 0.22 | 25 | – |
| **default** (a) | — | **max ≥ 0.2** | 50 / 2 | 0.90 [0.84–0.94] | 1.15 | 0.23 | 26 | 2 / 50 |
| default (b) | pick | min ≥ 0.2 | 50 / 2 | 0.84 [0.74–0.87] | 1.13 | 0.31 | 28 | 2 / 50 |
| default (b) | pick, floor | min ≥ 0.05 | 50 / 2 | 0.84 [0.74–0.87] | 1.13 | 0.31 | 28 | 2 / 50 |
| default | upstream's 1 × 1 | k1 ≥ 0.05 | 46 / 6 | 0.86 [0.81–0.93] | 1.11 | 0.33 | 27 | 2 / 46 |
| default | length window top 1.3 | max ≥ 0.2 | 42 / 10 | 0.87 [0.81–0.93] | 1.07 | 0.39 | 35 | 0 / 42 |
| light (c) | intensity light | max ≥ 0.2 | 51 / 1 | 0.83 [0.76–0.88] | 1.05 | 0.30 | 32 | 2 / 51 |
| light | intensity light | min ≥ 0.2 | 51 / 1 | 0.75 [0.65–0.85] | 1.05 | 0.43 | 34 | 1 / 51 |
| upstream (d) | upstream's prompt as our template | max ≥ 0.2 | 49 / 3 | 0.94 [0.89–0.98] | 1.16 | 0.17 | 24 | 2 / 49 |
| upstream | upstream's prompt | min ≥ 0.2 | 49 / 3 | 0.93 [0.86–0.95] | 1.12 | 0.22 | 24 | 0 / 49 |
| t07 (e) | temperature 0.7, top_p 0.8 | max ≥ 0.2 | 51 / 1 | 0.88 [0.79–0.93] | 1.12 | 0.25 | 35 | 0 / 51 |
| t07 | temperature 0.7, top_p 0.8 | min ≥ 0.2 | 51 / 1 | 0.83 [0.70–0.89] | 1.08 | 0.34 | 31 | 1 / 51 |
| **voice** | + the keep-voice rule | **max ≥ 0.2** | 51 / 1 | 0.89 [0.80–0.94] | **1.12** | **0.23** | **35** | 1 / 51 |
| voice | + the keep-voice rule | min ≥ 0.2 | 51 / 1 | 0.83 [0.64–0.88] | 1.08 | 0.35 | 39 | 0 / 51 |
| voice | + rule, window top 1.3 | max ≥ 0.2 | 50 / 2 | 0.84 [0.74–0.90] | 1.08 | 0.29 | 37 | 0 / 50 |
| voice + light | rule at light | max ≥ 0.2 | 51 / 1 | 0.81 [0.73–0.86] | 1.06 | 0.34 | 39 | 3 / 51 |
| voice + light | rule at light | min ≥ 0.2 | 51 / 1 | 0.74 [0.56–0.81] | 1.04 | 0.46 | 36 | 1 / 51 |
| whole, seed 0 | upstream as it runs | one call | – | 0.78 [0.63–0.86] ≈ | 0.91 | 0.39 | 21 | – |
| whole, seed 1 | upstream as it runs | one call | – | 0.78 [0.66–0.87] ≈ | 0.94 | 0.38 | 17 | – |
| whole, seed 2 | upstream as it runs | one call | – | 0.72 [0.56–0.84] ≈ | 0.91 | 0.44 | 20 | – |

Over the whole document (code, tables and the HTML comment included) the
`whole` answers have 0.85, 0.90 and 0.86 of the source's words and 20, 16
and 18 second-person words of 42; each took ~110 s for ~3 100 tokens and
stopped on its own.

Every candidate of each run (four per chunk), before any pick:

| run | attempts | passed | divergence of passed | length ratio of passed (code points) | rejected | judged CHANGED (passed) |
|---|---|---|---|---|---|---|
| default | 208 | 186 | 0.86 [0.80–0.93] | 1.28 [1.17–1.41] | 17 guard (9 length, 5 identifier, 2 numbers, 1 placeholder), 4 no-op, 1 empty | 8 / 186 |
| light | 208 | 188 | 0.80 [0.71–0.87] | 1.14 [1.06–1.24] | 7 guard, 13 no-op | 4 / 188 |
| upstream | 208 | 169 | 0.93 [0.87–0.97] | 1.36 [1.23–1.47] | 36 guard (16 length, 11 numbers, 8 identifier, 1 placeholder), 3 no-op | 6 / 169 |
| voice | 208 | 186 | 0.86 [0.75–0.91] | 1.20 [1.11–1.31] | 14 guard, 6 no-op, 2 language | 4 / 186 |
| voice + light | 208 | 179 | 0.79 [0.65–0.84] | 1.10 [1.04–1.17] | 6 guard, 22 no-op, 1 empty | 5 / 179 |
| t07 | 208 | 191 | 0.86 [0.76–0.92] | 1.25 [1.14–1.37] | 11 guard, 5 no-op, 1 language | 2 / 191 |

The bench's default run reproduces the application's within a point on
every measure (0.90 vs 0.89, ×1.15 vs ×1.18, 23 % vs 22 %, 26 vs 25), so
the bench stands for the run the owner saw.

## Three paragraphs across the runs

The same chunks in every run: what the loop chose (max ≥ 0.2) unless the
row says otherwise. `⟦1⟧` is the protected `take_screenshot`.

**The intro.** Source: *Your agent is smart, fast, and completely blind. A
**screenshot MCP server** fixes that: it gives Claude — or any MCP client —
a way to capture your screen, file what it captured, and find it again next
week. This post walks through how we built one into Heretic Lazy Shot, why
it exposes 18 tools instead of one ⟦1⟧, and how to wire it into Claude
Desktop, Claude Code, or n8n in about two minutes.*

| run | chosen |
|---|---|
| app run | Your agent possesses intelligence and speed yet lacks visual capability. A **screenshot MCP server** resolves this issue by providing Claude, or any other MCP client, with the means to capture your display, store the acquired images, and locate them later on, even if a week has passed. In this article, we detail the integration process into Heretic Lazy Shot. We also explain why the system offers 18 tools rather than just one `take_screenshot`, alongside instructions for connecting it to Claude Desktop, Claude Code, or n8n in roughly two minutes. |
| default, max = min | Smart and fast but entirely blind, your agent gains sight through a **screenshot MCP server**. This solution allows Claude — or any other MCP client — to grab screen images, store those captures, and retrieve them later. In this article, we detail the process of integrating this feature into Heretic Lazy Shot. We explain why 18 tools are exposed rather than just one ⟦1⟧, and demonstrate how to connect it with Claude Desktop, Claude Code, or n8n in roughly two minutes. |
| light | Although your agent remains completely blind despite being fast and smart, a **screenshot MCP server** solves this problem. It allows any MCP client, including Claude, to capture the screen, store the resulting image, and retrieve it later. In this post, we examine how Heretic Lazy Shot was constructed, the reason for exposing 18 tools rather than one ⟦1⟧, and the process of integrating it into n8n, Claude Code, or Claude Desktop in approximately two minutes. |
| upstream prompt | Lacking visual capabilities, your agent nonetheless proves both intelligent and rapid. A screenshot MCP server addresses this deficit by enabling Claude, along with other MCP clients, to record the display, store those recordings, and retrieve them later. The current entry details integrating such a component into Heretic Lazy Shot, explains why the system presents 18 distinct tools rather than a single one ⟦1⟧, and demonstrates connecting it to n8n, Claude Code, or Claude Desktop within roughly two minutes. |
| voice | Think of your agent as quick, clever, and totally sightless. A **screenshot MCP server** resolves this issue by letting any MCP client — including Claude — grab a picture of the display, store that image, and retrieve it weeks later. In roughly two minutes, you can plug this into n8n, Claude Code, or Claude Desktop. This guide details our construction of Heretic Lazy Shot, explains why we expose 18 tools rather than just one ⟦1⟧, and shows how to connect the system. |
| voice + light | Your agent operates with speed and intelligence, yet remains visually blind. A **screenshot MCP server** resolves this issue by enabling Claude — or any other MCP client — to grab your display, store the captured data, and retrieve it later. This article details the integration of such a feature into Heretic Lazy Shot, explaining why 18 tools are provided rather than just one ⟦1⟧, and showing how to connect it with Claude Desktop, Claude Code, or n8n in roughly two minutes. |
| t07 | Completely blind, yet fast and smart—this describes your agent. A **screenshot MCP server** resolves this issue by providing any MCP client, including Claude, with the capability to capture your screen, store the resulting file, and retrieve it later. This post details how we integrated this feature into Heretic Lazy Shot, explains why 18 tools are exposed rather than a single one ⟦1⟧, and demonstrates how to connect it to n8n, Claude Code, or Claude Desktop in roughly two minutes. |
| whole, seed 0 | Your agent is quick, intelligent, and entirely sightless. A **screenshot MCP server** cures that deficiency: it empowers Claude—or any other MCP client—to record your screen, archive the results, and retrieve them weeks later. This article details how we integrated such functionality into Heretic Lazy Shot, explains why eighteen tools were exposed rather than a solitary `take_screenshot` function, and demonstrates connecting this to n8n, Claude Code, or Claude Desktop within roughly two minutes. |
| whole, seed 1 | Intelligent agents are rapid and capable, yet they remain visually impaired. A **screenshot MCP server** resolves this limitation by providing Claude—or any compatible MCP client—with the ability to record your screen, organize those records, and retrieve them later in separate sessions. … explains the rationale behind exposing eighteen distinct tools … |

The voice rule's intro moved a sentence ("In roughly two minutes, you can
plug this into …") ahead of the one it belonged to — a rearrangement the
guards cannot see, and the judge, asked about facts, did not object.

**A list item.** Source: *You ask Claude Code to fix a layout bug it cannot see.*

| run | chosen |
|---|---|
| app run | Claude Code is tasked with resolving a layout bug, yet it lacks the ability to view the issue. |
| default (max) | Claude Code is instructed to correct a layout flaw it cannot perceive. |
| default (min) | Claude Code is asked to fix a layout bug, yet it lacks visual perception. |
| light | Claude Code is asked to correct a layout defect, yet it cannot perceive the issue. |
| upstream prompt | Claude Code is tasked with correcting a layout defect that remains invisible to it. |
| voice | Claude Code is tasked with resolving a layout defect it has no visual access to. |
| voice + light | Claude Code is asked to correct a layout bug that it cannot see. |
| t07 | Claude Code is tasked with correcting a layout issue, yet it lacks the ability to view it. |
| whole, seed 0 / 1 | - Instructing Claude Code to resolve a layout issue it cannot visualize. / Attempting to fix layout errors in Claude Code without visual feedback. |

Every run turns this "you" into a passive: in a one-sentence item, "change
the order of clauses" leaves the model one move, and the voice rule does not
outweigh it. Upstream's whole-document answer rewrites the four bullets into
noun phrases — a change of the list's grammar, and in seed 1 of its meaning.

**A paragraph.** Source: *Every one of those is the same missing capability.
The screen is where the truth is; the agent needs a sanctioned, predictable
way to look at it — and, just as important, a way to *manage* what it looked
at. A pile of unnamed PNGs helps nobody, agent included.*

| run | chosen |
|---|---|
| app run | Each instance represents the identical absent feature. Truth resides on the screen, necessitating that the agent possess an authorized and foreseeable method for viewing it; equally critical is the capacity to *manage* whatever it has observed. An accumulation of unidentified PNG files benefits no one, least of all the agent itself. |
| default (max) | Each of these reflects an identical gap in functionality. Truth resides on the display; therefore, the agent requires a predictable and authorized method for examining it, alongside—of equal significance—a mechanism to control what has been viewed. A collection of unlabeled PNG files benefits no one, including the agent itself. |
| default (min) | Each of these represents the same absent feature. The screen holds the reality; consequently, the agent requires an approved and reliable method to examine it, along with—equally critical—a mechanism to *manage* the visual data captured. A collection of unlabeled PNG files benefits no one, including the agent itself. |
| light | Each of these represents the same absent feature. Truth resides on the screen; the agent requires a predictable, sanctioned method to examine it — and equally vital, a means to *control* what was examined. An accumulation of unlabelled PNGs assists no one, including the agent. |
| upstream prompt | Each instance reflects an identical gap in functionality. Truth resides on the display; consequently, the agent requires a governed and consistent mechanism for inspecting it. Equally critical is the capacity to control what was observed. A collection of unidentified PNG files offers no benefit to anyone, including the agent itself. |
| voice | Each of those instances points to the same absent feature. The display holds the reality, so the agent requires an authorized and consistent method to observe it; equally critical is a means to *control* what was observed. A heap of unlabelled PNGs benefits no one, not even the agent itself. |
| voice + light | Each of those represents the same absent feature. Truth lives on the screen; the agent requires a permitted, foreseeable method to examine it — and equally crucially, a means to *organize* what it inspected. A heap of unlabeled PNGs aids no one, including the agent. |
| t07 | Each of these instances reflects an identical gap in functionality. Truth resides on the screen; consequently, the agent requires a controlled and predictable mechanism for observing it, alongside an equally critical method for *managing* its visual inputs. A collection of unlabeled PNG files serves no one, including the agent itself. |
| whole, seed 0 / 1 | Each instance represents an identical lack of capability. The display holds the ground truth; agents require a predictable, authorized method to inspect it—and equally critical—a means to *manage* inspected items. A heap of unlabelled PNGs benefits no one, agent included. / … A disorganized pile of unnamed PNG files serves neither humans nor agents effectively. |

The app run's "least of all the agent itself" (a meaning flip of "agent
included") is one draw: none of the bench's 24 candidates for this chunk (six runs
× four) repeat it, and upstream's own answer flips it differently ("serves neither
humans nor agents"). The register is not one draw: "Truth resides …" is in
3 of the default run's 4 candidates for this chunk and in at least one
candidate of every chunked run.

## The causes, ranked

1. **The instruction, unbalanced by anything about voice.** "Different at
   the level of individual words … replace content and function words
   wherever the meaning allows … vary sentence boundaries and length" —
   upstream's wording and ours — read by a capable 27B model, yields the
   thesaurus register: "possesses intelligence", "resides", "necessitating",
   "consequently". It is on every candidate (the least diverged keeps 28 of
   41 "you"s and still reads "Truth resides …"), it is stronger with
   upstream's wording (24 of 41, divergence 0.94, 16 length rejections),
   and one sentence about voice moves the person back (35 of 41) and the
   length down (×1.12) at no cost in pairs left. Temperature is not it:
   ours is upstream's 0.9, and Qwen's own 0.7/0.8 changes the picture by a
   couple of points.
2. **A paragraph at a time, with windows that pass +60 % (+100 % under 20
   words).** Asked to vary sentence length on 33 words, the model adds
   clauses: passed candidates ×1.28 in code points at moderate (×1.14 at
   light, ×1.20 with the voice rule). Given the whole document it condenses
   instead (×0.85–0.90). That inflation is most of the "formal" feel ("is
   smart" → "possesses intelligence" is four words for one). It is also why
   we diverge more than upstream: 23 % of pairs left against upstream's
   38–44 % on the same model. Chunking is deliberate (D70, D114: a fresh
   context per paragraph, a list item per chunk), so the window, not the
   chunking, is the knob — and tightening it alone (top 1.3) keeps 10
   chunks unrewritten, worse on pairs left (39 %).
3. **The most-diverged pick (D111)** takes the furthest of four already
   formal candidates: −8 points of pairs (31 % → 23 %), +2 % words, −2
   "you", the same judged drift. It sharpens the impression; it does not
   create it. **The 0.2 floor (D95)** does nothing on this article.

## Recommendations

* **Prompt wording first** (D64/D66's contract): add a voice rule to the
  rewrite contract (`paraphrase`, and `humanize` with its own wording) in
  en, ru and de. The English candidate is
  `crates/wipemark-pipeline/bench/variants/keep-voice`. Before shipping,
  run it through the whole bench on all four models (`--variant`), because
  one article and one model cannot show what it costs elsewhere; on this
  one it cost nothing in pairs left at the most-diverged pick (23 %) and
  rejected fewer candidates (14 vs 17 guard rejections). A follow-up worth
  measuring with it: "keep about the same length" as its own line, since
  the voice rule moved the length only from ×1.28 to ×1.20 per candidate.
* **Keep D111 for now.** The pick is not the cause of the register; it is
  worth 8 points of pairs left and costs no judged meaning. Its trade-off
  against the word-pair measure is real but bounded: by E4-5's own
  green-list illustration (z ≈ pairs carried × 0.69 × √tokens, threshold
  4), this 3 000-token article sits at z ≈ 8.7 with the max pick and ≈ 11.7
  with the min pick — both far over the threshold, so on a document this
  long the pick does not decide detectability either way, while on a
  one-page text it might. If the owner weighs fluency over those 8 points,
  the cheap middle is "min ≥ 0.2" with the voice rule (35 % pairs left, 39
  of 41 "you", ×1.08 words). A closeness cap between them would need its
  own bench run.
* **Keep D95's floor at 0.2** — it is not in play here.
* **Keep "moderate" as the default intensity**, with "light" as the
  documented lever for voice-sensitive text: light alone keeps 32 of 41
  "you"s and ×1.05 words at 30 % pairs left (as good as today's
  min-pick); light with the voice rule keeps 39 at 34 %. The window should
  not be tightened on its own (above).
* **Give the bench a measure of voice.** D111 was decided on a judge told
  that "style does not matter" and on a corpus of public-domain prose and
  machine text with little second person — the register shift this run
  showed is invisible to it by construction. Second-person retention and
  the word ratio (both in `analyse.py`) are deterministic and cheap to add
  to `bench report`; a register judge ("same tone and register?") would be
  a second, explicitly separate proxy.

## Limits

One article, one model, four candidates per chunk on one base seed per run;
differences of one or two "you"s or a point of pairs left are noise. The
judge is a proxy (a 12B model, its calibration here 54 of 54 on easy
cases; E4-5 measured it missing a dropped sentence 38 % of the time). The
`whole` rows are aligned to our chunks by a heuristic and their divergence
is approximate; `you` and the word ratio over the whole document do not
depend on it. Upstream ran on our engine, not on Ollama: the same
temperature, top-k and repetition penalty, top_p 0.95 instead of Ollama's
0.9, an empty system turn in the `upstream` variant where upstream sends
none.

## How to reproduce

```sh
git clone https://github.com/guillaumemeyer/watermarks-remover tmp/watermarks-remover   # 1181fd4
python3 -I docs/plan/reports/divergence-vs-upstream/prepare.py --upstream tmp/watermarks-remover \
    --article ~/Downloads/article-01-screenshot-mcp-server.md --out tmp/divergence
docs/plan/reports/divergence-vs-upstream/run.sh        # ~60 min on an RTX 5070 Ti
python3 -I docs/plan/reports/divergence-vs-upstream/analyse.py --dir tmp/divergence \
    --app-json <the app run's rewrite.json> --app-out ~/Downloads/article-01-screenshot-mcp-server.qwen38.md
```

The bench's `examples/bench/analyse.rs` fails `cargo clippy --features
local-llama --examples -D warnings` on toolchain 1.95.0
(`unnecessary_sort_by`, line 537) — before this change and untouched by
it; no gate builds the examples.
