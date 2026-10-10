# Model manifest

`models.v1.json` is the catalogue of downloadable weights. It is
`include_str!`-ed into the binary by `wipemark-models` and may one day be
refreshed from a remote URL — but only when the remote copy carries a
valid Ed25519 signature from the same key that signs licences. An
unsigned manifest is rejected, not merged (spec §5.1); signature checking
lands with epic E9.

Adding a model is a **JSON edit**, never a code change. Every rule the
parser enforces is in `crates/wipemark-models/src/manifest.rs`, and every
one of them exists because breaking it produces a *silent* wrong answer
rather than an error — see the note on `Manifest::parse`.

## Entry shape

```json
{
  "id": "qwen3-4b-instruct-2507-ud-q4",
  "display": "Qwen3 4B Instruct 2507 (UD-Q4_K_XL)",
  "roles": ["rewrite"],
  "format": "gguf",
  "status": "stable",
  "files": [
    {
      "url": "hf://unsloth/Qwen3-4B-Instruct-2507-GGUF@a06e946bb6b655725eafa393f4a9745d460374c9/Qwen3-4B-Instruct-2507-UD-Q4_K_XL.gguf",
      "sha256": "4bbe1f2f8ebe69fad3be8e15d69f220b06448a9dd26f82d7d81cce88ebfc39fd",
      "size_bytes": 2546340960
    }
  ],
  "mem": { "min_ram_mb": 4608, "min_vram_mb": 4608 },
  "ctx_default": 8192,
  "quant": "UD-Q4_K_XL",
  "quality_tier": 5,
  "license": "apache-2.0",
  "langs": ["multi"],
  "vendor": "open-llm",
  "notes": "…"
}
```

### `id` — a plain name, and never `user-…`

A plain directory name (no `/`, no `..`, no leading dot). It never begins
with `user-`: that prefix names a model the person added from a file
(E8-1, D400), and `models.rewrite` must name exactly one of the two —
`Manifest::parse` refuses it (`a_catalogue_id_never_reads_as_a_model_the_person_added`).

### `roles` — the purpose classification

A model serves a *purpose*, and some models serve two, so this is a list
rather than a single `task`. The six roles are `rewrite`, `detect`,
`fill-mask`, `embed`, `pixel` and `draft`; each one traces to something
this repository already describes, and `rewrite` and `draft` ship entries
in v1 — a draft only beside its target (below).
Adding the first entry for another role is caught by
`a_role_the_catalogue_serves_has_a_row` — a model nobody can select is a
download with no purpose.

### `draft_for` — a draft is tied to the one model it drafts for

A speculative draft (E2-dflash2, D484) serves `draft` and nothing else, and
names its target's id in `draft_for`:

```json
  "roles": ["draft"],
  "draft_for": "qwen3.8-27b-ud-iq3s",
```

Nobody chooses a draft: it decodes beside its target when the Engine page's
**Faster decoding** row is on and it is downloaded, and not at all
otherwise. So it is in no selector, never recommended and never on duty —
and the parser holds the shape: a draft names a target the catalogue has,
which rewrites; an entry that names a target is a draft and nothing else;
and a model has one draft at most (`a_draft_is_tied_to_one_rewriter`). Its
`mem` is what loading it beside its target **adds** — its weights, its
cache, and whatever the target keeps for it (recurrent-state snapshots,
the layer inputs the draft reads) — which `host::fit` adds to the
target's.

### `url` — pinned, always

`hf://<owner>/<repo>@<commit>/<path>` resolves to Hugging Face's
`resolve` endpoint. The `@<commit>` is **not optional** and must be a
40-character sha: a manifest that pinned a sha256 to a moving branch goes
stale the first time the repository is updated, and then looks like a
corrupt download rather than an out-of-date catalogue.

A plain `https://` URL is also accepted. `http://` is accepted **only**
for loopback — a local mirror or a test server, where there is no wire to
protect and the sha256 is doing the work either way. A URL carrying
userinfo (`https://user:token@host/…`) is refused outright rather than
stripped: that is a credential written into a data file.

### `sha256` and `size_bytes` — read, never estimated

Both come off Hugging Face, not off a guess:

```sh
curl -s "https://huggingface.co/api/models/<owner>/<repo>/tree/main?recursive=1" \
  | python3 -c "import json,sys; [print(f['path'], f['size'], (f.get('lfs') or {}).get('oid')) for f in json.load(sys.stdin) if f['type']=='file']"
```

`lfs.oid` **is** the file's sha256. Confirm it against the `resolve`
endpoint before committing — the `x-linked-etag` on the 302 is the same
value, and the 200 that follows is the CDN's own etag and is *not*:

```sh
curl -sIL "https://huggingface.co/<owner>/<repo>/resolve/<commit>/<file>" \
  | grep -iE "x-linked-(etag|size)|accept-ranges"
```

A file above 10 MB with no `sha256` is refused by the parser. Below that
a tokenizer or a config sidecar may go unhashed.

### `mem` — conservative estimates, and said so

`min_ram_mb` is weights plus a KV cache at `ctx_default` plus a fixed
overhead. It is an **estimate**, not a measurement of this model running
on this machine, and `wipemark_models::host::fit` treats it as one: it
never reports that a model *will* work, only that there is or is not room
for it. Show your arithmetic in `notes`.

## Which models belong here

Open-weight only: a catalogue entry is a file this product downloads
and runs on the user's machine, which a commercial vendor's model is not.
`vendor: "open-llm"` is a statement about how the model is *served* —
locally, under the user's control — and the entry's `notes` is where a
family's provenance belongs when it needs saying out loud. (The spec's
"non-origin rule", which once gave this section a second reason, was
dropped by D62: nothing refuses a vendor.)

Two more rules the shipped entries follow:

* **Ungated repositories only.** The downloader has no Hugging Face
  account and sends no `Authorization` header, so a repository gated
  behind an accepted licence cannot be fetched at all.
  `google/gemma-3-12b-it-qat-q4_0-gguf` is `gated: manual`; the unsloth
  mirror of the same weights is not, which is why it is the one listed.
* **Non-thinking builds for `rewrite`.** A hybrid model that reasons
  before it answers burns tokens nobody reads —
  `docs/sdd/layer-b-rewrite-reference.md` records 9,894 completion tokens
  against 12 for the same rewrite. Prefer an Instruct build.

The starting list in the spec: Qwen3 8B and 4B, Gemma 3 12B/4B,
Llama 3.x 8B, Mistral Nemo 12B. Two of them ship today, chosen so that a
12 GB machine and a workstation are each offered something rather than
the same thing with a warning beside it — see
`docs/architecture/model-downloads.md`.
