//! What a thing *is*, at two levels of precision.
//!
//! [`Kind`] is the level a person is told about and the level the
//! product acts on: this is text, and Layer A can scrub it; this is an
//! image, and epic E11 will; this is a film, and nothing here will ever
//! touch it. [`Format`] is the level underneath — the actual container,
//! which is what decides *how* the kind gets opened.
//!
//! # Neither of them is prose
//!
//! [`Format::name`] returns `"PNG"`, not "Portable Network Graphics",
//! and never a translated string: a format name is a proper noun, the
//! same category as a `Vendor` name or a config key, and translating one
//! would strand a user who searched for it. [`Kind`] carries no text at
//! all — it is an enum, and the surface that draws it decides what to
//! call it in the language it is drawing in. That is the rule for every
//! library in this workspace and the reason none of them may depend on
//! `wipemark-i18n`.

/// What the thing is, at the level the product acts on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Kind {
    /// Characters. Layer A's whole subject: this is the one kind the
    /// deterministic scrubber has anything to say about today.
    Text,
    /// Pixels in a container that also carries metadata — which is
    /// where provenance marks live. Epic E11.
    Image,
    /// Text inside a container: a PDF, a DOCX, an ODT. There is text in
    /// here, and getting at it is not reading the file.
    Document,
    /// Files inside a file. Somebody dropping a ZIP means the things
    /// inside it, and unpacking one is a decision with a directory
    /// traversal rule attached, not a detail.
    Archive,
    /// Sound and film. Named rather than lumped in with the unknown,
    /// because "this product does nothing with video" is a better
    /// answer than "unrecognised".
    Media,
    /// Recognised, and not a document by any reading — a model's
    /// weights, a database. Being specific about these is what lets the
    /// surface say "that is a model file" instead of shrugging.
    Data,
    /// A directory. The one kind that is established by the filesystem
    /// rather than by any byte.
    Folder,
    /// The bytes were read and they place it nowhere. Not an error: most
    /// of the formats in the world are not in the table below, and
    /// saying so is the honest answer.
    Unknown,
}

/// The container, when one was recognised.
///
/// Everything here is either something this product will eventually
/// open, or something a person plausibly drops on a window and deserves
/// a straight answer about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Format {
    // ── text ────────────────────────────────────────────────────────
    /// Characters with no further structure claimed. The fallback for
    /// anything that reads as text, source code included.
    PlainText,
    Markdown,
    Html,
    Xml,
    Json,
    Csv,
    /// A single line that is a URL. It arrived as text and it is text,
    /// but what a person means by dropping one is the page, which this
    /// product does not fetch.
    Url,

    // ── images ──────────────────────────────────────────────────────
    Png,
    Jpeg,
    Gif,
    WebP,
    Tiff,
    Bmp,
    Heic,
    Avif,
    /// XML that draws. The one format in this table whose bytes are
    /// text and whose kind is [`Kind::Image`].
    Svg,

    // ── documents ───────────────────────────────────────────────────
    Pdf,
    Rtf,
    /// A ZIP whose first entry is `[Content_Types].xml`: an Office Open
    /// XML document, without yet saying which of the three. Which one is
    /// in the central directory at the far end of the file, and the head
    /// this crate is handed does not reach it — so the *name* answers
    /// that, and [`Format::refines`] is how.
    Office,
    Docx,
    Xlsx,
    Pptx,
    /// The pre-2007 Word format: an OLE2 compound file.
    Doc,
    Odt,
    Epub,

    // ── archives ────────────────────────────────────────────────────
    Zip,
    Gzip,
    Bzip2,
    Xz,
    Zstd,
    Tar,
    SevenZip,
    Rar,

    // ── media ───────────────────────────────────────────────────────
    Mp3,
    Mp4,
    Mov,
    M4a,
    Wav,
    Flac,
    Ogg,
    /// Matroska, which is also what a `.webm` is.
    Matroska,
    Avi,

    // ── data ────────────────────────────────────────────────────────
    /// The weights format this product downloads. Somebody dropping a
    /// 4 GB `.gguf` on the panel is asking a question worth answering
    /// precisely.
    Gguf,
    Sqlite,
}

impl Format {
    /// What acting on this format would mean.
    pub fn kind(self) -> Kind {
        use Format::{
            Avi, Avif, Bmp, Bzip2, Csv, Doc, Docx, Epub, Flac, Gguf, Gif, Gzip, Heic, Html, Jpeg,
            Json, M4a, Markdown, Matroska, Mov, Mp3, Mp4, Odt, Office, Ogg, Pdf, PlainText, Png,
            Pptx, Rar, Rtf, SevenZip, Sqlite, Svg, Tar, Tiff, Url, Wav, WebP, Xlsx, Xml, Xz, Zip,
            Zstd,
        };
        match self {
            PlainText | Markdown | Html | Xml | Json | Csv | Url => Kind::Text,
            Png | Jpeg | Gif | WebP | Tiff | Bmp | Heic | Avif | Svg => Kind::Image,
            Pdf | Rtf | Office | Docx | Xlsx | Pptx | Doc | Odt | Epub => Kind::Document,
            Zip | Gzip | Bzip2 | Xz | Zstd | Tar | SevenZip | Rar => Kind::Archive,
            Mp3 | Mp4 | Mov | M4a | Wav | Flac | Ogg | Matroska | Avi => Kind::Media,
            Gguf | Sqlite => Kind::Data,
        }
    }

