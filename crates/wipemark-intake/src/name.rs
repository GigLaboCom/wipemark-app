//! What a *name* says — which is a weaker claim than what the bytes
//! say, and sometimes the only one available.
//!
//! Two questions live here, and they are the two the file-name half of
//! a drop asks:
//!
//! 1. [`of`] — this thing is called `report.docx`; what does that make
//!    it? An extension is a convention, not a fact: anybody can rename
//!    anything. It is still the only evidence there is for a file that
//!    cannot be read, and the only thing that separates Markdown from
//!    CSV from a Rust source file, none of which has a signature.
//! 2. [`path_in`] — a *string* arrived. Is it a path? Dragging out of a
//!    terminal, a file manager's "copy as text", or a chat message
//!    hands over a line of characters that names a file rather than
//!    being the file's contents, and treating `/Users/me/notes.txt` as
//!    a nine-word document would be a scrubber pointed at the wrong
//!    thing entirely.
//!
//! And a third, which is the same vocabulary pointed the other way:
//!
//! 3. [`with_infix`] — what a result made from this name is called.
//!    `report.docx` cleaned is `report.cleaned.docx`, wherever it was
//!    cleaned. A file name is this crate's vocabulary, and the CLI and
//!    the app must spell a result the same way — a script that looks
//!    for `*.cleaned.*` after a drop on the window has to find what the
//!    command line wrote too — so the spelling lives here, where both
//!    applications can reach it, and not in either of them.

use std::path::PathBuf;

use crate::format::Format;

/// Extension to format. Lower-case, without the dot.
///
/// The source-code extensions at the bottom all answer
/// [`Format::PlainText`] on purpose: a `.rs` file is text, the product
/// scrubs text, and a taxonomy of programming languages would be a
/// column nothing reads.
const EXTENSIONS: &[(&str, Format)] = &[
    ("txt", Format::PlainText),
    ("text", Format::PlainText),
    ("log", Format::PlainText),
    ("md", Format::Markdown),
    ("markdown", Format::Markdown),
    ("mdx", Format::Markdown),
    ("html", Format::Html),
    ("htm", Format::Html),
    ("xhtml", Format::Html),
    ("xml", Format::Xml),
    ("plist", Format::Xml),
    ("json", Format::Json),
    ("jsonl", Format::Json),
    ("ndjson", Format::Json),
    ("csv", Format::Csv),
    ("tsv", Format::Csv),
    ("png", Format::Png),
    ("jpg", Format::Jpeg),
    ("jpeg", Format::Jpeg),
    ("jpe", Format::Jpeg),
    ("gif", Format::Gif),
    ("webp", Format::WebP),
    ("tif", Format::Tiff),
    ("tiff", Format::Tiff),
    ("bmp", Format::Bmp),
    ("heic", Format::Heic),
    ("heif", Format::Heic),
    ("avif", Format::Avif),
    ("svg", Format::Svg),
    ("pdf", Format::Pdf),
    ("rtf", Format::Rtf),
    ("docx", Format::Docx),
    ("xlsx", Format::Xlsx),
    ("pptx", Format::Pptx),
    ("doc", Format::Doc),
    ("odt", Format::Odt),
    ("epub", Format::Epub),
    ("zip", Format::Zip),
    ("gz", Format::Gzip),
    ("tgz", Format::Gzip),
    ("bz2", Format::Bzip2),
    ("xz", Format::Xz),
    ("zst", Format::Zstd),
    ("tar", Format::Tar),
    ("7z", Format::SevenZip),
    ("rar", Format::Rar),
    ("mp3", Format::Mp3),
    ("mp4", Format::Mp4),
    ("m4v", Format::Mp4),
    ("mov", Format::Mov),
    ("m4a", Format::M4a),
    ("wav", Format::Wav),
    ("flac", Format::Flac),
    ("ogg", Format::Ogg),
    ("oga", Format::Ogg),
    ("webm", Format::Matroska),
    ("mkv", Format::Matroska),
    ("avi", Format::Avi),
    ("gguf", Format::Gguf),
    ("sqlite", Format::Sqlite),
    ("sqlite3", Format::Sqlite),
    ("db", Format::Sqlite),
    ("rs", Format::PlainText),
    ("py", Format::PlainText),
    ("js", Format::PlainText),
    ("ts", Format::PlainText),
    ("tsx", Format::PlainText),
    ("jsx", Format::PlainText),
    ("c", Format::PlainText),
    ("h", Format::PlainText),
    ("cpp", Format::PlainText),
    ("hpp", Format::PlainText),
    ("go", Format::PlainText),
    ("java", Format::PlainText),
    ("kt", Format::PlainText),
    ("swift", Format::PlainText),
    ("rb", Format::PlainText),
    ("php", Format::PlainText),
    ("sh", Format::PlainText),
    ("bash", Format::PlainText),
    ("zsh", Format::PlainText),
    ("sql", Format::PlainText),
    ("yml", Format::PlainText),
    ("yaml", Format::PlainText),
    ("toml", Format::PlainText),
    ("ini", Format::PlainText),
    ("cfg", Format::PlainText),
    ("conf", Format::PlainText),
    ("env", Format::PlainText),
    ("ftl", Format::PlainText),
    ("srt", Format::PlainText),
    ("vtt", Format::PlainText),
];

