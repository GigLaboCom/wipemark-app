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
image/    PNG/JPEG/WebP/TIFF carrying C2PA manifests, XMP
          DigitalSourceType, Stable Diffusion `parameters` blocks.
          Phase 2 (E11).
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
