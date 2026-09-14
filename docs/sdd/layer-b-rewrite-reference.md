# Layer B — the upstream rewrite reference

What `guillaumemeyer/watermarks-remover` (MIT) actually sends to an
Ollama or OpenAI-compatible endpoint, and which parts of it Wipemark
takes.

The spec (`heretic-unmark-overview-decomposition-2026-09-07`, §4.3) says
the implementer takes the prompt defaults "по референсной SKILL.md
(раздел 4) и адаптирует под правила выше". This document is that
reference, read out of the source rather than out of the README, so that
E2 (`wipemark-engine`) and E4 (`wipemark-pipeline`) can be written
without a second archaeology pass.

**Provenance.** Read at commit of `main` on 2026-09-09, upstream version
0.7.0, from a throwaway clone under `tmp/` (gitignored — see
`.gitignore`). Nothing from that tree is vendored: we take the shape,
not the code. If any of it is ever copied verbatim, it is MIT and needs
a `NOTICE` entry.

**What upstream is.** A Python HTTP service (`service/scripts/server.py`)
plus two Claude Code skills that talk to it over `curl`. Layer A is
`text_unicode.py`; Layer B is one file — `service/scripts/rewrite_text.py`,
1437 lines — which is the whole of what follows. We do not port the
architecture: Wipemark is one process with no service, no Python and no
torch. We port the taxonomy, the ladder of tactics, the non-origin rule
and the honest-report discipline.

---

## 1. The two backends, on the wire

`rewrite_text.py` has three backends: `print-prompt` (default — emits the
prompt and exits, so CI never needs a model), `ollama`, and
`openai-compatible`. Only the last two make a request.

### Ollama

```
POST {base_url}/api/chat
Content-Type: application/json

{
  "model": "<model>",
  "stream": false,
  "messages": [{"role": "user", "content": "<prompt>"}],
  "options": {"temperature": 0.9}
}
```

Answer is read from `data["message"]["content"]`; an empty one raises.
Default `base_url` is `http://127.0.0.1:11434`.

Two things worth noticing. It uses Ollama's **native** `/api/chat`, not
`/api/generate` and not the `/v1` OpenAI-compatible shim Ollama also
serves — so `temperature` travels inside `options`, which is Ollama's
spelling and nobody else's. And there is no `Authorization` header on
this path at all.

### OpenAI-compatible

```
POST {base_url}/v1/chat/completions
Content-Type: application/json
Authorization: Bearer <key>        # only when a key is set

{
  "model": "<model>",
  "messages": [{"role": "user", "content": "<prompt>"}],
  "temperature": 0.9,
  "reasoning_effort": "none"       # omitted when --reasoning-effort off
}
```

Answer is `choices[0].message.content`.

### What is *not* sent

No `system` message — the instruction and the document ride in a single
user turn, separated by a literal `\n\n---\n`. No `stream`, so both
backends are one blocking response. No `seed`, no `top_p`, no
`max_tokens`, no `stop`.

That last group is the first real delta for us. Spec §4.4 wants
`seed = base_seed + round*c` so that candidate *c* of round *r* is
reproducible; upstream re-rolls the dice and cannot replay a run. Spec
§4.1 wants `stream: true` with an SSE parser so the GPUI side can show
tokens arriving. Both are additions, not corrections — upstream simply
never needed them.

### `reasoning_effort`, and why it is a knob and not a constant

Default `"none"`; `"off"` omits the field entirely. Upstream's changelog
records the measurement that produced it: on a one-line rewrite,
`deepseek-v4-flash` spent **9,894 completion tokens on chain-of-thought
versus 12 without it**. A reasoning model pointed at a paraphrase task
burns minutes and money thinking about a job that has no reasoning in
it. Spec §4.1 already lists `reasoning_effort` as a separate knob with
default `none`; this is the number behind that decision.

Note the two spellings are not interchangeable: `"none"` is a *value*
some servers accept and others reject, so `"off"` — omit the key — has to
exist as a distinct choice. A single boolean would have stranded one
class of endpoint.

---

## 2. The security rules on the transport

Three, and all three are already in our spec. Upstream is the evidence
that they earn their place.