/// The infix a result carries: `report.docx` → `report.cleaned.docx`.
///
/// A **format** — never localized, because a shell script that looks
/// for `*.cleaned.*` has to find the file whatever language the window
/// was in — and `mat2`'s spelling, so a person who has used that tool
/// recognises this one's output. The CLI's `--out` help names the same
/// pattern.
pub const RESULT_INFIX: &str = "cleaned";

/// The infix a **rewrite's** result carries: `report.md` →
/// `report.rewritten.md` (В8, E4-6b) — in the windows, the batch queue and
/// the CLI alike.
///
/// Not [`RESULT_INFIX`]: a clean and a rewrite of the same file are two
/// results, and one name for both is a clean and a rewrite overwriting
/// each other. A format, like its sibling.
pub const REWRITTEN_INFIX: &str = "rewritten";

/// The infix a set-aside original carries when a file is replaced:
/// `report.docx` → `report.original.docx`.
///
/// An infix and not ExifTool's `report.docx_original` or `sed`'s
/// `report.docx.bak`, because both of those hide the extension and a
/// copy that Finder cannot open is a copy nobody checks. Kept
/// symmetrical with [`RESULT_INFIX`] on purpose: the two files beside
/// each other read as a pair.
pub const ORIGINAL_INFIX: &str = "original";

/// `report.docx` + `cleaned` → `report.cleaned.docx`.
///
/// The infix goes before the *last* extension and only when there is a
/// stem in front of it: `archive.tar.gz` becomes `archive.tar.cleaned.gz`
/// (which is what `mat2` does too), `README` becomes `README.cleaned`,
/// and `.bashrc` — a name that is all extension — becomes
/// `.bashrc.cleaned` rather than `.cleaned.bashrc`. A name that already
/// carries the infix gets it again: `x.cleaned.md` → `x.cleaned.cleaned.md`,
/// because collapsing it would make the result *the input*, and a
/// destination called "beside" must never write over what it was
/// handed.
pub fn with_infix(name: &str, infix: &str) -> String {
    match name.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() => format!("{stem}.{infix}.{ext}"),
        _ => format!("{name}.{infix}"),
    }
}

/// What the name claims, if anything.
///
/// Takes a file name rather than a path — the caller has already
/// decided which part of a path is the name, and on one platform that
/// decision involves a backslash.
pub fn of(name: &str) -> Option<Format> {
    let lower = name.to_ascii_lowercase();

    // `.tar.gz` is one name for two formats stacked, and the outer one
    // is the one a reader meets first. Same for the other three.
    for (suffix, format) in [
        (".tar.gz", Format::Gzip),
        (".tar.bz2", Format::Bzip2),
        (".tar.xz", Format::Xz),
        (".tar.zst", Format::Zstd),
    ] {
        if lower.ends_with(suffix) {
            return Some(format);
        }
    }

    // A leading dot is a hidden file, not an extension: `.gitignore` is
    // called that, it is not a `gitignore` file. `rsplit_once` on the
    // trimmed name is what keeps them apart.
    let extension = lower.trim_start_matches('.').rsplit_once('.')?.1;
    EXTENSIONS
        .iter()
        .find(|(known, _)| *known == extension)
        .map(|(_, format)| *format)
}

