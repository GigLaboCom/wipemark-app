#!/usr/bin/env python3
"""The images series' mutation table (E11-3, E12-1 ... E12-5), as a script
the verifier runs.

Each mutation is applied alone to the sources, the named tests run, and the
file is restored from memory whatever happens. A mutation PASSES this script
when its tests go **red** — the protection is real — and FAILS it when they
stay green. Run from the repository root, after the gates are green:

    python3 docs/plan/reports/images-series-mutate.py              # every mutation
    python3 docs/plan/reports/images-series-mutate.py E11-3        # one step
    python3 docs/plan/reports/images-series-mutate.py E12-1/M4     # one mutation
    python3 docs/plan/reports/images-series-mutate.py --check      # every text is there once
    python3 docs/plan/reports/images-series-mutate.py --compile    # each applied and compiled, nothing run

Written in a container that could not run the tests (only compile them):
`--check` and `--compile` were run there; whether each mutation goes red is
the verifier's to establish.
"""

import subprocess
import sys

CLI = ["-p", "wipemark-cli"]
APP = ["-p", "wipemark-app"]
IMG = ["-p", "wipemark-image"]
I18N = ["-p", "wipemark-i18n"]
PIX = ["-p", "wipemark-pixels"]
PIC = ["-p", "wipemark-picture"]

# (id, protection, file, old, new, [(cargo test args, test name filter)])
MUTATIONS = [
    # ------------------------------------------------------------ E11-3
    (
        "E11-3/M1",
        "an unknown critical PNG chunk is structure",
        "crates/wipemark-image/src/png.rs",
        "if STRUCTURE.contains(&ty) || ty[0].is_ascii_uppercase() {",
        "if STRUCTURE.contains(&ty) {",
        [(IMG + ["--test", "gaps"], "an_unknown_critical_chunk_is_structure")],
    ),
    (
        "E11-3/M2",
        "a C2PA box across APP11 segments is grouped by instance",
        "crates/wipemark-image/src/jpeg.rs",
        "                mut block,\n                instance,\n                ..\n            } => {\n"
        "                if c2pa_instances.contains(&instance) {",
        "                mut block,\n                c2pa,\n                ..\n            } => {\n"
        "                if c2pa {",
        [(IMG + ["--test", "gaps"], "a_c2pa_manifest_across_app11_segments_leaves_whole")],
    ),
    (
        "E11-3/M3",
        "the grouping is by instance, not every JUMBF",
        "crates/wipemark-image/src/jpeg.rs",
        "if c2pa_instances.contains(&instance) {",
        "if true || c2pa_instances.contains(&instance) {",
        [(IMG + ["--test", "gaps"], "an_unlabelled_jumbf_of_another_instance_is_not_c2pa")],
    ),
    (
        "E11-3/M4",
        "the MP Index's first size follows a removal before it",
        "crates/wipemark-image/src/lib.rs",
        "out[at - gone..at - gone + 4].copy_from_slice(&field);",
        "let _ = field;",
        [(IMG + ["--test", "gaps"], "the_mp_index_follows_a_removal_before_it")],
    ),
    (
        "E11-3/M5",
        "the MP Index is written in its own byte order",
        "crates/wipemark-image/src/lib.rs",
        "let field = if big {",
        "let field = if true {",
        [(IMG + ["--test", "gaps"], "the_mp_index_follows_a_removal_before_it")],
    ),
    (
        "E11-3/M6",
        "an MP Index that cannot be read refuses a removal",
        "crates/wipemark-image/src/lib.rs",
        "MpIndex::Unreadable => return Err(refuse(m)),",
        "MpIndex::Unreadable => {}",
        [(IMG + ["--test", "gaps"], "an_mp_index_that_cannot_be_read_refuses_a_removal")],
    ),
    (
        "E11-3/M7",
        "a removed orientation is reported",
        "crates/wipemark-image/src/lib.rs",
        "report.orientation_removed = orientation;",
        "let _ = orientation;",
        [(IMG + ["--test", "gaps"], "a_removed_orientation_is_reported")],
    ),
    (
        "E11-3/M8",
        "orientation 1 (and 0, 9) is not a rotation",
        "crates/wipemark-image/src/exif.rs",
        "return (2..=8).contains(&value).then_some(value);",
        "return Some(value);",
        [(IMG + ["--test", "gaps"], "orientation_one_is_not_a_rotation")],
    ),
    (
        "E11-3/M9",
        "the JSON carries the orientation",
        "crates/wipemark-image/src/json.rs",
        '            Some(value) => {\n                let _ = write!(out, "{value}");',
        '            Some(_value) => {\n                out.push_str("null");',
        [
            (IMG + ["--lib"], "the_json_form_of_a_strip_report_is_exact"),
            (IMG + ["--test", "gaps"], "a_removed_orientation_is_reported"),
        ],
    ),
    (
        "E11-3/M10",
        "the CLI says the rotation when it was removed",
        "apps/wipemark-cli/src/image.rs",
        "if report.orientation_removed.is_some() {",
        "if false {",
        [(CLI + ["--bin", "wipemark-cli"], "the_rotation_is_said_only_when_it_was_removed")],
    ),
]


