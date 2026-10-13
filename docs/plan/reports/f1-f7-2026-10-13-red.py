#!/usr/bin/env python3
"""The red checks of acceptance F1 and F7 (D715–D719).

What it is for, and who asked: the owner's acceptance of 0.1.0 (2026-10-12)
found that a model download wrote nothing to the log (F1) and that a SIGTERM
left the MCP beacon behind (F7); the owner asked for both on 2026-10-13. The
fix (`fix/f1-f7`) is guarded by ten tests, and the repository's rule is that
a protection is deleted once and its test is seen to go red. This script is
that check, kept so the table in `f1-f7-2026-10-13.md` can be reproduced.

What it does, step by step: for each mutation below it replaces one exact
piece of source with a broken one, runs the test that guards it with
`--exact` (`cargo test -p <package> --lib|--bin wipemark -- <name> --exact`),
records whether it failed, and puts the source back — whatever happened, in
a `finally`. A mutation whose text is not in the file exactly once stops the
run before anything is changed.

How to run it, from the repository root (on Linux, with the `LIBRARY_PATH`
CLAUDE.md describes under "Linking on Linux"):

    python3 docs/plan/reports/f1-f7-2026-10-13-red.py

What it needs: Python 3 and cargo. No network beyond loopback, no model, and
no signal sent to any process: the signal tests drive the plumbing, never a
real signal.

What its output means: one line per mutation — `red` when the guarding test
failed with the protection deleted (what is wanted), `GREEN` when it still
passed (a test that cannot fail), `DID NOT BUILD` when the mutation broke
the build. It exits 1 unless every line is `red`.
"""

import subprocess
import sys

STORE = "crates/wipemark-models/src/store.rs"
BEACON = "crates/wipemark-models/src/beacon.rs"
SERVER = "apps/wipemark-app/src/mcp/server.rs"
SIGNALS = "apps/wipemark-app/src/signals.rs"

THROTTLE = "store::tests::a_download_logs_its_progress_sparingly"
SAYS = "store::tests::a_download_says_what_it_does_in_the_log"
SWEPT = "mcp::server::tests::a_beacon_a_dead_process_left_is_swept_when_the_server_starts"
FIRST = "signals::tests::the_first_signal_quits_and_a_second_ends_at_once"
HANDED = "signals::tests::a_signal_handed_over_quits_on_the_gpui_thread"