**Redirects are refused outright.** `_NoRedirect` subclasses
`urllib.request.HTTPRedirectHandler` and raises on any 3xx instead of
following it. The reason is precise: urllib's default handler re-sends
request headers on 301/302/303, which would forward the `Authorization`
header to a host that never passed the allowlist. A 3xx surfaces as an
error rather than as a silently-relocated request.

**Non-loopback is default-deny.** `_check_remote` rejects any scheme that
is not `http`/`https`, then compares the hostname against a literal set
`{"localhost", "127.0.0.1", "::1"}`. Anything else needs
`--allow-remote` or `WATERMARKS_REWRITE_ALLOW_REMOTE=1`, and even then
prints `content will leave this machine` to stderr.

The check is a *string* comparison, which fails conservatively: `127.0.0.2`
and a DNS name that resolves to loopback are both treated as remote and
denied. It does not defend against a permitted remote host later
resolving somewhere else, but a rewrite endpoint the user typed in is not
that threat model. Our equivalent is `allow_remote = true` in config plus
the UI banner naming the host (§4.1) — the banner is the part upstream
can only do with a stderr line.

**The API key never touches argv.** There is deliberately no `--api-key`
flag; a comment says why — keys on argv are visible in `ps` and shell
history. `WATERMARKS_REWRITE_API_KEY` only. `--gumbel-key` exists but its
help text says the same thing and steers to the env var. Our spec puts
the key in the OS keychain via `keyring`, never in TOML, which is
strictly stronger.

---

## 3. The prompts

Eight templates live in the `PROMPTS` dict; two more tactics build their
prompt inline; two clauses are appended. All of them are **English
only**, which is a defect for us and is discussed in §7.

Every one of them ends with the same three moves: preserve facts,
numbers, names and technical identifiers; do not add or remove claims;
output only the rewritten text. That triple is the contract that makes
the output substitutable for the input, and it is the part to keep
verbatim.

The document is appended as `\n\n---\n{TEXT}`.

### `paraphrase` — the default

> Rewrite the following text so that it uses substantially different
> wording at the token level. Change clause order, connectors, and
> transition words; vary sentence boundaries and length; and replace both
> content words and function words where meaning allows. Preserve all
> facts, numbers, names, and technical identifiers. Do not add or remove
> claims. Output only the rewritten text.

"At the token level" is doing the work. A token-sampling watermark lives
in the choice of each next token, so an instruction to change *meaningful*
wording is not enough — function words carry the mark too, and this
prompt says so explicitly.

### `humanize`

The version in `rewrite_text.py` is considerably longer than the one in
`skills/remove-ai-marks/SKILL.md`; **the code is the current one** and the
SKILL.md copy has drifted. In full:

