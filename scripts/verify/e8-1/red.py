#!/usr/bin/env python3
"""red.py — the host verification's fixes to E8-1, each seen red once.

What it is for
  The host verification of E8-1 (models the person adds), asked by the
  coordinator for the owner, 2026-10-08. CLAUDE.md: "delete the protection
  and watch it go red". It found two Medium defects and fixed them on
  `integrate/e8-user-models`:
    * overflow — a GGUF header's cache shape at its largest numbers overflowed
      the memory estimate (a panic in a debug build, a wrapped figure that
      "fits" in a release one): `wipemark_models::gguf::KvShape::
      bytes_per_token`, `wipemark_models::user::estimate`/`total_mb`, and
      `wipemark_llama`'s `elements_per_token`/`kv_bytes_per_token`/
      `kv_cache_mb`/`MemEstimate::total_mb` made saturating;
    * taken — an add derived its id from the rows it had parsed (the window:
      the list the page holds; the CLI: the rows read before a minutes-long
      hash), so a row this build cannot read, or one added meanwhile, was
      written over: both now ask the table for the id itself.
  This is a record of the check, not a table to re-run every round.

What it does
  For each check: replaces the fixed text in its source with the text before
  the fix, runs the named tests, records RED (a test failed), GREEN (all
  passed — the test does not guard the fix) or BROKEN (no test ran / build
  failed), and puts the source back byte for byte whatever happened.

How to run
  From the repository root, with the environment the gates use (on the
  owner's host: LIBRARY_PATH pointing at a directory holding a
  libxkbcommon-x11.so symlink):
    python3 -I scripts/verify/e8-1/red.py

What it needs
  python3 (standard library), cargo.

What its output means
  One line per check: `<name>: RED|GREEN|BROKEN <tests>`. Every line should
  be RED. The sources are restored after each check.
"""
import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[3]

CHECKS = [
    (
        "overflow-models",
        [
            (
                "crates/wipemark-models/src/gguf.rs",
                """        u64::from(self.layers)
            .saturating_mul(u64::from(self.heads_kv))
            .saturating_mul(u64::from(self.key_length) + u64::from(self.value_length))
            .saturating_mul(2)""",
                """        u64::from(self.layers)
            * u64::from(self.heads_kv)
            * (u64::from(self.key_length) + u64::from(self.value_length))
            * 2""",
            ),
        ],
        ["cargo", "test", "--locked", "-p", "wipemark-models", "--lib",
         "a_shape_no_model_has_is_estimated_without_overflowing"],
    ),
    (
        "overflow-llama",
        [
            (
                "crates/wipemark-llama/src/model.rs",
                """        u64::from(self.n_layer)
            .saturating_mul(u64::from(self.n_head_kv))
            .saturating_mul(u64::from(self.key_length) + u64::from(self.value_length))""",
                """        u64::from(self.n_layer)
            * u64::from(self.n_head_kv)
            * (u64::from(self.key_length) + u64::from(self.value_length))""",
            ),
        ],
        ["cargo", "test", "--locked", "-p", "wipemark-llama", "--lib",
         "a_shape_no_model_has_is_estimated_without_overflowing"],
    ),
    (
        "taken-window",
        [
            (
                "apps/wipemark-app/src/settings.rs",
                """                taken.iter().any(|known| known == id)
                    || !matches!(
                        rows.settings()
                            .get::<serde_json::Value>(&wipemark_models::user::key_of(id)),
                        Ok(None)
                    )""",
                """                taken.iter().any(|known| known == id)""",
            ),
        ],
        ["cargo", "test", "--locked", "-p", "wipemark-app", "--bin", "wipemark",
         "an_add_never_writes_over_a_row_the_page_does_not_hold"],
    ),
    (
        "taken-cli",
        [
            (
                "apps/wipemark-cli/src/models.rs",
                """            known.iter().any(|model| model.id == id)
                || !matches!(writer.get::<serde_json::Value>(&user::key_of(id)), Ok(None))""",
                """            known.iter().any(|model| model.id == id)""",
            ),
        ],
        ["cargo", "test", "--locked", "-p", "wipemark-cli", "--test", "cli",
         "models_add_never_writes_over_a_row_it_cannot_read"],
    ),
]


def main() -> int:
    for name, edits, command in CHECKS:
        saved = {}
        try:
            for path, fixed, before in edits:
                file = ROOT / path
                text = file.read_bytes()
                saved[file] = text
                source = text.decode()
                if source.count(fixed) != 1:
                    raise SystemExit(f"{name}: the fixed text is not in {path} once")
                file.write_text(source.replace(fixed, before))
            run = subprocess.run(command, cwd=ROOT, capture_output=True, text=True)
            out = run.stdout + run.stderr
            if run.returncode == 0:
                verdict = "GREEN"
            elif "test result:" in out and ("FAILED" in out or "panicked" in out):
                verdict = "RED"
            else:
                verdict = "BROKEN"
                sys.stderr.write(out[-3000:])
            print(f"{name}: {verdict} {command[-1]}", flush=True)
        finally:
            for file, text in saved.items():
                file.write_bytes(text)
    return 0


if __name__ == "__main__":
    sys.exit(main())
