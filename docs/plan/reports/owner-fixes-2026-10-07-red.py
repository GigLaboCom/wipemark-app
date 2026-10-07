#!/usr/bin/env python3
"""The red checks of the owner's fixes F1-F6 (2026-10-07), as they were run.

What it is for
    The task `wipemark-task-owner-fixes-2026-10-07` (the coordinator, for
    the owner, 2026-10-07) asks that every protection added be deleted
    once, locally, and its test seen to go red. This script is the record
    of how that was done, so the claim in owner-fixes-2026-10-07.md can be
    re-checked. It is not a mutation table to run every round (the owner,
    2026-10-06, `wipemark-mutations-not-needed-2026-10-06`): it was run
    once, while the fixes were written, and is kept because a figure no
    script can reproduce is a figure nobody can check (CLAUDE.md).

What it does
    For each named check: replace one exact piece of source (the
    protection) with its deletion, run the named `cargo test` filter, print
    the tests that failed, and put the source back byte for byte — also
    when the run is interrupted. A check whose tests all pass is reported
    as GREEN, which would mean the test does not guard the protection.

How to run
    From the repository root, on the branch that carries F1-F6:
        python3 docs/plan/reports/owner-fixes-2026-10-07-red.py          # all
        python3 docs/plan/reports/owner-fixes-2026-10-07-red.py F3 F5a   # some
    On Linux the build needs libxkbcommon-x11.so on the linker path
    (docs/plan/reports/gpui-bump-host-check.md); set LIBRARY_PATH first.

What it needs
    Python 3, cargo, and nothing else. Each check is a cargo build of the
    crate concerned: minutes the first time, seconds after.

What the output means
    One line per check: RED with the failing tests (the protection is
    guarded), GREEN (it is not), or MISSING (the source moved and the
    piece to delete is no longer there — update the entry).
"""

import subprocess
import sys