> Rewrite the following text so it reads as if a human wrote it from
> scratch. Vary sentence rhythm and length unevenly — mix short and long
> sentences instead of a steady mid-length cadence — and merge or split
> paragraphs where a human would. Use plain, concrete wording and simple
> verbs (is/are/has); prefer active voice. Cut promotional language and
> inflated significance ("stands as a testament", "pivotal", "vibrant",
> "a rich tapestry"), superficial present-participle analyses
> ("reflecting", "showcasing", "underscoring"), vague attributions
> ("experts argue"), rule-of-three listing, filler ("in order to", "it is
> important to note"), hedging, and formulaic positive conclusions. Avoid
> AI vocabulary ("additionally", "delve", "crucial", "foster",
> "leverage", "utilize", "interplay", and abstract "landscape"). Do not
> add em dashes, bold text, emojis, or curly quotes. Preserve all facts,
> numbers, names, and technical identifiers. Do not add or remove claims.
> Output only the rewritten text.

The banned-vocabulary list is sourced from Wikipedia's "Signs of AI
writing". Two honesty notes for us:

* This is a **stylometric** pass, not a watermark pass. It defeats a human
  reader's pattern-match and a stylometry classifier. It does nothing in
  particular to a keyed token-sampling watermark beyond the token churn
  any rewrite produces. Filing it under *best-effort* alongside
  `paraphrase` is correct; implying it removes anything *verifiable* is
  not.
* The list is English-specific and does not translate. A Russian
  `humanize` prompt needs its own list, or it is a no-op instruction that
  still costs a full generation.

### `code`

> Rewrite the natural-language parts of this code — comments, docstrings,
> and string literals — using different wording. Rename local variables,
> function parameters, and private helper names to semantically
> equivalent names. Preserve program behavior, public API names, and all
> values that affect output. Output only the rewritten code.

Spec §4.3 already requires explicit UI confirmation for this tactic,
because renaming identifiers is behaviour-adjacent. Upstream's SKILL.md
says the same in prose and additionally recommends running a formatter
(`prettier`/`black`/`gofmt`) plus Layer A *instead*, which is the cheaper
and safer first move and belongs in our UI copy.

### `backtranslate`

Two templates for the two-call path, plus a combined single-call variant
used for `print-prompt` and one-shot backends:

```
Translate the following text to {LANG}. Output only the translation.
Translate the following text to {ORIGINAL_LANG}. Preserve meaning; use natural phrasing. Output only the translation.
```

Combined: `Translate the text to {LANG}, then translate that result back
to {ORIGINAL_LANG}. Preserve all facts, numbers, and names. Output only
the final {ORIGINAL_LANG} text.`

Defaults `--lang French`, `--original-lang English`. Our spec picks the
pivot by document language instead (`de` for RU, `ru` for EN), which is
the better default — a pivot too close to the source barely moves the
token stream.

Upstream's own benchmark rows for backtranslate were the reason the no-op
guard in §5 exists: the tactic can return something nearly identical to
the input and be scored as a successful clean.

### `structural`

Two steps — outline, then regenerate:

```
Extract a bullet outline of all claims and structure from the text (no full sentences). Output only the outline.
Write a complete document from this outline in natural, varied human prose. Avoid formulaic transitions. Do not omit any bullet. Output only the document.
```

Combined single-call variant exists for the same reason as backtranslate.
This is the most destructive tactic and the most effective one: the
second call generates fresh tokens from a bullet list, so essentially
none of the original sampling survives. It is also where "do not omit any
bullet" is the only thing standing between the user and silent content
loss — which is exactly what our guards (§3.3 of the spec) are for.

### `chunk`

> Rewrite only this fragment to change a modest fraction of its tokens.
> At low intensity keep the sentence structure, word order, and every
> token that can stay, changing only function words and a few
> non-essential content words. Preserve all facts, numbers, names, and
> technical identifiers. Do not add or remove claims. Output only the
> rewritten fragment.

`_split_units` splits after sentence punctuation or on any newline run,
and each fragment is sent as its **own request**. The stated reason is
the interesting one: *a fresh context per fragment ⇒ new per-token
watermark keys*. A keyed scheme hashes a window of preceding tokens, so
cutting the context is itself an attack, independent of what the model
writes back.

The separator (the whitespace run that followed the unit) is kept and
reassembled, so paragraph layout survives — unless `--chunk-shuffle` is
passed, which shuffles fragments before rewriting and drops separators.
That flag destroys document coherence and upstream marks it opt-in.
**We should not ship it**: it is a benchmark instrument, not a product
feature, and a user who reaches it has been handed a scrambled document.

### `mlm` — the local one

Not a prompt at all. `_mlm_infill` masks a `level` fraction of content
words and fills them with a `roberta-large` fill-mask pipeline from
`transformers`. Content words only: `t.isalpha()`, longer than 3
characters, not in a ~70-word function-word stoplist, not capitalised.
Masks are spread evenly (`step = len(content) / k`) rather than chosen at
random, and input longer than roberta's 512-token positional limit is
split into chunks that are infilled separately.

Its value is that it is **non-autoregressive** — the output is the
original token stream with holes filled, so it churns tokens without a
second LLM re-stamping the text. That is the cheapest possible answer to
the non-origin problem.

**It is out of scope for Wipemark.** Spec §0: no Python, no torch. There
is no phase-1 substitute, and pretending `paraphrase@0.8` covers the same
ground would be wrong — it is a different mechanism. Record it as a
deliberate omission, not an oversight.

### The two appended clauses

`_intensity_clause(level)` is appended when `--rewrite-level` is set
alongside a named tactic (except `code`, which is not naturally
intensity-modulated):

> Modulate this rewrite so roughly a fraction {LEVEL:.2f} of tokens
> change: 0 would keep the wording unchanged, 1 rewrites everything. At
> low intensity keep the sentence structure, word order, and every token
> that can stay, changing only function words and a few non-essential
> content words; at high intensity change wording substantially at the
> token level. Preserve all facts, numbers, names, and technical
> identifiers. Do not add or remove claims.

`_style_clause(style)` is appended when `--style` is given:

> Apply this writing style throughout the rewrite: {STYLE}. Keep the
> style subordinate to the content — preserve all facts, numbers, names,
> and technical identifiers, and do not add or remove claims.

Both docstrings say the same thing in the same words: **"a request, not a
contract"**. The level is not enforced; measured divergence is the real
outcome. That phrasing is the third shelf doing its job inside a
docstring, and it should survive into our config UI: the intensity slider
is a hint to a model, and the report says what actually changed.

There is also a standalone `level` template used when there is no tactic
at all — same content as the clause, wrapped as a complete prompt.

### Assembly

```
build_prompt(tactic, text, lang, original_lang, rewrite_level, style)
  base   = PROMPTS[tactic] or an inline template          # tactic prompt
  base  += "\n\n" + _intensity_clause(level)              # if level and tactic != "code"
  base  += "\n\n" + _style_clause(style)                  # if style
```

With `tactic=None` and a level set, `PROMPTS["level"]` replaces the base
outright.

---

## 4. The deterministic humanizer pass

`humanize_pass.py` runs over every candidate produced by the `humanize`
tactic, **before** evaluation, so the text that is scored is the text
that is returned. Its docstring draws the line precisely: this module
does "the mechanical, context-free subset … that is safe to apply without
reading the text", and everything needing judgment stays in the prompt.

| Transform | Detail |
| --- | --- |
| Straight quotes | `U+2018 U+2019` → `'`, `U+201C U+201D` → `"` |
| Em/en dash | `\s*[—–]\s*` → `", "` — **unless** both neighbours are digits (a numeric range like `2019 - 2020` survives) |
| Double hyphen | `--` → `", "` — **unless** it looks like a CLI flag (`--word` after a delimiter or string start) |
| Comma cleanup | `, ,` → `, ` and `word, .` → `word.` after the dash substitutions |
| Phrase collapses | 12 pairs: `in order to`→`to`, `due to the fact that`→`because`, `at this point in time`→`now`, `in the event that`→`if`, `has/have the ability to`→`can`, `it is important to note that`/`it is worth noting that`→`note that`, `delve/delves/delved/delving into`→`explore/explores/explored/exploring` |
| Word swaps | `utilize/utilizes/utilized/utilizing` → `use/uses/used/using` |

Capitalisation is carried across from the matched text
(`_capitalize_like`: all-caps stays all-caps, leading capital stays
capitalised).

**Why this matters to us.** These are exactly the transforms that belong
on the *verifiable* shelf and not on the best-effort one — they are
deterministic, countable, and they run without a model. Wipemark's
Layer A is Unicode scrubbing; this is a second deterministic pass over
*style* rather than over codepoints. The two carve-outs (numeric ranges,
CLI flags) are the kind of detail that only shows up after someone files
a bug, and they are free to copy.

One caution: the em-dash rule is a *content* edit, not a provenance one.
An author who writes em dashes on purpose will not thank us. It belongs
behind an explicit toggle, and our report has to name it as an edit we
made rather than a mark we removed.

---

## 5. The selection loop

```
for loop in 0..max_loops:                  # default 1  (WATERMARKS_REWRITE_LOOPS)
    for _ in 0..candidates:                # default 1  (WATERMARKS_REWRITE_CANDIDATES)
        cand = generate()                  # one backend call (or per-fragment calls for `chunk`)
        if tactic == humanize: cand = humanize_pass(cand)
        if layer_a_after:      cand = clean_text(cand)      # Layer A on the model's output
        divergence = 1 - jaccard(bigrams(original), bigrams(cand))
        evaluation = evaluator.detect(cand)  or  {"score": divergence}
        record(cand, evaluation)
    if any candidate passed: break
select_best()
```

Note that a round evaluates **all** its candidates before breaking, "so
the best is chosen, not the first to squeak under the threshold" — the
break is between rounds, not inside one.

### Evaluator priority

1. **keyed-Gumbel same-key replay** — when `--gumbel-key` /
   `WATERMARKS_GUMBEL_KEY` is set.
2. **MarkLLM same-config detection** — when `--markllm-scheme` is set.
3. **bigram-Jaccard lexical divergence** — the fallback, and it has **no
   pass/fail verdict**. Every attempt is generated and the most diverged
   one wins.

A slot is reserved above both for a vendor detector; upstream notes
Google retired SynthID-text detection on its API in Aug 2026, so the slot
is currently empty. Our spec has the same ordering (`keyed_gumbel` →
`divergence`, slot for `keyed_kgw`) — and the honesty caveat is identical
and must survive: a same-key or same-config detector proves nothing about
a vendor detector, and a negative result establishes nothing at all.
Third shelf, always.

`_safe_detect` wraps every detector call in a bare `except` — "a raising
detector never fails the rewrite". Right call: the evaluator is an
optional instrument, and a broken one must not cost the user their
rewrite.

### Passing, and the margin

`_candidate_pass(evaluation, target_margin)` returns
`(passed, margin, raw_margin)`:

* `is_watermarked` missing → `(None, None, None)`, fail-soft, no verdict.
* `is_watermarked is True` → `False`.
* Not watermarked, and `threshold - score >= target_margin` → `True`.
* Not watermarked but under the margin floor → `None`.

The subtlety worth stealing: some detectors report a **p-value**
threshold that does not live on the same scale as `score`, so
`threshold - score` is meaningless. A negative raw margin is therefore
read as "this threshold is not a score-scale cutoff" and the candidate is
treated as a clean pass with no margin gate, rather than being failed by
an arithmetic comparison that never applied.

### Choosing the winner

Among candidates that passed:

* `--select min-divergence` (default) — the one that **changed the
  least**. Content-preserving.
* `--select max-margin` — the largest margin. Robustness-first.
  Tie-broken by `(raw_margin, -p_value, -divergence)`; the raw margin is
  compared unrounded because the telemetry value rounds to four decimals
  and two candidates can collide there.

If nothing passed: the lowest watermark score, or — with no detector —
the most diverged candidate. `_select_candidate` also docks 0.15 from a
candidate whose length drifted beyond 2× or below 0.5× the original,
which is a cheap guard against a model that summarised instead of
rewriting.

`min-divergence` as the default is the right instinct and matches our
product: the user wants their document back, not a different document
that scores well.

### The no-op guard

```
noop = noop_lex_floor > 0 and lexical_divergence(input, output) < noop_lex_floor   # default 0.05
```

Emitted as `noop: true` in `--json-stats` with a stderr warning. The
comment names the failure it exists for: a benchmark counting a
near-verbatim output as "0% clear" — the misleading backtranslate row.

**Take this.** A rewrite that changed nothing is not a removal attempt,
and a report that files it as a completed Layer B pass is lying by
omission. This is a guard, and per CLAUDE.md guards get a RED-first test.

---

## 6. The strategy DSL and the service default

```
"paraphrase@0.8,mlm@0.2"
```

`parse_strategy` validates each `tactic@intensity`: the tactic must be one
of the seven, and intensity must be in **(0,1]** — zero is excluded
because zero is the unchanged original.

`apply_strategy` runs the steps **sequentially**, each feeding the next,
one generation per step, and — importantly — **no evaluation loop at
all**. It suits an operation that wants the rewrite regardless of a
removal verdict. Layer A runs once at the end, over the finished output,
not per step.

That is a second, simpler execution mode next to `rewrite()`, and the
service `/clean` endpoint uses it, not the loop. The default comes from
`config/clean_strategy.json`, and `/clean` on a text input **rejects with
400** when no strategy is available or a step's backend is unconfigured —
Layer B is a required step for text there, not an optional one.

For us this maps onto spec §4.3's `tactic_ladder` (default
`[paraphrase]`), with one structural difference: upstream's strategy is a
fixed pipeline, ours is an **escalation** ladder — the next tactic is
tried only when the round before it failed. Ours costs less on the common
case and more on the hard one, which is the right way round for an
interactive product.

