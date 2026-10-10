# Fixtures

Test inputs shared across crates, kept out of `src/` because several of
them are byte-exact and must not be reformatted by an editor or a
formatter.

Layout, filled in as the epics land:

```
text/     one file per Unicode class from spec §3.1, plus the
          false-positive set: emoji ZWJ sequences, VS15/VS16, CJK
          ideographic variation sequences, Arabic U+061C and
          U+0600–0605, Mongolian FVS, Egyptian quadrat controls.
          Files named `<class id>.txt` are cleaned — one per class;
          files named `survive-*.txt` must come out byte-identical.
          Every file's claim is in
          `crates/wipemark-core/tests/fixtures.rs`, and
          `every_fixture_is_asserted` fails for a file that has none.
image/    real PNG/JPEG/WebP files from the C2PA project's public
          fixtures (E11-1): a C2PA manifest in JPEG APP11, XMP
          provenance references, a camera WebP. Origin, licence and
          each file's claim are in `image/README.md`; everything
          injected (SD `parameters`, ComfyUI, XMP DigitalSourceType…)
          is built in `crates/wipemark-image/tests/support/`.
clean-parity/
          the windows' clean against the CLI's (E7, W9): six small
          inputs — two Markdown files, a UTF-16 text with a byte order
          mark, a soft hyphen, a homoglyph, a TIFF head — and
          `table.tsv`, which also names six of `image/`'s pictures. The
          table is the claim: the CLI's exit and whether it writes, and
          whether a window writes, per input; `apps/wipemark-cli/tests/
          parity.rs` and the app's `clean::tests` both read it.
marks/    mark catalogues the developer tools read through
          `--catalogue` (E12-R12 stage 4b): `synthetic-wordmark/` is a
          stand-in profile, `fixture-wordmark` — a text-like 72 × 24
          opacity map of straight strokes, written by
          `scripts/bench/wordmark.py` (no font file), pinned by sha256
          in its own `marks.json`, with a `degradations.json` in the
          bench's format. A fixture, not a vendor's mark: no figure
          about any vendor is read off it. The claims are the tests of
          `crates/wipemark-picture/examples/recon_bench.rs` and
          `measure_clean.rs` that read it (gen → run end to end for a
          profile that is not Gemini's; a non-square map measured at
          its own shape).
```

`text/keep-*.txt` — the survival set (E1-2): one file per context rule
in `docs/architecture/layer-a.md` › Classes and context. Every
finding-capable code point in these files is kept by context;
`wipemark_core::context`'s `every_keep_fixture_survives_whole` asserts
each file's byte length and hit count. `keep-script-format-controls.txt`
and `keep-leading-bom.txt` carry no finding at all. `wipemark-core`'s
`tests/fixtures.rs` holds them to the same survival and idempotence
claims through `clean`.

Two rules for anything added here:

1. **Byte-exact.** Never let an editor normalise line endings or strip a
   trailing zero-width character — that is the fixture.
2. **Every fixture has a claim.** A file whose expected outcome is not
   asserted anywhere is decoration. Fixtures in `text/` that exist to be
   *preserved* matter more than the ones that exist to be cleaned: they
   are what the mutation checks in `wipemark-core` fail against.
