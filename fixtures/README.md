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
          These are the ones that must SURVIVE a clean pass.
image/    PNG/JPEG/WebP/TIFF carrying C2PA manifests, XMP
          DigitalSourceType, Stable Diffusion `parameters` blocks.
          Phase 2 (E11).
```

Two rules for anything added here:

1. **Byte-exact.** Never let an editor normalise line endings or strip a
   trailing zero-width character — that is the fixture.
2. **Every fixture has a claim.** A file whose expected outcome is not
   asserted anywhere is decoration. Fixtures in `text/` that exist to be
   *preserved* matter more than the ones that exist to be cleaned: they
   are what the mutation checks in `wipemark-core` fail against.
