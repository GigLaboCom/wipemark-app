//! What is written, where, and what is kept — the Retention page's
//! vocabulary, and the one place its rules are stated.
//!
//! Two questions, and they have different shapes of answer.
//!
//! **Where a result goes, and what happens to the file it came from.**
//! One answer for the whole product, [`Destination`], and it follows
//! the tool this product most resembles: `mat2` writes
//! `name.cleaned.ext` beside the file and touches the file itself only
//! under an explicit `--inplace`, which is also what spec §4.5 asks
//! for. Replacing a file is offered — a cleaned file with the same
//! name is what a document with links pointing at it needs — but it is
//! never destructive: the original is set aside as `name.original.ext`
//! first, and an original already there is never overwritten, which is
//! ExifTool's `_original` rule. The "no copy at all" that a batch of
//! five hundred files under version control wants is a per-run flag
//! for the CLI and the batch queue to carry, not a preference: a row
//! that deletes originals is a landmine that goes off months after it
//! was set.
//!
//! **Whether Wipemark keeps a copy of its own.** A file never needs
//! one: the file is the original, and its result is written to disk.
//! What has no file behind it — text pasted, an image dragged out of a
//! browser, the text an agent hands the MCP server — exists nowhere
//! else once the result has replaced it, so the only place it can be
//! brought back from is `Layout::kept_dir`. Both switches are
//! **off** by default, and the period is bounded by default, because
//! a product whose purpose is removing provenance must not quietly
//! build an archive of marked originals in a folder the platform
//! hides; keeping is asked for, visible on the page, and bounded.
//!
//! The rule that answers the question this module was written to
//! settle — *is a Markdown or an HTML paste kept any differently?* —
//! is that **the format never decides whether a copy is kept**. What
//! decides is whether the original survives somewhere else once the
//! result exists — a file does, a paste does not — and nothing else:
//! not the container, and not the layer, since `CleanReport` offers no
//! byte-exact way back either. The format decides only what the copy
//! *is*: bytes as
//! they arrived, markup included, never the text extracted from them —
//! the same rule `wipemark-image` keeps for pixels. An HTML original is
//! where provenance hides (`<meta name="generator">`, a comment, an
//! attribute), so a copy that had been flattened to text would be a
//! copy of the wrong thing. [`Source::of`] is where the first rule is
//! structural — it reads nothing but how the thing arrived — and
//! `a_markdown_paste_and_a_plain_one_are_planned_alike` is the gate.
//!
//! Everything here is a function over values. Nothing opens the
//! database or touches the disk: the page gathers the rows and the
//! folders, and `plan` says what would happen. Nothing *does* happen
//! yet — cleaning is E1 and the batch is E4 — and the banner says so.
//! See `docs/architecture/retention.md`.

use std::path::{Path, PathBuf};

use wipemark_i18n::{t, Message};
/// The result and set-aside names. They live in `wipemark_intake::name`
/// because the CLI writes results too and must not depend on this crate
/// (D10); re-exported so every caller here keeps its spelling.
pub use wipemark_intake::name::{with_infix, ORIGINAL_INFIX, RESULT_INFIX};
use wipemark_intake::{Intake, Kind};

use crate::engine::Choice;

/// Where a result goes, and what happens to the file it came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Destination {
    /// `name.cleaned.ext` beside the file; the file itself is never
    /// touched. The default, and `mat2`'s.
    #[default]
    Beside,
    /// Into the results folder, under the same name; the file itself
    /// is never touched.
    Folder,
    /// Over the file — once the original has been set aside as
    /// `name.original.ext`, and never over an original already there.
    Replace,
}

impl Destination {
    /// Every choice, in the order the page lists them: the default
    /// first, the destructive-looking one last.
    pub const ALL: [Destination; 3] = [Self::Beside, Self::Folder, Self::Replace];

    /// The stored value. A format: never translated, never derived
    /// from the label.
    pub fn id(self) -> &'static str {
        match self {
            Self::Beside => "beside",
            Self::Folder => "folder",
            Self::Replace => "replace",
        }
    }

    /// Read a stored value back. `None` for anything else — the
    /// caller's answer is the default and the row left alone, the
    /// bargain every other preference makes.
    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|choice| choice.id() == value.trim())
    }

    /// The radio button's label.
    pub fn label(self) -> Message {
        match self {
            Self::Beside => Message::SettingsRetentionDestinationBeside,
            Self::Folder => Message::SettingsRetentionDestinationFolder,
            Self::Replace => Message::SettingsRetentionDestinationReplace,
        }
    }
}