---

## 7. What Wipemark takes, and what it must not

| Upstream | Wipemark | Why |
| --- | --- | --- |
| Prompt bodies (the "preserve facts / no claims / output only" triple) | **take verbatim** | Earned; the contract that makes output substitutable for input |
| Tactic set `paraphrase / humanize / backtranslate / structural / code` | **take** | Spec §4.3 already names all five |
| `chunk` per-fragment rewriting | **take the reason** | Fresh context ⇒ new per-token keys. Our chunker is paragraph-scoped with prev-context (§4.2), not per-sentence |
| `--chunk-shuffle` | **do not ship** | Destroys coherence; a benchmark instrument |
| `mlm` local infill | **out of scope** | No Python, no torch (spec §0). Record as a deliberate gap, not a substitute |
| Redirects refused, loopback default-deny, key never on argv | **take all three** | Already in spec §4.1; upstream is the proof |
| `reasoning_effort` default `none`, `off` to omit | **take** | 9,894 vs 12 completion tokens |
| Evaluator priority, fail-soft detectors | **take** | Matches spec §4.4 ordering |
| `min-divergence` default selection | **take** | The user wants their document back |
| Length-drift penalty, no-op guard | **take** | Cheap; both catch a "success" that isn't |
| Deterministic humanizer (quotes, dashes, phrase collapses) | **take, behind a toggle** | Deterministic and countable — but it is a content edit, and the report must say so |
| English-only prompt constants | **reject** | See below |
| Prompts as source constants | **reject** | Ours are config templates, editable in Settings (§4.3) |
| No `seed` / `stream` / `max_tokens` | **add all three** | Reproducible candidates; token streaming to the GPUI side |
| No protected spans | **add** | `⟦n⟧` placeholders for code/URLs/paths/numbers (§4.2). Upstream only *asks* the model to preserve identifiers |
| Non-origin rule in prose | **make blocking** | Ours is a blocking UI warning with `--force` in the CLI (§4.4) |

