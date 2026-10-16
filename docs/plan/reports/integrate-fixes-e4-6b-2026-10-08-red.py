#!/usr/bin/env python3
"""The red checks of E4-6b's host-verification fixes M1-M3, L1-L5 (2026-10-08).

What it is for
    The coordinator's task of 2026-10-08 asks every fix to come with a test
    seen red once with the fix reverted (no mutation tables). This is the
    record of how each was seen: one protection replaced by an edit, the
    test that guards it run, the file put back.

What it does
    For each named check (all when none is named): applies its edits to the
    working tree, runs `cargo test --locked` with its filter under
    `timeout 600`, prints RED or GREEN with the last lines of the output,
    and restores every edited file byte for byte, whatever happened.
    `M2-fifo` hangs by design once both of its protections are gone — the
    timeout ends it, which is the red.

How to run
    From the repository root, on the commit that adds this file:
        python3 docs/plan/reports/integrate-fixes-e4-6b-2026-10-08-red.py [CHECK ...]
    `LIBRARY_PATH` is set to `.verify/lib` (a directory holding
    `libxkbcommon-x11.so`, which a GPUI test build links against on a host
    without the development package); create it or drop the line.

What it needs
    Python 3 (standard library), cargo, coreutils `timeout`; a FIFO-capable
    file system and `mkfifo` for the FIFO checks.

What its output means
    `RED` for every check is the expected result: each test fails without
    its protection. `GREEN` would mean a test that cannot fail.
"""
import os
import subprocess
import sys

ENV = dict(os.environ, LIBRARY_PATH=os.path.abspath('.verify/lib'))
APP = ['-p', 'wipemark-app', '--']