/// How long a kept copy stays.
///
/// Fixed choices rather than a number of days, the way Safari's
/// "Remove history items" is: a dropdown can tick one of five and
/// cannot tick `14d`, and a preference the control cannot show is a
/// preference the user cannot see. The stored spelling is `<n>d` for
/// the four that end and `forever` for the one that does not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Period {
    Day,
    #[default]
    Week,
    Month,
    Quarter,
    /// Until removed by hand. Offered because a bounded default is
    /// the right default and not the only reasonable answer; a person
    /// who wants an archive should be able to say so out loud.
    Forever,
}

impl Period {
    /// Every choice, shortest first.
    pub const ALL: [Period; 5] = [
        Self::Day,
        Self::Week,
        Self::Month,
        Self::Quarter,
        Self::Forever,
    ];

    /// The stored value. A format.
    pub fn id(self) -> &'static str {
        match self {
            Self::Day => "1d",
            Self::Week => "7d",
            Self::Month => "30d",
            Self::Quarter => "90d",
            Self::Forever => "forever",
        }
    }

    /// Read a stored value back; `None` for anything this build does
    /// not offer.
    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|choice| choice.id() == value.trim())
    }

    /// The dropdown's label: "A week".
    pub fn label(self) -> Message {
        match self {
            Self::Day => Message::SettingsRetentionPeriodDay,
            Self::Week => Message::SettingsRetentionPeriodWeek,
            Self::Month => Message::SettingsRetentionPeriodMonth,
            Self::Quarter => Message::SettingsRetentionPeriodQuarter,
            Self::Forever => Message::SettingsRetentionPeriodForever,
        }
    }

    /// The same span inside a sentence: "kept *for a week*". A second
    /// message rather than the label lower-cased, because Fluent cannot
    /// lower-case and a translator may need a different case entirely.
    pub fn span(self) -> Message {
        match self {
            Self::Day => Message::SettingsRetentionSpanDay,
            Self::Week => Message::SettingsRetentionSpanWeek,
            Self::Month => Message::SettingsRetentionSpanMonth,
            Self::Quarter => Message::SettingsRetentionSpanQuarter,
            Self::Forever => Message::SettingsRetentionSpanForever,
        }
    }
}

/// The rows of the period selector.
pub fn period_choices() -> Vec<Choice<Period>> {
    Period::ALL
        .into_iter()
        .map(|period| Choice::new(period, t(period.label()), period.id()))
        .collect()
}

/// The Retention page, as values — every row on it.
///
/// One struct rather than five loose values, for the reason
/// `EngineSettings` is one: nothing reads the destination without
/// needing to know which folder it means, and the two switches mean
/// nothing without the period beside them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Retention {
    pub destination: Destination,
    /// The results folder, when a row names one. `None` is the
    /// platform's Downloads folder — see [`Homes`].
    pub folder: Option<PathBuf>,
    /// Whether the original of something that arrived without a file
    /// is kept in Wipemark's own folder.
    pub keep_originals: bool,
    /// Whether its result is.
    pub keep_results: bool,
    /// For how long, when either is.
    pub keep_for: Period,
}

impl Default for Retention {
    /// Beside the file, nothing kept, a week if something were.
    ///
    /// The two switches are off because keeping is the exception —
    /// see the module docs — and the period is a week rather than
    /// forever because a bounded default is what every product that
    /// keeps history on a person's behalf has settled on.
    fn default() -> Self {
        Self {
            destination: Destination::Beside,
            folder: None,
            keep_originals: false,
            keep_results: false,
            keep_for: Period::Week,
        }
    }
}

impl Retention {
    /// Whether anything is kept at all.
    pub fn keeps(&self) -> bool {
        self.keep_originals || self.keep_results
    }
}

/// The two folders the page cannot name from the database alone.
///
/// Gathered by `main` once and handed to `Preferences`, for the reason
/// `models_default` is: one of them asks the platform and the other is
/// the data directory, and neither is a question a test wants asked of
/// the real machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Homes {
    /// Where a result goes when no row says otherwise: the platform's
    /// Downloads folder, or the best stand-in this machine has.
    pub results: PathBuf,
    /// Where kept copies go: `Layout::kept_dir`, which no row moves.
    pub kept: PathBuf,
}