### The language problem

Every prompt is an English string literal, and `humanize`'s
banned-vocabulary list is English lexis. Point that at a Russian document
and two things go wrong: the model drifts into *translating* rather than
rewriting, and the entire stylometric half of the instruction is inert
while still being paid for.

Spec §4.3 already fixes this — prompt language follows document language,
detected by script ratio — and this is where the CLAUDE.md rule bites:
*"no_language_promises_more_than_the_product_does gates the catalogues,
and every item of not_established::ALL must have a translation in every
one of them."* A prompt catalogue that is complete in `en-US` and empty
in `ru-RU` is a product that quietly does less in Russian while the
report says the same thing in both. The prompt templates are read by a
model, not by a person, so by the i18n rule they are **not** catalogue
messages — but they still need per-language defaults, and the gate that
keeps them honest has to be ours to write.

### The honesty line

Upstream never claims removal. `rewrite()` attaches this note to every
result:

> Layer B is best-effort against statistical token-sampling watermarks;
> cannot certify removal against a vendor detector.

and when a MarkLLM scheme was configured it adds the cross-model hygiene
warning: *rewrite with a model that is neither the generator nor itself
watermarked, or the rewritten text can be re-stamped*. Its SKILL.md
"Report" step requires stating what was **verifiably** removed, what
Layer B did **best-effort**, and what is out of scope — the same three
shelves, arrived at independently.