CHECKS = {
    # M1, D355: the waiter ends on Removed.
    'M1-wait': ([('apps/wipemark-app/src/mcp/rewrite.rs',
        "Ok(QueueEvent::Removed { item: of }) if of == item => {",
        "Ok(QueueEvent::Removed { item: of }) if of == item && false => {")],
        APP + ['mcp::protocol::tests::a_removed_agent', 'mcp::server::tests::the_command_lines_call']),
    # M1, D355: Remove greyed while a caller waits.
    'M1-remove': ([('apps/wipemark-app/src/queue.rs',
        "        Status::RewriteQueued | Status::Rewriting(_)\n            if matches!(origin, Origin::Agent | Origin::Cli) =>",
        "        Status::RewriteQueued | Status::Rewriting(_)\n            if false && matches!(origin, Origin::Agent | Origin::Cli) =>")],
        APP + ['queue::tests::remove_is_greyed']),
    # M2, D356: intake opens no FIFO.
    'M2-intake': ([('crates/wipemark-intake/src/lib.rs',
        "    if !std::fs::metadata(path).ok()?.is_file() {",
        "    if false && !std::fs::metadata(path).ok()?.is_file() {")],
        ['-p', 'wipemark-intake', '--lib', '--', 'a_fifo_with_no_writer']),
    # M2, D356: the window's journal read (both protections gone: it hangs).
    'M2-fifo': ([('apps/wipemark-app/src/journal.rs',
        "    if !(meta.is_file() || meta.is_dir()) {",
        "    if false && !(meta.is_file() || meta.is_dir()) {"),
        ('crates/wipemark-intake/src/lib.rs',
        "    if !std::fs::metadata(path).ok()?.is_file() {",
        "    if false && !std::fs::metadata(path).ok()?.is_file() {")],
        APP + ['queue::rewrite_tests::a_journal_row_naming_a_fifo']),
    # M2, D356: the CLI records a non-regular path as no file.
    'M2-cli': ([('apps/wipemark-cli/src/journal.rs',
        '    if path != "-" && is_a_file(Path::new(path)) {',
        '    if path != "-" {')],
        ['-p', 'wipemark-cli', '--bin', 'wipemark-cli', '--', 'journal::tests::a_path']),
    # M2, D356: a _meta path is said, never a file.
    'M2-meta': ([('apps/wipemark-app/src/mcp/rewrite.rs',
        "            said_path: self.path.clone(),",
        "            path: self.path.clone(),")],
        APP + ['mcp::protocol::tests::a_meta_path']),
    # M3, D357: checked at push.
    'M3': ([('apps/wipemark-app/src/queue/rewriting.rs',
        "                if std::fs::symlink_metadata(path).is_ok() {",
        "                if false && std::fs::symlink_metadata(path).is_ok() {")],
        APP + ['queue::rewrite_tests::a_rewrite_over_an_existing']),
    # L1, D358: the push back before the row's write.
    'L1': ([('apps/wipemark-app/src/queue/rewriting.rs',
        "                    match (&self.writer, &row.arrival) {\n                        (Some(writer), Some(arrival)) => writer.queue(",
        "                    let push = { push(); || {} };\n                    match (&self.writer, &row.arrival) {\n                        (Some(writer), Some(arrival)) => writer.queue(")],
        APP + ['queue::rewrite_tests::the_queued_row_is_written']),
    # L2, D359: paused from the row again.
    'L2': ([('crates/wipemark-queue/src/lib.rs',
        "        self.paused.load(Ordering::SeqCst)\n",
        "        self.store.queue().paused().unwrap_or(false)\n")],
        APP + ['queue::rewrite_tests::the_window_reads_no_row']),
    'L2-queue': ([('crates/wipemark-queue/src/lib.rs',
        "        self.paused.load(Ordering::SeqCst)\n",
        "        self.store.queue().paused().unwrap_or(false)\n")],
        ['-p', 'wipemark-queue', '--test', 'duty', '--', 'paused_is_answered']),
    # L3, D360: the template refusal recorded again.
    'L3': ([('apps/wipemark-cli/src/rewrite.rs',
        "        journal::discard();\n        let line = laid_line",
        "        journal::note(|draft| draft.failed = Some(\"templates\"));\n        let line = laid_line")],
        ['-p', 'wipemark-cli', '--bin', 'wipemark-cli', '--', 'rewrite::tests::a_template']),
    # L4, D361: no question before an item starts.
    'L4': ([('crates/wipemark-queue/src/worker.rs',
        "        if let Some(asking) = self.question_for(id) {",
        "        if let Some(asking) = None::<Asking>.filter(|_| self.question_for(id).is_some()) {")],
        ['-p', 'wipemark-queue', '--test', 'duty', '--', 'a_consent_to_stay_here']),
    # L5a, D362: the CLI's check before the read.
    'L5a': ([('apps/wipemark-cli/src/run.rs',
        '                    if command == "rewrite" && std::fs::symlink_metadata(&beside).is_ok() {',
        '                    if false && command == "rewrite" && std::fs::symlink_metadata(&beside).is_ok() {')],
        ['-p', 'wipemark-cli', '--test', 'cli', '--', 'a_rewrite_never_writes']),
    # L5b, D363: the date for another day.
    'L5b': ([('apps/wipemark-app/src/queue.rs',
        "    if at.date_naive() == now.date_naive() {",
        "    if at.date_naive() == now.date_naive() || true {")],
        APP + ['queue::tests::a_row_from_another_day']),
}


def run(name):
    edits, args = CHECKS[name]
    backups = {}
    for path, old, new in edits:
        if path not in backups:
            backups[path] = open(path).read()
        text = open(path).read()
        assert old in text, (name, path, old[:60])
        open(path, 'w').write(text.replace(old, new, 1))
    try:
        result = subprocess.run(['timeout', '600', 'cargo', 'test', '--locked'] + args,
                                capture_output=True, text=True, env=ENV)
        out = result.stdout + result.stderr
        lines = [line for line in out.splitlines()
                 if line.startswith('test ') or 'test result' in line or line.startswith('error')]
        red = result.returncode != 0
        print(f'== {name}: {"RED" if red else "GREEN (the test did not fail)"} (exit {result.returncode})',
              flush=True)
        for line in lines[-6:]:
            print('   ', line, flush=True)
    finally:
        for path, text in backups.items():
            open(path, 'w').write(text)


for check in (sys.argv[1:] or CHECKS):
    run(check)