impl Homes {
    /// Discover both.
    ///
    /// A platform with no Downloads folder falls back to the home
    /// directory, and one with no home falls back to the temporary
    /// directory — the same ladder the models folder climbs — because
    /// a page has to name *somewhere* and a download that refuses on
    /// write is better than a page that fails to open. `kept` follows
    /// the data directory wherever `WIPEMARK_DATA_DIR` points it.
    pub fn discover(layout: Option<&wipemark_models::layout::Layout>) -> Self {
        let results = wipemark_models::layout::downloads_dir()
            .or_else(|| std::env::var_os("HOME").map(PathBuf::from))
            .unwrap_or_else(std::env::temp_dir);
        let kept = match layout {
            Some(layout) => layout.kept_dir(),
            None => std::env::temp_dir().join("wipemark-kept"),
        };
        Self { results, kept }
    }
}

/// What arrived, reduced to the two facts that decide where its result
/// goes: whether there is a file behind it, and what it is called.
///
/// Deliberately **not** the format. A Markdown paste and a plain one
/// are the same `Text`; a PNG on disk and a DOCX on disk are the same
/// `File`. That is the rule in the module docs made structural: a
/// reader of this type cannot branch on a format it was never given.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// A file on disk. The file is the original.
    File(PathBuf),
    /// A folder on disk. Every file in it is a [`Source::File`], and
    /// the batch plans each one; this is what a *drop* of one means.
    Folder(PathBuf),
    /// Characters that arrived as characters. The result goes back the
    /// same way, and no file is written unless somebody asks.
    Text,
    /// Bytes with no file behind them — an image dragged out of a
    /// browser, a screenshot off the clipboard — and the name the
    /// sender attached, which is often nothing.
    Bytes { name: Option<String> },
}

impl Source {
    /// Which of the four this is.
    ///
    /// Keyed on the *path*, not on how it arrived: a line of text that
    /// names a file that exists is that file — `wipemark_intake::of_text`
    /// already decided so — and a plan that wrote the result beside
    /// nineteen characters instead of beside the document would be the
    /// wrong file, silently.
    pub fn of(intake: &Intake) -> Self {
        match (&intake.path, intake.kind) {
            (Some(path), Kind::Folder) => Self::Folder(path.clone()),
            (Some(path), _) => Self::File(path.clone()),
            (None, _) => match intake.arrived {
                wipemark_intake::Arrived::AsText => Self::Text,
                wipemark_intake::Arrived::AsBytes | wipemark_intake::Arrived::AsPath => {
                    Self::Bytes {
                        name: intake.name.clone(),
                    }
                }
            },
        }
    }
}

/// What would happen to one thing that arrived, given the page's rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Plan {
    /// A file: where its result goes, which is also what happens to it.
    /// Nothing about keeping — a file is never copied into Wipemark's
    /// folder, and the absence of the field is that rule stated in the
    /// type.
    File(Written),
    /// A folder: every file in it is planned as a [`Plan::File`].
    EachFileIn(PathBuf),
    /// Nothing on disk. The result goes back the way the original
    /// came, and Wipemark's own folder is the only place either can be
    /// kept.
    Loose { result: Written, kept: Option<Kept> },
}

/// Where a result goes — and, for a file, what that does to the file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Written {
    /// To this path, beside the file it came from. The file is left
    /// exactly as it is.
    Beside(PathBuf),
    /// Into this folder, under this name — or under one the job makes
    /// up, when the thing arrived without a name. A file it came from
    /// is left exactly as it is.
    Into {
        folder: PathBuf,
        name: Option<String>,
    },
    /// Over the file it came from — once the original has been set
    /// aside as `set_aside_as`, beside it, extension kept. If a file of
    /// that name is already there it is **not** overwritten: the first
    /// original is the original, and the job refuses rather than losing
    /// it. That half is the batch's to enforce (E4); this is what it
    /// enforces.
    Over {
        file: PathBuf,
        set_aside_as: PathBuf,
    },
    /// Back as text — the clipboard, the pane — and to no file.
    AsText,
}