def run(args, name):
    """`(red, compiled, command)`: a mutation that does not compile is not a
    protection that bit, and is reported apart."""
    command = ["cargo", "test", "--locked"] + args + ["--", name]
    done = subprocess.run(command, capture_output=True, text=True)
    compiled = "could not compile" not in done.stderr
    ran = "running " in done.stdout
    return done.returncode != 0 and compiled and ran, compiled, " ".join(command)


def compile_only(args):
    """Whether the mutated tree builds the test binary — nothing run."""
    packages = []
    i = 0
    while i < len(args):
        if args[i] == "-p":
            packages += args[i : i + 2]
            i += 2
        else:
            i += 1
    command = ["cargo", "test", "--locked", "--no-run"] + packages
    done = subprocess.run(command, capture_output=True, text=True)
    return done.returncode == 0, " ".join(command)


def selected(wanted):
    for m in MUTATIONS:
        mid = m[0]
        if not wanted or mid in wanted or mid.split("/")[0] in wanted:
            yield m


def check(wanted):
    """Every text to mutate is there exactly once — nothing is run."""
    ok = True
    for mid, _, path, old, _, _ in selected(wanted):
        with open(path, encoding="utf-8") as f:
            n = f.read().count(old)
        if n != 1:
            ok = False
        print(f"{mid}: {path}: {n} match{'es' if n != 1 else ''}")
    return 0 if ok else 1


def mutate(path, old, new, body):
    with open(path, encoding="utf-8") as f:
        original = f.read()
    if original.count(old) != 1:
        return None
    try:
        with open(path, "w", encoding="utf-8") as f:
            f.write(original.replace(old, new))
        return body()
    finally:
        with open(path, "w", encoding="utf-8") as f:
            f.write(original)


def compile_all(wanted):
    ok = True
    for mid, _, path, old, new, tests in selected(wanted):
        result = mutate(path, old, new, lambda: compile_only(sum((a for a, _ in tests), [])))
        if result is None:
            print(f"{mid}: NOT APPLIED: the text to mutate moved", flush=True)
            ok = False
            continue
        built, command = result
        ok &= built
        print(f"{mid}: {'compiles' if built else 'DOES NOT COMPILE'}   {command}", flush=True)
    return 0 if ok else 1


def main(wanted):
    results = []
    for mid, protection, path, old, new, tests in selected(wanted):

        def body():
            red = []
            for args, name in tests:
                bit, compiled, command = run(args, name)
                if not compiled:
                    command += "   <- DID NOT COMPILE"
                red.append((name, bit, command))
            return red

        red = mutate(path, old, new, body)
        if red is None:
            results.append((mid, protection, "NOT APPLIED: the text to mutate moved", ""))
            continue
        verdict = "red" if all(r for _, r, _ in red) else "GREEN — the protection did not bite"
        results.append((mid, protection, verdict, ", ".join(n for n, _, _ in red)))
        print(f"{mid}: {verdict}", flush=True)
        for name, r, command in red:
            print(f"    {'red  ' if r else 'GREEN'} {command}", flush=True)
    print()
    print("| # | protection | result | tests |")
    print("|---|---|---|---|")
    for mid, protection, verdict, names in results:
        print(f"| {mid} | {protection} | {verdict} | {names} |")
    return 0 if all(v == "red" for _, _, v, _ in results) else 1


if __name__ == "__main__":
    argv = sys.argv[1:]
    if argv[:1] == ["--check"]:
        sys.exit(check(set(argv[1:])))
    if argv[:1] == ["--compile"]:
        sys.exit(compile_all(set(argv[1:])))
    sys.exit(main(set(argv)))