That convergence is worth noting rather than glossing: the third shelf is
not our stylistic preference, it is what anyone who builds this honestly
ends up with.

---

## 8. Configuration surface (for the config-key mapping)

Upstream's env vars, as a checklist against our config keys. Ours live in
the `settings` table of `wipemark.db`, one row per key (CLAUDE.md), so
these are names to map, not a file to copy.

| Upstream env | Meaning | Wipemark |
| --- | --- | --- |
| `WATERMARKS_REWRITE_BACKEND` | `print-prompt` / `ollama` / `openai-compatible` | Engine choice — `OpenAiCompatEngine` / `LlamaEngine` / `FakeEngine` |
| `WATERMARKS_REWRITE_BASE_URL` | API base, default `http://127.0.0.1:11434` | `base_url` + editable preset list (Ollama, LM Studio, mnemoria, OpenAI, OpenRouter) |
| `WATERMARKS_REWRITE_MODEL` | Model name | `model_id` |
| `WATERMARKS_REWRITE_API_KEY` | Env only, never argv | OS keychain via `keyring` |
| `WATERMARKS_REWRITE_ALLOW_REMOTE` | `1` to permit non-loopback | `allow_remote` + the host banner |
| `WATERMARKS_REWRITE_REASONING_EFFORT` | `none` (default) / low / medium / high / `off` | Same knob, same default |
| `WATERMARKS_REWRITE_CANDIDATES` | Variants per round, default 1 | `candidates`, default **2** (§4.4) |
| `WATERMARKS_REWRITE_LOOPS` | Max rounds, default 1 | `max_rounds`, default **2** (§4.4) |
| `WATERMARKS_GUMBEL_KEY` | Keyed-Gumbel replay key | Same; keychain, never logged |
| `WATERMARKS_CLEAN_STRATEGY_FILE` | Path to the strategy config | `tactic_ladder` in settings |
| — | `--temperature`, default 0.9 | Same default (§4.4) |
| — | `--timeout`, default 120 s | connect/read timeouts, separately |
| — | `--target-margin`, default 0.0 | Not in the spec; consider for the evaluator |
| — | `--noop-lex-floor`, default 0.05 | Take as a guard |