/// What Wipemark keeps of something that arrived without a file, and
/// for how long. Present only when at least one of the two is kept.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Kept {
    pub original: bool,
    pub result: bool,
    pub for_: Period,
    /// [`Homes::kept`].
    pub in_: PathBuf,
}

/// The rows, applied to one thing that arrived.
///
/// Pure: the folders come in through `homes` and nothing here asks the
/// disk whether they exist. Whether `name.original.ext` is already
/// there is exactly the check this function *cannot* make, which is
/// why [`Written::Over`] states the rule for whoever does.
pub fn plan(source: &Source, retention: &Retention, homes: &Homes) -> Plan {
    let results_folder = retention.folder.as_deref().unwrap_or(&homes.results);
    match source {
        Source::Folder(path) => Plan::EachFileIn(path.clone()),
        Source::File(path) => {
            let name = file_name(path);
            Plan::File(match retention.destination {
                Destination::Beside => {
                    Written::Beside(path.with_file_name(with_infix(&name, RESULT_INFIX)))
                }
                Destination::Folder => Written::Into {
                    folder: results_folder.to_path_buf(),
                    name: Some(with_infix(&name, RESULT_INFIX)),
                },
                Destination::Replace => Written::Over {
                    file: path.clone(),
                    set_aside_as: path.with_file_name(with_infix(&name, ORIGINAL_INFIX)),
                },
            })
        }
        Source::Text => Plan::Loose {
            result: Written::AsText,
            kept: kept(retention, homes),
        },
        Source::Bytes { name } => Plan::Loose {
            // Bytes have nowhere to sit beside, whatever the
            // destination says, so the results folder is the answer
            // in every mode. The name is kept as it came: a result
            // that renamed a screenshot would be a result nobody
            // could match to what they dropped.
            result: Written::Into {
                folder: results_folder.to_path_buf(),
                name: name.as_deref().map(|name| with_infix(name, RESULT_INFIX)),
            },
            kept: kept(retention, homes),
        },
    }
}

fn kept(retention: &Retention, homes: &Homes) -> Option<Kept> {
    retention.keeps().then(|| Kept {
        original: retention.keep_originals,
        result: retention.keep_results,
        for_: retention.keep_for,
        in_: homes.kept.clone(),
    })
}