# (decision and protection, file, exact text, its stand-in, package, test)
MUTATIONS = [
    ("D716 the first byte said", STORE,
     "    let Some(last) = last else {\n        return true;\n    };\n    let since",
     "    let Some(last) = last else {\n        return false;\n    };\n    let since",
     "wipemark-models", THROTTLE),
    ("D716 never twice within a second", STORE,
     "    if since < LOG_AT_MOST {\n        return false;\n    }\n",
     "",
     "wipemark-models", THROTTLE),
    ("D716 a tenth and thirty seconds, whichever later", STORE,
     "    a_tenth && since >= LOG_EVERY",
     "    a_tenth || since >= LOG_EVERY",
     "wipemark-models", THROTTLE),
    ("D716 the end waits for neither", STORE,
     "    if done >= total {\n        return last.done < done;\n    }\n",
     "",
     "wipemark-models", THROTTLE),
    ("D716 the end said once", STORE,
     "        return last.done < done;",
     "        return true;",
     "wipemark-models", THROTTLE),
    ("D715 the start line", STORE,
     "        self.done = from;\n        tracing::info!(",
     "        self.done = from;\n        #[cfg(any())]\n        tracing::info!(",
     "wipemark-models", SAYS),
    ("D715 the host only", STORE,
     "            host = %host_of(url),",
     "            host = %url,",
     "wipemark-models", SAYS),
    ("D715 the progress line", STORE,
     "        if !should_log(now, done, total, self.last) {\n            return;\n        }",
     "        if true {\n            return;\n        }",
     "wipemark-models", SAYS),
    ("D715 the verified end", STORE,
     "            Some(_) => told.verified(told.done),",
     "            Some(_) => {}",
     "wipemark-models", SAYS),
    ("D715 the mismatched end", STORE,
     "                told.mismatched(expected, &actual);\n",
     "",
     "wipemark-models", "store::tests::a_mismatched_download_says_both_hashes"),
    ("D715 the cancelled end", STORE,
     "        if matches!(error, StoreError::Cancelled) {",
     "        if false {",
     "wipemark-models", "store::tests::a_cancelled_download_says_so"),
    ("D715 the wire's HTTP status", STORE,
     "            told.stopped(&error, Wire::of(&err));\n",
     "",
     "wipemark-models", "store::tests::a_failed_download_says_its_kind_and_status"),
    ("D715 the resume line", STORE,
     "        if resumed {\n            tracing::info!(",
     "        if resumed {\n            #[cfg(any())]\n            tracing::info!(",
     "wipemark-models", "store::tests::an_interrupted_download_asks_only_for_the_rest"),
    ("D715 the ignored-range line", STORE,
     "            self.done = 0;\n            tracing::info!(",
     "            self.done = 0;\n            #[cfg(any())]\n            tracing::info!(",
     "wipemark-models",
     "store::tests::a_server_that_ignores_a_range_does_not_corrupt_the_file"),
    ("D718 the sweep at the supervisor's start", SERVER,
     "    sweep_beacon(beacon);\n",
     "",
     "wipemark-app", SWEPT),
    ("D718 a running process's beacon stays", BEACON,
     "        if found.alive() {\n            return None;\n        }\n",
     "",
     "wipemark-models", "beacon::tests::a_dead_process_s_beacon_is_removed_and_a_live_one_is_not"),
    ("D718 a running process's beacon stays", BEACON,
     "        if found.alive() {\n            return None;\n        }\n",
     "",
     "wipemark-app", SWEPT),
    ("D717 a second signal ends at once", SIGNALS,
     "        if std::mem::replace(&mut self.quitting, true) {",
     "        if false {",
     "wipemark-app", FIRST),
    ("D717 the first signal handed to the GPUI side", SIGNALS,
     "                if tell.send(signal).is_err() {",
     "                if false {",
     "wipemark-app", FIRST),
    ("D717 a first signal with nobody to hear it ends at once", SIGNALS,
     "                        name(signal)\n                    );\n                    end(code);\n",
     "                        name(signal)\n                    );\n",
     "wipemark-app", FIRST),
    ("D717 quit on the GPUI thread", SIGNALS,
     "                quit(cx);\n",
     "                let _ = &quit;\n",
     "wipemark-app", HANDED),
    ("D717 the line `quitting on SIGTERM`", SIGNALS,
     "                tracing::info!(\"quitting on {}\", name(signal));\n",
     "",
     "wipemark-app", HANDED),
]


def main():
    for name, path, old, _, _, _ in MUTATIONS:
        found = open(path).read().count(old)
        if found != 1:
            sys.exit(f"{name}: the text is in {path} {found} times, not once")

    verdicts = []
    for name, path, old, new, package, test in MUTATIONS:
        original = open(path).read()
        try:
            open(path, "w").write(original.replace(old, new))
            target = ["--bin", "wipemark"] if package == "wipemark-app" else ["--lib"]
            run = subprocess.run(
                ["cargo", "test", "-p", package, *target, "--", test, "--exact"],
                capture_output=True,
                text=True,
            )
        finally:
            open(path, "w").write(original)
        out = run.stdout + run.stderr
        if "could not compile" in out:
            verdict = "DID NOT BUILD"
        elif run.returncode != 0 and "FAILED" in out:
            verdict = "red"
        elif "1 passed" in out:
            verdict = "GREEN"
        else:
            verdict = "unclear: " + out[-300:]
        verdicts.append(verdict)
        print(f"{name} | {test} | {verdict}", flush=True)
    sys.exit(0 if all(verdict == "red" for verdict in verdicts) else 1)


if __name__ == "__main__":
    main()
