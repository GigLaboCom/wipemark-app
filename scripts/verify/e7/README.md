# scripts/verify/e7 — the host verification of E7, the windows clean

Written by the host verifier of the E7 series (asked by the coordinator,
2026-10-05), and extended by the host verifier of its first follow-ups,
W1–W15 (asked by the coordinator, 2026-10-06), and again by the host
verifier of its second follow-ups, X1–X14 (asked by the coordinator,
2026-10-06). Each script opens with a
header: what it is for, what it does, how to run it, what it needs and what
its output means. Run them from the repository root of an
`e7/windows-clean` checkout. Every path is a parameter (environment
variables); none is a home directory.

| script | what it checks |
|---|---|
| `mutate-host.py` | the verifier's own mutations: protections the implementer's `docs/plan/reports/e7-windows-clean-mutate.py` does not cover. H1–H18 (2026-10-05): Layer A's defaults, the picture scope, C2PA alone, what is kept, the log line, the sweep's names, the in-place road, the picture limit, the catalogue. H6 and H14 follow the code W3 and D284 moved; H12 is retired as an equivalent mutation (D281). H19–H39 (2026-10-06): the application's one line of cleans, the no-clobber publish and set-aside, the Markdown copy, the report's own shelf, Compare's strict road, the CLI's parity table, the epic-number gate, the keep sentence, and whether W5's FIFO test can hang. H31 follows the code X10 moved. H40–H56 (X1–X14): the queue's recovery after a hard-link set-aside (D286), the link refusal and a linked folder (D287), the panel's look by the clean's plan, the panicked verdict and its words (D288), a text's own third shelf and its JSON (D289, also against the CLI's and the app's suites alone), the Markdown copy's spelling by position (X10) and the epic-number gate's letters (X9); some are questions rather than protections, and say so. Each must go red over the whole crate; a run past its timeout is killed and reported as HANG. An entry may edit several files. |
| `parity.sh` | the window's clean against the CLI's, byte for byte: `wipemark-cli clean -o` and `clean::clean_one` (through a temporary `#[ignore]`d probe appended to `clean.rs` and removed afterwards) over the same inputs; a table of exits, verdicts and sha256s. Since 2026-10-06 it also copies every input of `fixtures/clean-parity/table.tsv` (D285) and prints a second table: per row, the table's CLI exit, CLI write, app write and (since X11) app verdict beside what this script measured. |
| `json-bytes.sh` | (X1–X14) D289 moved a text's third shelf into core's report: two `wipemark-cli` builds, feat's and the head's, over twenty-one inputs — `inspect --json`, `clean --json` with every Layer A option, stdin, prose, `audit --json` and `--sarif` — every stdout, stderr, exit code and written file `cmp`ed. |
| `mcp-bytes.sh` | (X1–X14) the same for the MCP server, headlessly: a temporary `#[ignore]`d test in each tree hands JSON-RPC bodies (`tools/call` of `inspect` and `clean` over the same texts and options, `initialize`, `tools/list`, `ping`, two refusals) to `mcp::protocol::respond`, and the answers are `cmp`ed. |
| `live-disk.sh` | the automatable half of `docs/plan/reports/E7-windows-clean-live-check.md`: the real application launched with `--clean=` against a seeded scratch `WIPEMARK_DATA_DIR`, the disk and the log checked, no clicks and no keystrokes; cases 1–6, 8, 9 and the first half of 10. `APP_DBUS` picks the session bus the application is given. |
| `probes.sh` | (2026-10-06) questions the follow-ups' code raised that no test in the tree answers, each answered by a temporary test or by the built CLI and put back afterwards: the queue's recovery of an in-place delivery a crash interrupted after D284's hard link; the Markdown copy in ru and de for Cyrillic and umlaut names; the windows' in-place clean of a symbolic link; `clean --in-place` over a file with a second hard link. Since X1–X14: A asks what the round's own test does not (the file replaced by someone's atomic save after a hard-link set-aside, of the same length and another); C2 an in-place clean through a linked folder; G the link refusal (D287) and the linked folder through the running application with `--clean=`; H a panic in `plan_for`, outside D288's catch. `PROBES` picks probes by letter. |
| `clip.py` | `pbcopy` / `pbpaste` over GTK's clipboard, for running the live check's clipboard cases (7, 11, 12) on a Linux host that has no xclip, xsel or wl-copy. |

Linking the app's tests on the Ubuntu host needs `LIBRARY_PATH` pointing at a
directory holding a `libxkbcommon-x11.so` symlink to the system's `.so.0`.

On the 2026-10-05 build GPUI panicked on this Ubuntu X11 host at the first
frame when the desktop portal reported the appearance (`RefCell already
mutably borrowed`, `gpui_linux` `x11/window.rs:1556`). `live-disk.sh` starts
the app with an unreachable session bus by default (`APP_DBUS`, default
`unix:path=/nonexistent-wipemark-verify`) so the portal stays quiet. Since
`2e006cf` GPUI comes from `GigLaboCom/zed` with that fix; run `live-disk.sh`
a second time with `APP_DBUS=$DBUS_SESSION_BUS_ADDRESS` to check the real
bus.
