# SDD

Spec-driven design notes: what a spec decided, what an external reference
actually does, and what an audit of this tree found. Durable enough to
outlive a Watchword entry, narrower than `architecture/`.

| document | what it is |
|---|---|
| [layer-b-rewrite-reference.md](layer-b-rewrite-reference.md) | what `guillaumemeyer/watermarks-remover` sends to an Ollama or OpenAI-compatible endpoint — every prompt verbatim, the wire format, the selection loop — and which parts Wipemark takes |
| [line-decorations.md](line-decorations.md) | the `LineDecorationProvider` patch our gpui-component fork carries: what it paints in the Compare window (screenshots), the API, who uses it, where the fork lives since it moved to `GigLaboCom`, what upstream has instead, and what any replacement must do |
| [visible-marks.md](visible-marks.md) | visible watermarks on AI pictures: GeminiWatermarkTool read at `7c6a99f` and measured, what every vendor stamps (Gemini, OpenAI, Grok, …) with sources, what invisible marks remain, and the vendor-neutral architecture of epic E12 — profiles as data, two proofs before a pixel changes, one writer |
| [hooks.md](hooks.md) | every hook in this repository across five unrelated meanings of the word, and the ones that deliberately do not exist |

## How these differ from the neighbours

`architecture/` describes **what this repository does**: the icon
pipeline, the i18n contract, the log. Read it to work on the code.

`docs/sdd/` describes **why it will do what it is going to do**: a
reference implementation read out at a fixed commit, an audit taken on a
date, a decision and the measurement behind it. Read it before writing an
epic.

`ssd-docs/` is gitignored and holds the Watchword specs themselves. The
canonical copy of a spec is its Watchword key, never a file here — this
repository was built from FILE
`heretic-unmark-overview-decomposition-2026-09-07` (ttl 0).

## Conventions

* **Date and pin every survey.** An external project read at "some point"
  is not a source. Say the commit or the version, and where the clone
  was.
* **Reference clones live in `tmp/`**, which is gitignored. Nothing from
  one is vendored — vendoring goes through `.gitmodules` so the rev is
  pinned and the licence is recorded in `NOTICE`.
* **Say what we reject, not only what we take.** A document that lists
  only the good ideas leaves the next reader to rediscover the bad ones.
* **The third shelf applies here too.** Nothing in this directory says
  "undetectable"; a reference project's claim is quoted as its claim.