The endpoint half of that table is implemented — provider, base URL,
model, key, `allow_remote`, `reasoning_effort`, temperature and timeout
are the Engine section of the Settings window as of E6/S6.3, and
`docs/architecture/engine-settings.md` is how. The pipeline half —
candidates, rounds, the tactic ladder, the Gumbel key — is still ahead
of us, and belongs to E4 rather than to a row on that page.

Our defaults for candidates and rounds are 2×2 where upstream ships 1×1.
That is four generations per chunk against one, and spec §4.4 requires
the cost to be shown *before* the run (`chunks × candidates × max_rounds`,
with a token and time estimate from the warmup measurement). Upstream has
no cost preview because at 1×1 there is nothing to warn about.

---

## 9. Exit codes — a contract we already inherited

Upstream's audit CLIs use `0` no actionable findings, `1` actionable
findings, `2` usage/refusal error, `3` **partial scan** — "treat as
inconclusive; the audit was incomplete, not clean."

`apps/wipemark-cli/src/main.rs` pins exactly those four as
`Exit::{Clean, Findings, Usage, Partial}`, with
`exit_codes_are_pinned` as the gate. The lineage is direct, and the
meaning of `3` is the interesting one: it is the third shelf expressed as
an integer. A hook author who treats `3` as success has concluded "clean"
from "we did not finish looking".

---

## Sources

All paths relative to the upstream clone in `tmp/watermarks-remover/`
(gitignored; re-clone with
`git clone --depth 1 https://github.com/guillaumemeyer/watermarks-remover tmp/watermarks-remover`).

| What | Where |
| --- | --- |
| Prompts, backends, selection loop, strategy DSL | `service/scripts/rewrite_text.py` |
| Deterministic humanizer | `service/scripts/humanize_pass.py` |
| `/clean` Layer B wiring, 400 on unconfigured backend | `service/scripts/server.py` (`_apply_layer_b`, `_load_default_strategy`) |
| Default strategy | `config/clean_strategy.json` |
| Prompt copies (drifted — code wins) and the report contract | `skills/remove-ai-marks/SKILL.md` §4, §5 |
| Env var table, benchmark flags, honesty caveats | `README.md` |
| Benchmark methodology | `benchmarks/README.md`, `docs/synthid-text-benchmark.md` |

See also [hooks.md](hooks.md) for this repository's own hook inventory,
which includes upstream's plugin and pre-commit hooks as prior art.
