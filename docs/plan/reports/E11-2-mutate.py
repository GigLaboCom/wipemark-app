#!/usr/bin/env python3
"""E11-2's mutation table, as a script the host runs.

Each mutation is applied alone to the sources, the named tests run, and the
file is restored from memory whatever happens. A mutation PASSES this script
when its tests go **red** — the protection is real — and FAILS it when they
stay green. Run from the repository root, after the gates are green:

    python3 docs/plan/reports/E11-2-mutate.py            # every mutation
    python3 docs/plan/reports/E11-2-mutate.py M1 M15      # some
    python3 docs/plan/reports/E11-2-mutate.py --check     # every text is there, nothing run

Written in a container that could not run the tests (only compile them);
the report (`E11-2-2026-10-04.md`) says which results are the host's.
"""

import subprocess
import sys

CLI = ["-p", "wipemark-cli"]
APP = ["-p", "wipemark-app"]
IMG = ["-p", "wipemark-image"]
I18N = ["-p", "wipemark-i18n"]

# (id, protection, file, old, new, [(cargo test args, test name filter)])
MUTATIONS = [
    (
        "M1",
        "still_has_* never exits 0 or 1",
        "apps/wipemark-cli/src/image.rs",
        "if report.still_has_ai_metadata || report.still_has_c2pa {",
        "if false {",
        [(CLI + ["--bin", "wipemark-cli"], "an_output_that_still_carries_provenance_never_exits_zero")],
    ),
    (
        "M1b",
        "either still_has_* alone is enough",
        "apps/wipemark-cli/src/image.rs",
        "if report.still_has_ai_metadata || report.still_has_c2pa {",
        "if report.still_has_ai_metadata {",
        [(CLI + ["--bin", "wipemark-cli"], "an_output_that_still_carries_provenance_never_exits_zero")],
    ),
    (
        "M2",
        "inspect exits by AI provenance only",
        "apps/wipemark-cli/src/image.rs",
        "    if report.has_ai_metadata() {\n        Exit::Findings",
        "    if !report.findings.is_empty() {\n        Exit::Findings",
        [(CLI + ["--test", "image"], "inspect_exits_one_on_each_ai_signal_and_zero_on_a_camera")],
    ),
    (
        "M3",
        "a name alone does not make a picture",
        "apps/wipemark-cli/src/input.rs",
        "by_content && PICTURES.contains(format)",
        "PICTURES.contains(format)",
        [(CLI + ["--bin", "wipemark-cli"], "a_picture_is_decided_by_its_bytes")],
    ),
    (
        "M4",
        "rewrite's reader still refuses a picture",
        "apps/wipemark-cli/src/input.rs",
        "read_source(source, stdin, false)?",
        "read_source(source, stdin, true)?",
        [(CLI + ["--bin", "wipemark-cli"], "only_read_any_hands_over_a_picture")],
    ),
    (
        "M5",
        "TIFF is a refusal by name, 2",
        "apps/wipemark-cli/src/image.rs",
        'ImageError::NotYet(_) => ("image not yet", Exit::Usage),',
        'ImageError::NotYet(_) => ("image not yet", Exit::Partial),',
        [(CLI + ["--test", "image"], "a_tiff_is_refused_by_name")],
    ),
    (
        "M6",
        "a malformed picture is not read and not clean, 3",
        "apps/wipemark-cli/src/image.rs",
        '("image malformed", Exit::Partial)',
        '("image malformed", Exit::Usage)',
        [(CLI + ["--test", "image"], "a_truncated_jpeg_is_not_read_and_not_clean")],
    ),
    (
        "M7",
        "never image bytes to a terminal",
        "apps/wipemark-cli/src/image.rs",
        "if stdout_is_terminal {",
        "if false {",
        [(CLI + ["--bin", "wipemark-cli"], "image_bytes_are_never_written_to_a_terminal")],
    ),
    (
        "M8",
        "--json never shares stdout with an image",
        "apps/wipemark-cli/src/image.rs",
        "        if json {\n            let line = run::say(Message::CliImageJsonStdout",
        "        if false {\n            let line = run::say(Message::CliImageJsonStdout",
        [
            (CLI + ["--bin", "wipemark-cli"], "image_bytes_are_never_written_to_a_terminal"),
            (CLI + ["--test", "image"], "a_picture_on_standard_input_goes_to_standard_output"),
        ],
    ),
    (
        "M9",
        "a text flag on a picture is a usage error",
        "apps/wipemark-cli/src/run.rs",
        "if let Some(flag) = text_flag {",
        "if let Some(flag) = text_flag.filter(|_| false) {",
        [(CLI + ["--test", "image"], "a_flag_for_the_other_kind_is_a_usage_error")],
    ),
    (
        "M10",
        "--all-metadata on a text is a usage error",
        "apps/wipemark-cli/src/run.rs",
        "Content::Text(_) if all_metadata => {",
        "Content::Text(_) if false => {",
        [
            (CLI + ["--bin", "wipemark-cli"], "all_metadata_on_a_text_is_a_usage_error"),
            (CLI + ["--test", "image"], "a_flag_for_the_other_kind_is_a_usage_error"),
        ],
    ),
    (
        "M11",
        "every picture report says the pixels were not examined",
        "apps/wipemark-cli/src/image.rs",
        "let mut lines = vec![say(Message::CliImagePixels, &FluentArgs::new())];",
        "let mut lines = Vec::new();",
        [
            (CLI + ["--bin", "wipemark-cli"], "every_image_report_says_the_pixels_were_not_examined"),
            (CLI + ["--test", "image"], "inspect_exits_one_on_each_ai_signal_and_zero_on_a_camera"),
        ],
    ),
    (
        "M12",
        "an unreadable picture makes audit inconclusive (3 beats 1)",
        "apps/wipemark-cli/src/audit.rs",
        "matches!(self, Status::Unreadable(_) | Status::ImageUnreadable(_))",
        "matches!(self, Status::Unreadable(_))",
        [(CLI + ["--test", "image"], "an_unreadable_picture_makes_the_audit_inconclusive")],
    ),
    (
        "M13",
        "a picture with AI provenance is an audit finding",
        "apps/wipemark-cli/src/audit.rs",
        "Status::Image(report) => report.has_ai_metadata(),",
        "Status::Image(_) => false,",
        [(CLI + ["--test", "image"], "audit_lists_pictures_in_every_output")],
    ),
    (
        "M14",
        "a picture's SARIF region is bytes, no line",
        "apps/wipemark-cli/src/audit.rs",
        '"byteOffset": finding.offset,',
        '"startLine": 1, "byteOffset": finding.offset,',
        [(CLI + ["--test", "image"], "audit_lists_pictures_in_every_output")],
    ),
    (
        "M15",
        "a string read out of the file is spelled",
        "crates/wipemark-image/src/json.rs",
        "if character == ' ' || character.is_ascii_graphic() {",
        "if true {",
        [
            (IMG + ["--lib"], "a_keyword_read_out_of_the_file_is_spelled"),
            (APP + ["--bin", "wipemark"], "no_image_answer_carries_a_character_it_read_out_of_the_file"),
        ],
    ),
    (
        "M16",
        "the third shelf is written whatever the field says",
        "crates/wipemark-image/src/json.rs",
        "for (i, (id, _)) in not_established::ALL.iter().enumerate() {",
        "for (i, (id, _)) in not_established::ALL.iter().take(0).enumerate() {",
        [
            (IMG + ["--lib"], "the_json_form_of_an_image_report_is_exact"),
            (APP + ["--bin", "wipemark"], "every_image_answer_carries_the_third_shelf"),
            (CLI + ["--test", "image"], "inspect_json_is_ascii_parses_and_carries_the_third_shelf"),
        ],
    ),
    (
        "M17",
        "an MCP picture refusal is an isError result, never a report",
        "apps/wipemark-app/src/mcp/protocol.rs",
        "Err(refusal) => Answer::Result(image_refused(tool, &refusal)),",
        'Err(_) => answered("{}".to_owned()),',
        [(APP + ["--bin", "wipemark"], "every_image_refusal_is_an_error_result_naming_why")],
    ),
    (
        "M18",
        "the base64 round trip is byte-identical",
        "apps/wipemark-app/src/mcp/image.rs",
        "BASE64.encode(bytes)",
        "BASE64.encode(&bytes[..bytes.len().saturating_sub(1)])",
        [
            (APP + ["--bin", "wipemark"], "base64_reads_the_rfc_vectors_and_nothing_looser"),
            (APP + ["--bin", "wipemark"], "the_base64_round_trip_is_byte_identical"),
        ],
    ),
    (
        "M19",
        "a picture over a megabyte is a 413, whole",
        "apps/wipemark-app/src/mcp/server.rs",
        "const LARGEST_BODY: usize = 1024 * 1024;",
        "const LARGEST_BODY: usize = 2 * 1024 * 1024;",
        [(APP + ["--bin", "wipemark"], "an_image_over_the_limit_is_refused_whole")],
    ),
    (
        "M20",
        "--all-metadata reaches the library",
        "apps/wipemark-cli/src/run.rs",
        "                wipemark_image::Scope::AllMetadata\n",
        "                wipemark_image::Scope::AiProvenance\n",
        [(CLI + ["--test", "image"], "all_metadata_removes_exif_and_keeps_colour")],
    ),
    (
        "M21",
        "--in-place sets the original aside",
        "apps/wipemark-cli/src/image.rs",
        "match inplace::replace(file, &bytes, *keep) {",
        "match inplace::replace(file, &bytes, inplace::Keep::Nothing) {",
        [(CLI + ["--test", "image"], "in_place_sets_the_original_aside_and_never_overwrites_one")],
    ),
    (
        "M22",
        "the pixels of a cleaned picture are the pixels it had",
        "apps/wipemark-cli/src/image.rs",
        "    let exit = clean_exit(&report);\n",
        "    let mut bytes = bytes;\n    let mid = bytes.len() / 2;\n    bytes[mid] ^= 0x55;\n    let exit = clean_exit(&report);\n",
        [(CLI + ["--test", "image"], "the_pixels_of_a_cleaned_image_are_the_pixels_it_had")],
    ),
    (
        "M23",
        "the input is never touched",
        "apps/wipemark-cli/src/run.rs",
        "                Some(name) => Destination::Beside(\n                    input.with_file_name(with_infix(&name.to_string_lossy(), RESULT_INFIX)),\n                ),",
        "                Some(_) => Destination::Beside(input.clone()),",
        [(CLI + ["--test", "image"], "clean_writes_beside_the_file_and_leaves_it_untouched")],
    ),
    (
        "M24",
        "every new key is in every language",
        "crates/wipemark-i18n/i18n/ru/wipemark.ftl",
        "image-kind-xmp = XMP\n",
        "",
        [(I18N, "shipped_languages_are_complete")],
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


def check():
    """Every text to mutate is there exactly once — nothing is run."""
    ok = True
    for mid, _, path, old, _, _ in MUTATIONS:
        with open(path, encoding="utf-8") as f:
            n = f.read().count(old)
        if n != 1:
            ok = False
        print(f"{mid}: {path}: {n} match{'es' if n != 1 else ''}")
    return 0 if ok else 1


def main(wanted):
    results = []
    for mid, protection, path, old, new, tests in MUTATIONS:
        if wanted and mid not in wanted:
            continue
        with open(path, encoding="utf-8") as f:
            original = f.read()
        if original.count(old) != 1:
            results.append((mid, protection, "NOT APPLIED: the text to mutate moved", ""))
            continue
        try:
            with open(path, "w", encoding="utf-8") as f:
                f.write(original.replace(old, new))
            red = []
            for args, name in tests:
                bit, compiled, command = run(args, name)
                if not compiled:
                    command += "   <- DID NOT COMPILE"
                red.append((name, bit, command))
        finally:
            with open(path, "w", encoding="utf-8") as f:
                f.write(original)
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
    if sys.argv[1:] == ["--check"]:
        sys.exit(check())
    sys.exit(main(set(sys.argv[1:])))