    /// The name people write on a whiteboard. Never localized — see the
    /// module docs.
    pub fn name(self) -> &'static str {
        use Format::{
            Avi, Avif, Bmp, Bzip2, Csv, Doc, Docx, Epub, Flac, Gguf, Gif, Gzip, Heic, Html, Jpeg,
            Json, M4a, Markdown, Matroska, Mov, Mp3, Mp4, Odt, Office, Ogg, Pdf, PlainText, Png,
            Pptx, Rar, Rtf, SevenZip, Sqlite, Svg, Tar, Tiff, Url, Wav, WebP, Xlsx, Xml, Xz, Zip,
            Zstd,
        };
        match self {
            PlainText => "Text",
            Markdown => "Markdown",
            Html => "HTML",
            Xml => "XML",
            Json => "JSON",
            Csv => "CSV",
            Url => "URL",
            Png => "PNG",
            Jpeg => "JPEG",
            Gif => "GIF",
            WebP => "WebP",
            Tiff => "TIFF",
            Bmp => "BMP",
            Heic => "HEIC",
            Avif => "AVIF",
            Svg => "SVG",
            Pdf => "PDF",
            Rtf => "RTF",
            Office => "Office Open XML",
            Docx => "DOCX",
            Xlsx => "XLSX",
            Pptx => "PPTX",
            Doc => "DOC",
            Odt => "ODT",
            Epub => "EPUB",
            Zip => "ZIP",
            Gzip => "gzip",
            Bzip2 => "bzip2",
            Xz => "xz",
            Zstd => "Zstandard",
            Tar => "tar",
            SevenZip => "7z",
            Rar => "RAR",
            Mp3 => "MP3",
            Mp4 => "MP4",
            Mov => "QuickTime",
            M4a => "M4A",
            Wav => "WAV",
            Flac => "FLAC",
            Ogg => "Ogg",
            Matroska => "Matroska",
            Avi => "AVI",
            Gguf => "GGUF",
            Sqlite => "SQLite",
        }
    }

    /// The media type, for the surfaces that speak in them — the MCP
    /// server's reports and the CLI's `--json`.
    pub fn mime(self) -> &'static str {
        use Format::{
            Avi, Avif, Bmp, Bzip2, Csv, Doc, Docx, Epub, Flac, Gguf, Gif, Gzip, Heic, Html, Jpeg,
            Json, M4a, Markdown, Matroska, Mov, Mp3, Mp4, Odt, Office, Ogg, Pdf, PlainText, Png,
            Pptx, Rar, Rtf, SevenZip, Sqlite, Svg, Tar, Tiff, Url, Wav, WebP, Xlsx, Xml, Xz, Zip,
            Zstd,
        };
        match self {
            PlainText => "text/plain",
            Markdown => "text/markdown",
            Html => "text/html",
            Xml => "application/xml",
            Json => "application/json",
            Csv => "text/csv",
            Url => "text/uri-list",
            Png => "image/png",
            Jpeg => "image/jpeg",
            Gif => "image/gif",
            WebP => "image/webp",
            Tiff => "image/tiff",
            Bmp => "image/bmp",
            Heic => "image/heic",
            Avif => "image/avif",
            Svg => "image/svg+xml",
            Pdf => "application/pdf",
            Rtf => "application/rtf",
            Office | Zip => "application/zip",
            Docx => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            Xlsx => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            Pptx => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
            Doc => "application/msword",
            Odt => "application/vnd.oasis.opendocument.text",
            Epub => "application/epub+zip",
            Gzip => "application/gzip",
            Bzip2 => "application/x-bzip2",
            Xz => "application/x-xz",
            Zstd => "application/zstd",
            Tar => "application/x-tar",
            SevenZip => "application/x-7z-compressed",
            Rar => "application/vnd.rar",
            Mp3 => "audio/mpeg",
            Mp4 => "video/mp4",
            Mov => "video/quicktime",
            M4a => "audio/mp4",
            Wav => "audio/wav",
            Flac => "audio/flac",
            Ogg => "application/ogg",
            Matroska => "video/x-matroska",
            Avi => "video/x-msvideo",
            Gguf => "application/octet-stream",
            Sqlite => "application/vnd.sqlite3",
        }
    }

    /// Whether the bytes of this format are characters.
    ///
    /// Not the same question as [`Kind::Text`]: an [`Format::Svg`] is an
    /// image made of text, and an [`Format::Rtf`] is a document made of
    /// text. This is what decides whether the head can be decoded, and
    /// [`Kind::kind`](Format::kind) is what decides what the product
    /// does with it.
    pub fn is_textual(self) -> bool {
        use Format::{Csv, Html, Json, Markdown, PlainText, Rtf, Svg, Url, Xml};
        matches!(
            self,
            PlainText | Markdown | Html | Xml | Json | Csv | Url | Svg | Rtf
        )
    }

    /// Whether `self` is a more specific answer to the same question as
    /// `other`.
    ///
    /// This is the whole reason a name is allowed to beat a signature
    /// without that being a disagreement. Every ZIP-based document
    /// *is* a ZIP; every Office Open XML file *is* a ZIP whose first
    /// entry says so; every Markdown file *is* text. When the bytes can
    /// only reach the general answer and the name offers the specific
    /// one underneath it, they agree — nobody has been contradicted.
    ///
    /// A name that offers something the signature does not sit above is
    /// a different matter entirely, and [`crate::Evidence::Disagreed`]
    /// is where that goes.
    pub fn refines(self, other: Format) -> bool {
        let mut step = self;
        while let Some(broader) = step.broader() {
            if broader == other {
                return true;
            }
            step = broader;
        }
        false
    }

    /// The next less specific answer this format is a case of.
    fn broader(self) -> Option<Format> {
        use Format::{
            Csv, Docx, Epub, Html, Json, Markdown, Odt, Office, PlainText, Pptx, Svg, Url, Xlsx,
            Xml, Zip,
        };
        match self {
            Markdown | Html | Xml | Json | Csv | Url => Some(PlainText),
            Svg => Some(Xml),
            Docx | Xlsx | Pptx => Some(Office),
            Office | Odt | Epub => Some(Zip),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Format, Kind};

    /// The table is walked rather than spot-checked: every format has to
    /// answer all four questions, and a variant added without a `mime`
    /// or a `name` is a `match` arm somebody forgot.
    const ALL: [Format; 44] = [
        Format::PlainText,
        Format::Markdown,
        Format::Html,
        Format::Xml,
        Format::Json,
        Format::Csv,
        Format::Url,
        Format::Png,
        Format::Jpeg,
        Format::Gif,
        Format::WebP,
        Format::Tiff,
        Format::Bmp,
        Format::Heic,
        Format::Avif,
        Format::Svg,
        Format::Pdf,
        Format::Rtf,
        Format::Office,
        Format::Docx,
        Format::Xlsx,
        Format::Pptx,
        Format::Doc,
        Format::Odt,
        Format::Epub,
        Format::Zip,
        Format::Gzip,
        Format::Bzip2,
        Format::Xz,
        Format::Zstd,
        Format::Tar,
        Format::SevenZip,
        Format::Rar,
        Format::Mp3,
        Format::Mp4,
        Format::Mov,
        Format::M4a,
        Format::Wav,
        Format::Flac,
        Format::Ogg,
        Format::Matroska,
        Format::Avi,
        Format::Gguf,
        Format::Sqlite,
    ];

    #[test]
    fn every_format_has_a_name_and_a_media_type() {
        for format in ALL {
            assert!(!format.name().is_empty(), "{format:?} has no name");
            assert!(
                format.mime().contains('/'),
                "{format:?}: {:?} is not a media type",
                format.mime()
            );
        }
    }

    /// A format that is text on disk and an image to the product is
    /// exactly the case that makes these two questions different ones.
    #[test]
    fn an_svg_is_an_image_made_of_text() {
        assert_eq!(Format::Svg.kind(), Kind::Image);
        assert!(Format::Svg.is_textual());
        assert_eq!(Format::Png.kind(), Kind::Image);
        assert!(!Format::Png.is_textual());
    }

    /// The lattice the arbitration rests on: a name that lands
    /// underneath what the bytes could see is not a contradiction.
    #[test]
    fn a_more_specific_answer_refines_the_general_one() {
        assert!(Format::Docx.refines(Format::Office));
        assert!(Format::Docx.refines(Format::Zip));
        assert!(Format::Odt.refines(Format::Zip));
        assert!(Format::Markdown.refines(Format::PlainText));
        assert!(Format::Svg.refines(Format::Xml));
        assert!(Format::Svg.refines(Format::PlainText));
    }

    /// And the other half, which is what keeps `refines` from being a
    /// way for any name to overrule any signature: two documents that
    /// are both ZIPs are still two different documents, nothing refines
    /// itself, and the relation only ever points one way.
    #[test]
    fn two_formats_side_by_side_never_refine_each_other() {
        assert!(!Format::Odt.refines(Format::Office));
        assert!(!Format::Docx.refines(Format::Xlsx));
        assert!(!Format::Png.refines(Format::Jpeg));
        assert!(!Format::Zip.refines(Format::Docx));
        assert!(!Format::PlainText.refines(Format::Markdown));
        for format in ALL {
            assert!(!format.refines(format), "{format:?} refines itself");
        }
    }
}
