# Model manifest

`models.v1.json` is the catalogue of downloadable weights. It is
`include_str!`-ed into the binary by `wipemark-models` and may be
refreshed from a remote URL — but only when the remote copy carries a
valid Ed25519 signature from the same key that signs licences. An
unsigned manifest is rejected, not merged (spec §5.1).

It ships **empty** at the skeleton stage on purpose. Every entry carries
a `sha256` and a `size_bytes` that the downloader enforces, and inventing
those values would produce a catalogue that fails verification on first
use. Epic **E3 / S3.1** fills it in, after checking the current file
names on Hugging Face.

## Entry shape

```json
{
  "id": "qwen3-8b-instruct-q4_k_m",
  "display": "Qwen3 8B Instruct (Q4_K_M)",
  "task": "rewrite",
  "format": "gguf",
  "source": {
    "hf_repo": "Qwen/Qwen3-8B-Instruct-GGUF",
    "file": "Qwen3-8B-Instruct-Q4_K_M.gguf",
    "revision": "<commit sha, never a branch name>"
  },
  "sha256": "…",
  "size_bytes": 4920000000,
  "quant": "Q4_K_M",
  "ctx_default": 8192,
  "min_ram_gb": 7.0,
  "license": "apache-2.0",
  "langs": ["ru", "en", "multi"],
  "vendor": "open-llm"
}
```

## Which models belong here

Open-weight only, and never a model from Claude / Gemini / OpenAI — the
non-origin rule (spec §4.4) means a document suspected of carrying a
vendor's mark must not be rewritten by that same vendor, and the shipped
catalogue should make the safe choice the easy one. The starting list in
the spec: Qwen3 8B and 4B, Gemma 3 12B/4B, Llama 3.x 8B, Mistral Nemo
12B.

`revision` pins a commit, never `main`: a manifest whose sha256 no longer
matches the file at the other end is worse than no manifest.