# name: (file, the protection as it is, what it becomes, cargo test args)
CHECKS = {
    "F6": (
        "crates/wipemark-core/src/guard.rs",
        "        .flat_map(between_placeholders)\n",
        "",
        ["-p", "wipemark-core", "-p", "wipemark-pipeline", "--", "identifier", "link_only"],
    ),
    "F5a-clipboard": (
        "apps/wipemark-app/src/clipboard.rs",
        "                // stands in for a caption an image would have beaten.\n                if !handed.is_nothing() {",
        "                // stands in for a caption an image would have beaten.\n                if true {",
        ["-p", "wipemark-app", "--", "clipboard::", "empty_text"],
    ),
    "F5b-catcher": (
        "apps/wipemark-app/src/drop.rs",
        "            .filter(|handed| !handed.is_nothing())",
        "            .filter(|_| true)",
        ["-p", "wipemark-app", "--", "empty_text"],
    ),
    "F2a-elsewhere": (
        "crates/wipemark-models/src/store.rs",
        "        let expected = file.sha256.as_deref()?;\n",
        "        let expected = file.sha256.as_deref()?;\n        if !expected.is_empty() { return None; }\n",
        ["-p", "wipemark-models", "--", "store::"],
    ),
    "F2b-size": (
        "crates/wipemark-models/src/store.rs",
        "found.bytes == size && found.path",
        "(found.bytes == size || true) && found.path",
        ["-p", "wipemark-models", "--", "store::"],
    ),
    "F2c-remove": (
        "crates/wipemark-models/src/store.rs",
        "        let mut doomed = vec![dir.join(META_FILE)];",
        "        std::fs::remove_dir_all(&dir).ok();\n        let mut doomed = vec![dir.join(META_FILE)];",
        ["-p", "wipemark-models", "--", "store::"],
    ),
    "F2d-fetch": (
        "crates/wipemark-models/src/store.rs",
        "        if let (State::Present { .. }, Some(weights)) = (&located.state, &located.weights) {",
        "        if let (State::Present { .. }, Some(weights), false) = (&located.state, &located.weights, true) {",
        ["-p", "wipemark-models", "--", "store::"],
    ),
    "F2e-card": (
        "apps/wipemark-app/src/models.rs",
        "            Some(at) => Availability::Found { at: at.to_owned() },",
        "            Some(_) => Availability::Installed,",
        ["-p", "wipemark-app", "--", "found_elsewhere"],
    ),
    "F4a-records": (
        "crates/wipemark-models/src/store.rs",
        '        self.records.join(format!("{}-{name}", &key[..32]))',
        '        target.with_file_name(format!(".{}-{name}", &key[..32]))',
        ["-p", "wipemark-models", "--", "store::"],
    ),
    "F4b-cache": (
        "crates/wipemark-models/src/store.rs",
        "        if let Some(actual) = self.recorded(target) {",
        "        if let Some(actual) = self.recorded(target).filter(|_| false) {",
        ["-p", "wipemark-models", "--", "store::"],
    ),
    "F3": (
        "apps/wipemark-app/src/settings.rs",
        "        if self.scanning {\n            self.rescan = true;\n            return;\n        }\n",
        "",
        ["-p", "wipemark-app", "--", "two_scans"],
    ),
    "F1-bar": (
        "apps/wipemark-app/src/models.rs",
        "    let value = card.availability.bar()?;",
        "    let value: f32 = None?;",
        ["-p", "wipemark-app", "--", "bar"],
    ),
    "F1-hash-store": (
        "crates/wipemark-models/src/store.rs",
        "            if due {",
        "            if due && false {",
        ["-p", "wipemark-models", "--", "a_hash_tells"],
    ),
    "F1-hash-listener": (
        "apps/wipemark-app/src/settings.rs",
        "        Self::listen_to_hashes(&self.models, cx);\n",
        "",
        ["-p", "wipemark-app", "--", "a_hash_in_progress"],
    ),
    "F1a-forwarding": (
        "apps/wipemark-app/src/engine_host.rs",
        "        if let Slot::Engine(engine) = &slot {\n            engine.watch_loads(self.shared.loads.clone());\n        }\n",
        "",
        ["-p", "wipemark-app", "--", "a_load_tells"],
    ),
    "F1b-ended": (
        "crates/wipemark-engine/src/local.rs",
        "    let _ = sink.send(LoadProgress::Ended);\n",
        "",
        ["-p", "wipemark-engine", "--features", "local-llama", "--", "a_load_tells"],
    ),
    "F1c-pacer": (
        "crates/wipemark-engine/src/progress.rs",
        "        if through {",
        "        if through || true {",
        ["-p", "wipemark-engine", "--", "progress"],
    ),
    "F1d-host-clears": (
        "apps/wipemark-app/src/engine_host.rs",
        "            LoadProgress::Ended => None,",
        "            LoadProgress::Ended => self.loading,",
        ["-p", "wipemark-app", "--", "a_load_tells"],
    ),
}


def run(name):
    path, old, new, args = CHECKS[name]
    source = open(path, encoding="utf-8").read()
    if old not in source:
        return f"{name}: MISSING ({path})"
    try:
        with open(path, "w", encoding="utf-8") as out:
            out.write(source.replace(old, new, 1))
        done = subprocess.run(
            ["cargo", "test", "--locked", *args], capture_output=True, text=True
        )
    finally:
        with open(path, "w", encoding="utf-8") as out:
            out.write(source)
    failed = [
        line.split()[1]
        for line in (done.stdout + done.stderr).splitlines()
        if line.startswith("test ") and line.endswith("FAILED")
    ]
    if failed:
        return f"{name}: RED {', '.join(failed)}"
    if done.returncode != 0:
        return f"{name}: RED (did not build or did not run: exit {done.returncode})"
    return f"{name}: GREEN — the protection is not guarded"


if __name__ == "__main__":
    for name in sys.argv[1:] or CHECKS:
        print(run(name), flush=True)
