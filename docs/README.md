# Docs

| document | what it is |
|---|---|
| [plan/README.md](plan/README.md) | **the plan of record**: where the project stands, every remaining epic in order with what it is built from, the decisions taken beyond the specs, the owner's open questions — and the E1 series of self-sufficient implementer documents (`plan/E1-1` … `plan/E1-7`) |
| [architecture/skeleton.md](architecture/skeleton.md) | what epic E0 built, the decisions it had to make, and what is deliberately absent |
| [architecture/layer-a.md](architecture/layer-a.md) | Layer A: the UCD 18.0.0 tables, what the classifier finds and what it keeps, the scrubber, NFKC, homoglyphs, the guards, the report and its JSON, and the two surfaces that call it |
| [architecture/icons.md](architecture/icons.md) | the typed icon set, the `currentColor` contract, and the Font Awesome Free pull-and-promote workflow |
| [architecture/i18n.md](architecture/i18n.md) | the message catalogue, the generated `Message` enum, and the rule that only applications localize |
| [architecture/engine-settings.md](architecture/engine-settings.md) | the Layer B endpoint settings, and why the API key is the one preference that is not a row in the database |
| [architecture/who-rewrites.md](architecture/who-rewrites.md) | which of an endpoint and this machine serves a role, why there is no fallback between them, and how a decision becomes an engine |
| [architecture/remote-engine.md](architecture/remote-engine.md) | the endpoint over HTTP: the two wire formats as sent, the transport rules and where each is enforced, the error table, threads and cancel, and the live check against llama.cpp's own server |
| [architecture/local-engine.md](architecture/local-engine.md) | the local engine: llama.cpp copied in at a named commit and pinned, the two features and which gate builds which, where the backends come from, why a cancel waits for the worker, the memory refusal, and how to run the native gates |
| [architecture/model-downloads.md](architecture/model-downloads.md) | the model catalogue, the resumable verifying downloader, and how the machine's memory decides what is offered |
| [architecture/setup.md](architecture/setup.md) | the first-launch walk-through: what it recommends from the machine's memory, what it writes and when, and what it took from mnemoria-lvkb |
| [architecture/window-placement.md](architecture/window-placement.md) | which screen a window opens on and where on it, the six zones a screen divides into, and how the displays are watched |
| [architecture/drag-and-drop.md](architecture/drag-and-drop.md) | what a window accepts when something is dragged onto it, why GPUI only offers files, and how the bytes and the name of a thing are arbitrated |
| [architecture/queue.md](architecture/queue.md) | the main window's table — lazy-shot's table for a product that takes text as well as pictures: the preview beside every row, the hover card, the filter bar, the sort and the paginator, and what each keeps or changes from lazy-shot |
| [architecture/compare.md](architecture/compare.md) | the Compare window — the result beside its original, the line diff under the marks and the word diff within a changed line, the Settings page that chooses the grain, and the result editor whose toolbar is the library's own actions taken by a different road |
| [architecture/pipeline.md](architecture/pipeline.md) | the pipeline, a section per E4 step — so far preparing the text: what of a Markdown, HTML or plain document is prose, the protected spans and their placeholders, chunks by paragraph and list, the context, the language, the calibrated token estimate, and the way back byte for byte |
| [architecture/cli.md](architecture/cli.md) | the command line: every command's exit codes and streams, `clean --in-place` and the order that protects the original, `audit` with its JSON and SARIF, and `models` without the window |
| [architecture/retention.md](architecture/retention.md) | where a result goes, what happens to the file it came from, what the product keeps of its own and for how long — the survey behind the defaults, and why a Markdown or HTML paste is kept no differently |
| [architecture/logging.md](architecture/logging.md) | the rotating log, the panic hook, and what must never reach a log line |
| [architecture/hotkeys.md](architecture/hotkeys.md) | system-wide shortcuts: the recorder, why it intercepts keystrokes before the bindings, the row format, and the registration |
| [sdd/layer-b-rewrite-reference.md](sdd/layer-b-rewrite-reference.md) | the upstream Layer B rewrite reference — every prompt verbatim, the Ollama / OpenAI-compatible wire format, and what we take from it |
| [sdd/hooks.md](sdd/hooks.md) | every hook in this repository across five meanings of the word, and the ones that deliberately do not exist |
| [../CONTRIBUTING.md](../CONTRIBUTING.md) | submodule setup, the pin script, the gates |
| [../manifests/README.md](../manifests/README.md) | model manifest shape and why it ships empty |

Design notes that survey an external reference or record an audit go
in [sdd/](sdd/README.md); `architecture/` is for how this repository
works today.

Specs live in **Watchword**, not here. The overview and decomposition
this repo was built from is the FILE entry
`heretic-unmark-overview-decomposition-2026-09-07` (ttl 0). Keep the
working copy in `ssd-docs/`, which is gitignored; anything durable that
outlives a spec belongs in this directory instead.
