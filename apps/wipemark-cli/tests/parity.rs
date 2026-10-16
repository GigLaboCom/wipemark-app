//! The CLI's half of the windows' parity table (E7 follow-ups, W9, D285).
//!
//! `fixtures/clean-parity/table.tsv` says, per input, what
//! `wipemark-cli clean <input> -o <out>` exits with and whether it writes;
//! the application's `clean::tests::the_windows_clean_to_the_clis_table`
//! reads the same table for what a window does. This test runs the real
//! binary over every row, so a change to the CLI's exit or write rule is
//! red here until the table says so — and then red in the application's
//! test until a window follows. What is written must be the library's own
//! answer at the CLI's defaults: Layer A at `Options::default()` in the
//! encoding the text came in, or the picture passes at
//! `Scope::AiProvenance` (no `--all-metadata`).

use std::path::{Path, PathBuf};
use std::process::Command;

use wipemark_intake::Kind;
use wipemark_picture::PictureOptions;

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

/// One row of the table.
struct Row {
    input: String,
    /// A file already where the result goes beside the input (D598): the
    /// CLI refuses it, where a window writes the row's own name (D635).
    taken: bool,
    cli_exit: i32,
    cli_writes: bool,
}

/// What is already where a `taken` row's result would go.
const SOMEONES: &[u8] = b"an edit saved in Compare\n";

fn table() -> Vec<Row> {
    let table = std::fs::read_to_string(fixtures().join("clean-parity/table.tsv")).expect("table");
    table
        .lines()
        .filter(|line| !line.starts_with('#') && !line.trim().is_empty())
        .map(|line| {
            let columns: Vec<&str> = line.split('\t').collect();
            // input, place, cli_exit, cli_writes, then the window's two
            // columns, read past here, and the person's.
            assert_eq!(columns.len(), 7, "a row of seven columns: {line:?}");
            Row {
                input: columns[0].to_owned(),
                taken: columns[1] == "taken",
                cli_exit: columns[2].parse().expect("an exit code"),
                cli_writes: columns[3] == "yes",
            }
        })
        .collect()
}

/// What the libraries make of `bytes` at the CLI's defaults.
fn library(bytes: &[u8], name: &str) -> Vec<u8> {
    let intake = wipemark_intake::of_bytes(bytes, Some(name));
    if intake.kind == Kind::Image {
        let options = PictureOptions {
            scope: wipemark_image::Scope::AiProvenance,
            catalogue: None,
        };
        return wipemark_picture::clean(bytes, &options)
            .expect("the picture passes")
            .0;
    }
    let encoding = intake.encoding.expect("an encoding");
    let text = wipemark_intake::text::decode(bytes, encoding).expect("decodes");
    let cleaned = wipemark_core::clean(&text, &wipemark_core::Options::default()).text;
    wipemark_intake::text::encode(&cleaned, encoding)
}

#[test]
fn the_cli_cleans_to_the_windows_table() {
    let out = std::env::temp_dir().join(format!("wipemark-cli-parity-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).expect("scratch");
    let rows = table();
    assert!(rows.len() >= 12, "the table lost rows");
    for row in rows {
        let input = fixtures().join(&row.input);
        let name = Path::new(&row.input)
            .file_name()
            .expect("a name")
            .to_string_lossy()
            .into_owned();
        if row.taken {
            taken(&row, &input, &name, &out);
            continue;
        }
        let result = out.join(&name);
        let run = Command::new(env!("CARGO_BIN_EXE_wipemark-cli"))
            .arg("clean")
            .arg(&input)
            .arg("-o")
            .arg(&result)
            .env("WIPEMARK_DATA_DIR", out.join("data"))
            .env("WIPEMARK_LANG", "en-US")
            .env_remove("WIPEMARK_LOG")
            .output()
            .expect("the CLI runs");
        assert_eq!(
            run.status.code(),
            Some(row.cli_exit),
            "{}: the CLI's exit is not the table's: {}",
            row.input,
            String::from_utf8_lossy(&run.stderr)
        );
        assert_eq!(
            result.exists(),
            row.cli_writes,
            "{}: the CLI's write is not the table's",
            row.input
        );
        if row.cli_writes {
            let bytes = std::fs::read(&input).expect("the input");
            assert_eq!(
                std::fs::read(&result).expect("the result"),
                library(&bytes, &name),
                "{}: not the library's answer at the CLI's defaults",
                row.input
            );
        }
    }
    std::fs::remove_dir_all(&out).ok();
}

/// A `taken` row: the input copied into a folder of its own with a file
/// already where its result goes, cleaned beside it with no `-o`.
fn taken(row: &Row, input: &Path, name: &str, out: &Path) {
    let folder = out.join(format!("taken-{name}"));
    std::fs::create_dir_all(&folder).expect("a folder");
    let copy = folder.join(name);
    std::fs::copy(input, &copy).expect("the input");
    let beside = folder.join(wipemark_intake::name::with_infix(
        name,
        wipemark_intake::name::RESULT_INFIX,
    ));
    std::fs::write(&beside, SOMEONES).expect("someone's file");
    let run = Command::new(env!("CARGO_BIN_EXE_wipemark-cli"))
        .arg("clean")
        .arg(&copy)
        .env("WIPEMARK_DATA_DIR", out.join("data"))
        .env("WIPEMARK_LANG", "en-US")
        .env_remove("WIPEMARK_LOG")
        .output()
        .expect("the CLI runs");
    assert_eq!(
        run.status.code(),
        Some(row.cli_exit),
        "{} (taken): the CLI's exit is not the table's: {}",
        row.input,
        String::from_utf8_lossy(&run.stderr)
    );
    let left = std::fs::read(&beside).expect("the file beside") == SOMEONES;
    assert_eq!(
        !left, row.cli_writes,
        "{} (taken): the CLI's write is not the table's",
        row.input
    );
    // A window writes the row's own `name.cleaned-<id>.ext` here (D635);
    // the command line writes nothing at all — no name of its own either.
    if !row.cli_writes {
        let entries = std::fs::read_dir(&folder).expect("the folder").count();
        assert_eq!(
            entries, 2,
            "{} (taken): the CLI wrote a file under another name",
            row.input
        );
    }
}
