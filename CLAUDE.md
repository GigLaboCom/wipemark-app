# Wipemark — working notes

@dev/working-notes.md

The working notes are in the developer repository, the private
`GigLaboCom/wipemark-dev`, which `scripts/dev-checkout.sh` clones into
`dev/` (in a worktree: links it to the main copy's `dev/`); the line above
imports them. Without `dev/`, `CONTRIBUTING.md` has the build, the gates
and the rules a change keeps, and a `dev/…` path in a comment names a
document you cannot open.

Two rules hold either way:

* **A commit has human authors only** — no `Co-Authored-By:` naming an
  LLM, no `Claude-Session:` line, no "Generated with" line, here and in
  anything sent upstream.
* **Nothing private enters this repository**: no developer document, no
  path into a developer's machine, no machine's or employer's name, no
  private infrastructure, no secret. `scripts/install-hooks.sh` turns on
  the pre-commit scan (gitleaks), with the private rules when `dev/` is
  there.