/// The path this line of text names, if it names one.
///
/// Deliberately narrow. One line, and either a `file:` URL or something
/// that begins at the root or at a home directory — because the cost of
/// being wrong is asymmetric: reading a document that happens to say
/// `notes.txt` as a *file* loses the document, while failing to notice
/// a path only means the user drops the file itself instead.
///
/// Whether the path exists is not asked here. That is a question with
/// I/O in it, and this module answers from the string alone.
pub fn path_in(text: &str) -> Option<PathBuf> {
    let line = text.trim();
    if line.is_empty() || line.lines().count() > 1 {
        return None;
    }

    if let Some(rest) = line.strip_prefix("file://") {
        // `file://localhost/x` and `file:///x` both name `/x`; the
        // authority is empty or the local host and nothing else is
        // reachable from here.
        let rest = rest.strip_prefix("localhost").unwrap_or(rest);
        if !rest.starts_with('/') {
            return None;
        }
        return Some(PathBuf::from(unescape(rest)));
    }

    if line.starts_with('/') || line.starts_with("~/") {
        return Some(PathBuf::from(line));
    }
    None
}

/// Percent-decoding, which is the only part of URL syntax a `file:` URL
/// needs — a space in a file name arrives as `%20`, and every name with
/// a Cyrillic letter in it arrives as UTF-8 in percent escapes.
///
/// A stray `%` that is not followed by two hex digits is kept as a
/// literal `%`, because it is one: file names contain the character.
fn unescape(text: &str) -> String {
    let mut out = Vec::with_capacity(text.len());
    let bytes = text.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        let escape = (bytes[index] == b'%')
            .then(|| {
                let pair = text.get(index + 1..index + 3)?;
                u8::from_str_radix(pair, 16).ok()
            })
            .flatten();
        match escape {
            Some(byte) => {
                out.push(byte);
                index += 3;
            }
            None => {
                out.push(bytes[index]);
                index += 1;
            }
        }
    }
    // The bytes came out of a `str` with only whole escapes replaced, so
    // they are UTF-8 unless the sender escaped something that is not —
    // in which case the lossy form is a name that at least points
    // somewhere rather than nowhere.
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{of, path_in, with_infix, ORIGINAL_INFIX, RESULT_INFIX, REWRITTEN_INFIX};
    use crate::format::Format;

    #[test]
    fn a_result_is_named_after_its_file() {
        assert_eq!(
            with_infix("report.docx", RESULT_INFIX),
            "report.cleaned.docx"
        );
        assert_eq!(
            with_infix("archive.tar.gz", RESULT_INFIX),
            "archive.tar.cleaned.gz"
        );
        assert_eq!(with_infix("README", RESULT_INFIX), "README.cleaned");
        assert_eq!(with_infix(".bashrc", RESULT_INFIX), ".bashrc.cleaned");
        assert_eq!(
            with_infix("photo.original.png", ORIGINAL_INFIX),
            "photo.original.original.png"
        );
    }

    #[test]
    fn the_infixes_are_formats_and_stay_ascii() {
        for infix in [RESULT_INFIX, REWRITTEN_INFIX, ORIGINAL_INFIX] {
            assert!(infix.is_ascii() && !infix.contains('.'), "{infix:?}");
        }
        assert_ne!(RESULT_INFIX, ORIGINAL_INFIX);
        // A clean and a rewrite of one file are two results under two
        // names (В8), and neither is the set-aside original.
        assert_ne!(RESULT_INFIX, REWRITTEN_INFIX);
        assert_ne!(REWRITTEN_INFIX, ORIGINAL_INFIX);
        assert_eq!(
            with_infix("article.md", REWRITTEN_INFIX),
            "article.rewritten.md"
        );
    }

    /// The one case where collapsing would be wrong, for every shape a
    /// name can take: whatever it is given, the answer is a different
    /// name — so a result written "beside" can never land on its input.
    /// The app's `a_result_beside_a_file_is_never_the_file_itself` and
    /// the CLI's own tests rest on this.
    #[test]
    fn with_infix_never_returns_the_name_it_was_given() {
        for name in [
            "x.cleaned.md",
            "x.md",
            "x",
            ".x",
            "x.",
            "a.b.c",
            "",
            "x.cleaned",
            "заметки.cleaned.md",
        ] {
            for infix in [RESULT_INFIX, REWRITTEN_INFIX, ORIGINAL_INFIX] {
                assert_ne!(with_infix(name, infix), name, "{name:?} + {infix:?}");
            }
        }
    }

    #[test]
    fn an_extension_is_matched_whatever_its_case() {
        assert_eq!(of("Report.DOCX"), Some(Format::Docx));
        assert_eq!(of("photo.JPG"), Some(Format::Jpeg));
        assert_eq!(of("notes.md"), Some(Format::Markdown));
        assert_eq!(of("main.rs"), Some(Format::PlainText));
    }

    /// Two formats stacked have one name, and the outer one is what a
    /// reader meets first. `.tar` alone still answers `tar`.
    #[test]
    fn a_doubled_extension_names_the_outer_format() {
        assert_eq!(of("weights.tar.gz"), Some(Format::Gzip));
        assert_eq!(of("weights.tar.zst"), Some(Format::Zstd));
        assert_eq!(of("weights.tar"), Some(Format::Tar));
        assert_eq!(of("weights.tgz"), Some(Format::Gzip));
    }

    /// A dotfile is a name, not an extension — `.gitignore` is not a
    /// file of type `gitignore`, and `.env` in the middle of a name is
    /// a different thing from a file called `.env`.
    #[test]
    fn a_hidden_file_is_not_an_extension() {
        assert_eq!(of(".gitignore"), None);
        assert_eq!(of(".env"), None);
        assert_eq!(of("local.env"), Some(Format::PlainText));
        assert_eq!(of("README"), None);
    }

    #[test]
    fn a_line_that_names_a_file_is_recognised_as_one() {
        assert_eq!(
            path_in("/Users/me/notes.txt"),
            Some(PathBuf::from("/Users/me/notes.txt"))
        );
        assert_eq!(
            path_in("  file:///Users/me/my%20notes.txt\n"),
            Some(PathBuf::from("/Users/me/my notes.txt"))
        );
        assert_eq!(
            path_in("file://localhost/tmp/a.png"),
            Some(PathBuf::from("/tmp/a.png"))
        );
    }

    /// The narrowness is the feature. A document that mentions a file
    /// is a document, and reading it as a path would lose it.
    #[test]
    fn prose_that_merely_contains_a_name_is_not_a_path() {
        assert_eq!(path_in("see notes.txt for the rest"), None);
        assert_eq!(path_in("/Users/me/a.txt\n/Users/me/b.txt"), None);
        assert_eq!(path_in("https://example.com/a.txt"), None);
        assert_eq!(path_in(""), None);
    }

    /// A percent sign is a legal character in a file name, and a `%`
    /// that is not an escape has to survive as itself.
    #[test]
    fn an_escape_that_is_not_one_stays_a_percent_sign() {
        assert_eq!(
            path_in("file:///tmp/100%25.txt"),
            Some(PathBuf::from("/tmp/100%.txt"))
        );
        assert_eq!(
            path_in("file:///tmp/50%off.txt"),
            Some(PathBuf::from("/tmp/50%off.txt"))
        );
        assert_eq!(
            path_in("file:///tmp/%D0%BF%D1%80%D0%B8%D0%B2%D0%B5%D1%82.txt"),
            Some(PathBuf::from("/tmp/привет.txt"))
        );
    }
}