/// The last component of a path, as text. Lossy for a name this
/// platform cannot spell — the plan is for reading, and a name with a
/// replacement character in it still says which file is meant.
fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use wipemark_intake::{Arrived, Evidence, Format, Intake, Kind};

    use super::{
        period_choices, plan, Destination, Homes, Kept, Period, Plan, Retention, Source, Written,
    };

    fn homes() -> Homes {
        Homes {
            results: PathBuf::from("/Users/someone/Downloads"),
            kept: PathBuf::from("/Users/someone/Library/Application Support/wipemark/kept"),
        }
    }

    fn file(path: &str) -> Source {
        Source::File(PathBuf::from(path))
    }

    /// A paste, as the intake crate reports one, in the given format.
    fn pasted(format: Format) -> Intake {
        Intake {
            kind: format.kind(),
            format: Some(format),
            encoding: None,
            name: None,
            path: None,
            size: Some(12),
            evidence: Evidence::Content,
            arrived: Arrived::AsText,
        }
    }

    /// The default is the non-destructive one, and the one `mat2` and
    /// spec §4.5 both chose: the file is not touched, and the result
    /// sits beside it under the name a script can find.
    #[test]
    fn by_default_a_file_is_left_alone_and_its_result_goes_beside_it() {
        let planned = plan(&file("/docs/report.docx"), &Retention::default(), &homes());
        assert_eq!(
            planned,
            Plan::File(Written::Beside(PathBuf::from("/docs/report.cleaned.docx")))
        );
    }

    /// The one case where collapsing would be wrong. `x.cleaned.md`
    /// dropped again must not plan a result at `x.cleaned.md` — that is
    /// the input, and "beside" would have become "over".
    #[test]
    fn a_result_beside_a_file_is_never_the_file_itself() {
        for name in ["x.cleaned.md", "x.md", "x", ".x", "x.cleaned"] {
            let path = PathBuf::from("/d").join(name);
            let planned = plan(&Source::File(path.clone()), &Retention::default(), &homes());
            let Plan::File(Written::Beside(result)) = planned else {
                panic!("{name}: not planned beside");
            };
            assert_ne!(result, path, "{name}: the result would overwrite the input");
            assert_eq!(result.parent(), path.parent(), "{name}: not beside");
        }
    }

    /// Replacing is never destructive: the original is set aside under
    /// a name that keeps its extension, before the result goes over
    /// the file.
    #[test]
    fn replacing_a_file_sets_the_original_aside_first() {
        let retention = Retention {
            destination: Destination::Replace,
            ..Retention::default()
        };
        let planned = plan(&file("/docs/report.docx"), &retention, &homes());
        assert_eq!(
            planned,
            Plan::File(Written::Over {
                file: PathBuf::from("/docs/report.docx"),
                set_aside_as: PathBuf::from("/docs/report.original.docx"),
            })
        );
    }

    /// The results folder is the platform's Downloads folder until a
    /// row says otherwise, and the row wins when it does.
    #[test]
    fn the_results_folder_is_downloads_until_a_row_names_one() {
        let into = Retention {
            destination: Destination::Folder,
            ..Retention::default()
        };
        let planned = plan(&file("/docs/report.docx"), &into, &homes());
        assert_eq!(
            planned,
            Plan::File(Written::Into {
                folder: PathBuf::from("/Users/someone/Downloads"),
                name: Some("report.cleaned.docx".to_owned()),
            })
        );

        let named = Retention {
            folder: Some(PathBuf::from("/Volumes/Work/cleaned")),
            ..into
        };
        let Plan::File(Written::Into { folder, .. }) =
            plan(&file("/docs/report.docx"), &named, &homes())
        else {
            panic!("not planned into a folder");
        };
        assert_eq!(folder, Path::new("/Volumes/Work/cleaned"));
    }

    /// A paste comes back as text and is written to no file, whatever
    /// the destination row says — there is no file for "beside" or
    /// "over" to mean.
    #[test]
    fn text_comes_back_as_text_whatever_the_destination() {
        for destination in Destination::ALL {
            let retention = Retention {
                destination,
                ..Retention::default()
            };
            assert_eq!(
                plan(&Source::Text, &retention, &homes()),
                Plan::Loose {
                    result: Written::AsText,
                    kept: None,
                },
                "{destination:?}"
            );
        }
    }

    /// Bytes have nowhere to sit beside, so they land in the results
    /// folder in every mode, under the name they came with.
    #[test]
    fn bytes_land_in_the_results_folder_under_their_own_name() {
        for destination in Destination::ALL {
            let retention = Retention {
                destination,
                ..Retention::default()
            };
            let planned = plan(
                &Source::Bytes {
                    name: Some("shot.png".to_owned()),
                },
                &retention,
                &homes(),
            );
            assert_eq!(
                planned,
                Plan::Loose {
                    result: Written::Into {
                        folder: PathBuf::from("/Users/someone/Downloads"),
                        name: Some("shot.cleaned.png".to_owned()),
                    },
                    kept: None,
                },
                "{destination:?}"
            );
        }
        let Plan::Loose {
            result: Written::Into { name, .. },
            ..
        } = plan(
            &Source::Bytes { name: None },
            &Retention::default(),
            &homes(),
        )
        else {
            panic!("not planned into a folder");
        };
        assert_eq!(name, None, "a name was invented for nameless bytes");
    }

    /// The rule this module exists to state. A Markdown paste, an HTML
    /// paste and a plain one are one `Source`, so nothing downstream
    /// can keep one and not another — and a PNG and a DOCX on disk are
    /// likewise one. The failure this catches is a later `match` on
    /// `intake.format` in [`Source::of`].
    #[test]
    fn a_markdown_paste_and_a_plain_one_are_planned_alike() {
        let plain = Source::of(&pasted(Format::PlainText));
        for format in [Format::Markdown, Format::Html, Format::Json, Format::Csv] {
            assert_eq!(
                Source::of(&pasted(format)),
                plain,
                "{format:?} was sourced differently from plain text"
            );
        }
        let on_disk = |format: Format, name: &str| Intake {
            kind: format.kind(),
            format: Some(format),
            encoding: None,
            name: Some(name.to_owned()),
            path: Some(PathBuf::from("/d").join(name)),
            size: Some(1),
            evidence: Evidence::Agreed,
            arrived: Arrived::AsPath,
        };
        assert_eq!(
            Source::of(&on_disk(Format::Png, "a")),
            Source::of(&on_disk(Format::Docx, "a"))
        );
    }

    /// Both switches are off unless somebody turned one on, and when
    /// they are, the plan says which, for how long and where. Files are
    /// never kept: the variant has no field for it.
    #[test]
    fn nothing_is_kept_unless_it_is_asked_for() {
        assert!(!Retention::default().keeps());
        let asked = Retention {
            keep_originals: true,
            keep_for: Period::Month,
            ..Retention::default()
        };
        assert_eq!(
            plan(&Source::Text, &asked, &homes()),
            Plan::Loose {
                result: Written::AsText,
                kept: Some(Kept {
                    original: true,
                    result: false,
                    for_: Period::Month,
                    in_: homes().kept,
                }),
            }
        );
        // A file with keeping on is still a `File`, and a `File` has
        // nowhere to say "kept".
        assert!(matches!(
            plan(&file("/d/a.md"), &asked, &homes()),
            Plan::File(_)
        ));
    }

    /// A line of text that names a file that exists *is* that file —
    /// the intake crate has already said so — and the plan must go
    /// beside the document, not beside nineteen characters.
    #[test]
    fn text_that_names_a_file_is_planned_as_the_file() {
        let intake = Intake {
            kind: Kind::Document,
            format: Some(Format::Docx),
            encoding: None,
            name: Some("report.docx".to_owned()),
            path: Some(PathBuf::from("/docs/report.docx")),
            size: Some(1),
            evidence: Evidence::Agreed,
            // How it arrived is still the truth about the transport.
            arrived: Arrived::AsText,
        };
        assert_eq!(Source::of(&intake), file("/docs/report.docx"));
    }

    #[test]
    fn a_folder_is_its_files() {
        let intake = Intake {
            kind: Kind::Folder,
            format: None,
            encoding: None,
            name: Some("photos".to_owned()),
            path: Some(PathBuf::from("/Users/someone/photos")),
            size: None,
            evidence: Evidence::Content,
            arrived: Arrived::AsPath,
        };
        assert_eq!(
            plan(&Source::of(&intake), &Retention::default(), &homes()),
            Plan::EachFileIn(PathBuf::from("/Users/someone/photos"))
        );
    }

    /// The stored spellings round-trip, and a spelling this build does
    /// not know is refused rather than guessed.
    #[test]
    fn stored_spellings_round_trip() {
        for destination in Destination::ALL {
            assert_eq!(Destination::parse(destination.id()), Some(destination));
            assert_eq!(
                Destination::parse(&format!(" {} ", destination.id())),
                Some(destination)
            );
        }
        for period in Period::ALL {
            assert_eq!(Period::parse(period.id()), Some(period));
        }
        assert_eq!(Destination::parse("in-place"), None);
        assert_eq!(Destination::parse(""), None);
        assert_eq!(Period::parse("14d"), None);
        assert_eq!(Period::parse("0"), None);
        assert_eq!(Period::parse(""), None);
    }

    /// The id is the number of days with a `d` on it, or `forever` —
    /// so whoever removes old copies (E4) reads the span off the row
    /// without a second table, and no spelling reads as zero days,
    /// which is "remove at once" to anything doing arithmetic.
    #[test]
    fn the_period_is_the_number_in_its_own_id() {
        let days = |period: Period| -> Option<u32> {
            period.id().strip_suffix('d').and_then(|n| n.parse().ok())
        };
        assert_eq!(days(Period::Day), Some(1));
        assert_eq!(days(Period::Week), Some(7));
        assert_eq!(days(Period::Month), Some(30));
        assert_eq!(days(Period::Quarter), Some(90));
        assert_eq!(Period::Forever.id(), "forever");
        for period in Period::ALL {
            assert_ne!(
                days(period),
                Some(0),
                "{period:?} would remove a copy at once"
            );
            assert!(
                days(period).is_some() || period == Period::Forever,
                "{period:?} is neither a number of days nor forever"
            );
        }
    }

    /// The dropdown lists every period once, in order, and each row's
    /// value is the stored spelling rather than the label.
    #[test]
    fn the_period_selector_offers_every_period_once() {
        let choices = period_choices();
        assert_eq!(choices.len(), Period::ALL.len());
        for (choice, period) in choices.iter().zip(Period::ALL) {
            assert_eq!(*choice.item(), period);
        }
    }
}
