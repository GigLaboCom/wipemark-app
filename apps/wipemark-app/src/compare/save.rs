//! What the Compare window's Save writes, and where (E7-9).
//!
//! The window's half that has no window in it: where a save goes, by what
//! the window was opened on ([`save_target`], the S1 table), whether the
//! file it would write over is still the one the window read ([`Stamp`]),
//! the blocking writes ([`save_file`], [`save_item`]), and when an edit is
//! saved on its own ([`Saver`]). Everything here that touches a disk or a
//! database **blocks**, and the window runs it on the background executor;
//! the rest is pure, and tested as such.
//!
//! # Where Save writes (S1, D410, D411)
//!
//! | the window was opened on | Save writes |
//! |---|---|
//! | a row nothing was written for — waiting, nothing found, refused, failed — or `--compare=<path>` | a **Clean of the pane's text** ([`Target::Clean`]): the application's one line of cleans, the plan the row's Clean takes when it starts, its refusals — a taken name asked about, never written over unasked |
//! | a cleaned row whose result is a file — beside, in the results folder | **over that file** ([`Target::File`]), atomically |
//! | a row cleaned in place | over the file that **now holds the result** — the source's own name — and never over the original set aside beside it, which is what the window shows on the left |
//! | a cleaned paste | **its row** ([`Target::Row`]): what Copy the result copies and Compare opens on — in memory, as the cleaned text itself is |
//! | a rewritten row | over the **rewrite's file** — `name.rewritten.ext`, or the source's name for a rewrite in place |
//! | a rewritten paste | the **batch queue's row** ([`Target::Item`]), the result's one home |
//! | a result that *is* the original file, nothing set aside | nowhere ([`NoTarget::Original`]): Save is greyed and says why |
//!
//! Never the original; never a symbolic link (D287's rule: a save would
//! replace the link and leave the file it points to as it was); text in
//! the encoding it arrived in.
//!
//! # Changed on disk (D413)
//!
//! A save over a file first asks whether the file is still what this
//! window last read or wrote there — by its size, then by its bytes, never
//! by its time alone, which a file system may keep to the second. A file
//! that moved is not written over: the window asks (Overwrite, Keep
//! theirs, Cancel). A batch queue's row is held to the text this window
//! last read or wrote there the same way.

use std::hash::{Hash as _, Hasher as _};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use wipemark_intake::{inplace, Encoding, Handed};
use wipemark_queue::{ItemId, Queue};

use super::{CleanedTo, Made, RewriteFrom};

/// Where a save writes — see the module docs.
#[derive(Clone)]
pub enum Target {
    /// A Clean of the pane's text, through the application's line, by the
    /// plan taken when it starts (D411).
    Clean,
    /// Over this file — the result's own (D410).
    File(PathBuf),
    /// Into the batch queue's row of a rewritten paste.
    Item(Arc<Queue>, ItemId),
    /// Into the main window's row of a cleaned paste.
    Row,
}

impl PartialEq for Target {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Target::Clean, Target::Clean) | (Target::Row, Target::Row) => true,
            (Target::File(a), Target::File(b)) => a == b,
            (Target::Item(a, i), Target::Item(b, j)) => Arc::ptr_eq(a, b) && i == j,
            _ => false,
        }
    }
}

impl std::fmt::Debug for Target {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Target::Clean => f.write_str("Clean"),
            Target::File(path) => f.debug_tuple("File").field(path).finish(),
            Target::Item(_, item) => f.debug_tuple("Item").field(item).finish(),
            Target::Row => f.write_str("Row"),
        }
    }
}

/// How a save is told to go past what it would otherwise stop at — the
/// person's answer to a question.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Force {
    /// Overwrite: write over a file or a row that changed since it was read.
    Overwrite,
    /// A Save that cleans may write over this one file, already where the
    /// result goes (D261's Replace, asked by the person).
    Replacing(PathBuf),
}

/// Why a window has nowhere to save.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoTarget {
    /// The result is the original's own file, with nothing set aside: a
    /// save would write over the original.
    Original,
    /// The result lives in a row of the main window, and this window was
    /// not opened from one.
    NoRow,
}

/// Where Save writes, by what the window was opened on (S1, D410, D411).
/// `original` is what the window shows on the left; `linked` whether it
/// was opened from a row of the main window. Pure.
pub fn save_target(made: &Made, original: &Handed, linked: bool) -> Result<Target, NoTarget> {
    let over = |path: &PathBuf| match original {
        Handed::Path(original) if original == path => Err(NoTarget::Original),
        _ => Ok(Target::File(path.clone())),
    };
    match made {
        Made::Cleaned => Ok(Target::Clean),
        Made::CleanedTo(CleanedTo::File(path)) => over(path),
        Made::CleanedTo(CleanedTo::Text(_)) if linked => Ok(Target::Row),
        Made::CleanedTo(CleanedTo::Text(_)) => Err(NoTarget::NoRow),
        Made::Rewritten {
            from: RewriteFrom::File(path),
            ..
        } => over(path),
        Made::Rewritten {
            from: RewriteFrom::Item(queue, item),
            ..
        } => Ok(Target::Item(Arc::clone(queue), *item)),
    }
}

/// A file's contents as this window last read or wrote them: its length
/// and a digest of its bytes — what [`unchanged`] compares a file against
/// before a save writes over it (D413).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stamp {
    pub len: u64,
    pub digest: u64,
}

impl Stamp {
    /// The stamp of `bytes`.
    pub fn of(bytes: &[u8]) -> Self {
        Self {
            len: bytes.len() as u64,
            digest: digest(bytes),
        }
    }

    /// The stamp of the file at `path` now. Blocking.
    pub fn read(path: &Path) -> io::Result<Self> {
        std::fs::read(path).map(|bytes| Self::of(&bytes))
    }
}

/// A digest of `bytes` — enough to tell a file somebody changed from the
/// one this window wrote, within one run. Not a checksum anything is
/// verified by.
fn digest(bytes: &[u8]) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut hasher);
    hasher.finish()
}

/// The digest of a text — what a batch queue row's text is held to.
pub fn digest_of(text: &str) -> u64 {
    digest(text.as_bytes())
}

/// Whether the file at `path` still holds what `seen` says: its size
/// first, which costs a `stat`, and its bytes when the size agrees —
/// never its modification time alone, which a file system may keep to
/// the second and a copy may carry over. A file that is gone has changed.
/// Blocking.
pub fn unchanged(path: &Path, seen: &Stamp) -> io::Result<bool> {
    let metadata = match std::fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error),
    };
    if metadata.len() != seen.len {
        return Ok(false);
    }
    Ok(Stamp::read(path)? == *seen)
}

/// Why a save did not write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotSaved {
    /// The file, or the row, is not what this window last read or wrote
    /// there (D413) — asked about, never written over unasked.
    Changed,
    /// The result's file is a symbolic link (D287).
    Link,
    /// The file is the original itself.
    Original,
    /// The row that held the result is gone, or holds no text any more.
    Gone,
    /// The file could not be read back to be compared.
    Unreadable,
    /// The document is in the line of cleans already, or being rewritten:
    /// a Save that cleans waits for nothing it did not ask for.
    Busy,
    /// The write failed — the operating system's sentence, for the person
    /// and never for a log.
    Write {
        kind: io::ErrorKind,
        message: String,
    },
}

impl NotSaved {
    fn write(error: &io::Error) -> Self {
        NotSaved::Write {
            kind: error.kind(),
            message: error.to_string(),
        }
    }
}

/// Write `text` over the result's own file at `path`, in `encoding`
/// (D410) — never over `original`, never through a symbolic link, and,
/// unless `seen` is `None` (the person said Overwrite), only while the
/// file still holds what `seen` says (D413). The write is
/// `inplace::write_atomically`: a temporary beside it, synced, renamed
/// over — so a run that dies part way leaves the file as it was, and a
/// hard link to it keeps the old bytes. The stamp of what was written.
/// Blocking.
pub fn save_file(
    path: &Path,
    original: Option<&Path>,
    encoding: Encoding,
    text: &str,
    seen: Option<&Stamp>,
) -> Result<Stamp, NotSaved> {
    // Only the last name is a link's to answer for: a file reached
    // through a linked folder is the file (`/var` → `/private/var`).
    if std::fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
        return Err(NotSaved::Link);
    }
    if original.is_some_and(|original| original == path || inplace::same_file(original, path)) {
        return Err(NotSaved::Original);
    }
    if let Some(seen) = seen {
        match unchanged(path, seen) {
            Ok(true) => {}
            Ok(false) => return Err(NotSaved::Changed),
            Err(_) => return Err(NotSaved::Unreadable),
        }
    }
    let bytes = wipemark_intake::text::encode(text, encoding);
    // The file's own permissions, while it is there to take them from.
    let model = std::fs::metadata(path).is_ok().then_some(path);
    inplace::write_atomically(path, &bytes, model).map_err(|error| NotSaved::write(&error))?;
    Ok(Stamp::of(&bytes))
}

/// Put `text` in the batch queue's row of a rewritten paste (D410) —
/// unless `seen` is `None` (Overwrite), only while that row still holds
/// the text whose digest `seen` is. The digest of what was written.
/// Blocking.
pub fn save_item(
    queue: &Queue,
    item: ItemId,
    text: &str,
    seen: Option<u64>,
) -> Result<u64, NotSaved> {
    if let Some(seen) = seen {
        let now = queue
            .result(item)
            .map_err(|_| NotSaved::Unreadable)?
            .and_then(|result| result["text"].as_str().map(digest_of));
        match now {
            None => return Err(NotSaved::Gone),
            Some(now) if now != seen => return Err(NotSaved::Changed),
            Some(_) => {}
        }
    }
    match queue.save_text(item, text) {
        Ok(true) => Ok(digest_of(text)),
        Ok(false) => Err(NotSaved::Gone),
        Err(error) => Err(NotSaved::Write {
            kind: io::ErrorKind::Other,
            message: error.to_string(),
        }),
    }
}

/// When a window saves on its own (S3, D415) — pure, so the rules are
/// held without a window.
///
/// With autosave on, an edit is saved once typing has been quiet for
/// [`QUIET`]: every edit numbers itself, and a quiet that ends for an edit
/// a later one overtook saves nothing. One save at a time: a save asked
/// while one runs — by a quiet, by Save, by a close — is remembered and
/// run when it ends, with the text as it stands *then*, so the last edit
/// wins. A save that fails stops autosaving until a Save succeeds; Save
/// itself is always tried.
#[derive(Debug, Clone, Default)]
pub struct Saver {
    on: bool,
    stopped: bool,
    running: bool,
    again: bool,
    edits: u64,
}

/// How long typing has to be quiet before an edit is saved on its own.
/// A second and a half: long enough that a word is not saved letter by
/// letter, short enough that a window closed by a crash has lost little.
pub const QUIET: std::time::Duration = std::time::Duration::from_millis(1500);

impl Saver {
    /// A window's saver: autosaving or not, as the Compare page said when
    /// it opened.
    pub fn new(autosave: bool) -> Self {
        Self {
            on: autosave,
            ..Self::default()
        }
    }

    /// Whether edits are saved on their own in this window.
    pub fn autosaves(&self) -> bool {
        self.on
    }

    /// Whether a failed save has stopped autosaving until a Save succeeds.
    pub fn stopped(&self) -> bool {
        self.stopped
    }

    /// Whether a save is under way.
    pub fn running(&self) -> bool {
        self.running
    }

    /// The text was edited: the number of the quiet to wait for, or `None`
    /// when nothing in this window saves on its own now.
    pub fn edited(&mut self) -> Option<u64> {
        self.edits += 1;
        (self.on && !self.stopped).then_some(self.edits)
    }

    /// The quiet after edit `edit` is over: whether to start a save now.
    /// Not for an edit a later one overtook, not while autosaving is off
    /// or stopped — and, while a save runs, not now but after it.
    pub fn quiet(&mut self, edit: u64) -> bool {
        if edit != self.edits || !self.on || self.stopped {
            return false;
        }
        self.ask()
    }

    /// A save is asked for — Save, a close, a quiet: whether to start it
    /// now, or, while one runs, after it.
    pub fn ask(&mut self) -> bool {
        if self.running {
            self.again = true;
            return false;
        }
        self.running = true;
        true
    }

    /// The save under way ended: whether to start the one asked for
    /// meanwhile. A failure stops autosaving, and drops what was asked
    /// after it: the next save is the person's.
    pub fn ended(&mut self, saved: bool) -> bool {
        self.running = false;
        if !saved {
            self.stopped = true;
            self.again = false;
            return false;
        }
        self.stopped = false;
        if std::mem::take(&mut self.again) {
            return self.ask();
        }
        false
    }

    /// Autosave again, as before a stop: the question was answered by
    /// putting the file's text in the window.
    pub fn resume(&mut self) {
        self.stopped = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Scratch(PathBuf);

    impl Scratch {
        fn new(label: &str) -> Self {
            let directory =
                std::env::temp_dir().join(format!("wipemark-save-{label}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&directory);
            std::fs::create_dir_all(&directory).expect("scratch directory");
            Self(directory)
        }

        fn file(&self, name: &str, bytes: &[u8]) -> PathBuf {
            let path = self.0.join(name);
            std::fs::write(&path, bytes).expect("scratch file");
            path
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).ok();
        }
    }

    fn queue() -> Arc<Queue> {
        Arc::new(
            Queue::on(
                Arc::new(wipemark_store::Store::in_memory().expect("memory")),
                wipemark_queue::Durability::Memory { detail: None },
                Arc::new(wipemark_engine::fake::FakeEngine::new()),
            )
            .expect("opens"),
        )
    }

    /// The S1 table, row by row (D410, D411): what the window was opened
    /// on decides where Save writes, and a result that is the original's
    /// own file is nowhere. Point a rewrite's file at the clean's road, or
    /// take the original check out of `over`, and a row here is red.
    #[test]
    fn where_save_writes_by_what_the_window_was_opened_on() {
        let source = PathBuf::from("/docs/notes.md");
        let original = Handed::Path(source.clone());
        let beside = PathBuf::from("/docs/notes.cleaned.md");
        let set_aside = Handed::Path(PathBuf::from("/docs/notes.original.md"));
        let rewritten = PathBuf::from("/docs/notes.rewritten.md");
        let paste = Handed::Text("pasted\n".to_owned());
        let queue = queue();
        let item = ItemId(4);

        let rows: Vec<(&str, Made, &Handed, bool, Result<Target, NoTarget>)> = vec![
            (
                "a row nothing was written for: a Clean of the pane",
                Made::Cleaned,
                &original,
                true,
                Ok(Target::Clean),
            ),
            (
                "--compare=<path>: a Clean of the pane, no row",
                Made::Cleaned,
                &original,
                false,
                Ok(Target::Clean),
            ),
            (
                "a paste nothing was kept for: a Clean of the pane",
                Made::Cleaned,
                &paste,
                true,
                Ok(Target::Clean),
            ),
            (
                "cleaned beside: over the result",
                Made::CleanedTo(CleanedTo::File(beside.clone())),
                &original,
                true,
                Ok(Target::File(beside.clone())),
            ),
            (
                "cleaned in place: over the source's name, the original set aside",
                Made::CleanedTo(CleanedTo::File(source.clone())),
                &set_aside,
                true,
                Ok(Target::File(source.clone())),
            ),
            (
                "cleaned in place with nothing set aside: nowhere",
                Made::CleanedTo(CleanedTo::File(source.clone())),
                &original,
                true,
                Err(NoTarget::Original),
            ),
            (
                "a cleaned paste: its row",
                Made::CleanedTo(CleanedTo::Text("cleaned\n".to_owned())),
                &paste,
                true,
                Ok(Target::Row),
            ),
            (
                "a cleaned paste with no row to reach",
                Made::CleanedTo(CleanedTo::Text("cleaned\n".to_owned())),
                &paste,
                false,
                Err(NoTarget::NoRow),
            ),
            (
                "rewritten beside: over the rewrite's file",
                Made::Rewritten {
                    from: RewriteFrom::File(rewritten.clone()),
                    kept: None,
                },
                &original,
                true,
                Ok(Target::File(rewritten.clone())),
            ),
            (
                "rewritten in place: over the source's name",
                Made::Rewritten {
                    from: RewriteFrom::File(source.clone()),
                    kept: None,
                },
                &set_aside,
                true,
                Ok(Target::File(source.clone())),
            ),
            (
                "a rewritten paste: the batch queue's row",
                Made::Rewritten {
                    from: RewriteFrom::Item(Arc::clone(&queue), item),
                    kept: Some((1, 3)),
                },
                &paste,
                true,
                Ok(Target::Item(Arc::clone(&queue), item)),
            ),
        ];
        for (case, made, original, linked, expected) in rows {
            assert_eq!(save_target(&made, original, linked), expected, "{case}");
        }
    }

    /// Over the result's own file, in the encoding it arrived in, and
    /// atomically: a hard link to the old result keeps the old bytes, as a
    /// rename leaves them. Never the original, never a symbolic link, and
    /// never a file that moved since it was read unless the person said
    /// Overwrite. Take any of the three checks out of `save_file` and its
    /// assertion here is red.
    #[test]
    fn a_save_writes_over_the_result_and_never_the_original() {
        let scratch = Scratch::new("file");
        let original = scratch.file("x.md", "original\u{200B}\n".as_bytes());
        let text = "\u{FEFF}cleaned\n";
        let encoded = wipemark_intake::text::encode(text, Encoding::Utf16Be);
        let result = scratch.file("x.cleaned.md", &encoded);
        let seen = Stamp::of(&encoded);
        let alias = scratch.0.join("alias.md");
        std::fs::hard_link(&result, &alias).expect("hard link");

        let edited = "\u{FEFF}cleaned, and edited\n";
        let stamp = save_file(
            &result,
            Some(&original),
            Encoding::Utf16Be,
            edited,
            Some(&seen),
        )
        .expect("saved");
        let written = wipemark_intake::text::encode(edited, Encoding::Utf16Be);
        assert_eq!(
            std::fs::read(&result).expect("read"),
            written,
            "not in UTF-16BE"
        );
        assert_eq!(stamp, Stamp::of(&written));
        assert_eq!(
            std::fs::read(&alias).expect("read"),
            encoded,
            "written into the file rather than renamed over it"
        );
        assert_eq!(
            std::fs::read(&original).expect("read"),
            "original\u{200B}\n".as_bytes()
        );

        // Changed since: asked, not written — whatever the time says.
        std::fs::write(&result, "theirs, same length!".repeat(2)).expect("theirs");
        assert_eq!(
            save_file(
                &result,
                Some(&original),
                Encoding::Utf8,
                "mine\n",
                Some(&stamp)
            ),
            Err(NotSaved::Changed)
        );
        assert_eq!(
            std::fs::read(&result).expect("read"),
            "theirs, same length!".repeat(2).as_bytes()
        );
        // Same length, other bytes: still changed.
        let same_length = vec![b'x'; written.len()];
        std::fs::write(&result, &same_length).expect("theirs");
        assert_eq!(
            save_file(
                &result,
                Some(&original),
                Encoding::Utf8,
                "mine\n",
                Some(&stamp)
            ),
            Err(NotSaved::Changed),
            "a change of the same length passed for none"
        );
        // Overwrite: the person's word.
        save_file(&result, Some(&original), Encoding::Utf8, "mine\n", None).expect("overwrite");
        assert_eq!(std::fs::read(&result).expect("read"), b"mine\n");

        // The original itself, by name and by a hard link to it.
        assert_eq!(
            save_file(&original, Some(&original), Encoding::Utf8, "x", None),
            Err(NotSaved::Original)
        );
        let second = scratch.0.join("second-name.md");
        std::fs::hard_link(&original, &second).expect("hard link");
        assert_eq!(
            save_file(&second, Some(&original), Encoding::Utf8, "x", None),
            Err(NotSaved::Original)
        );
        assert_eq!(
            std::fs::read(&original).expect("read"),
            "original\u{200B}\n".as_bytes()
        );
    }

    /// A result reached through a symbolic link is refused (D287): the
    /// link stays a link to the same file, and that file is untouched.
    #[cfg(unix)]
    #[test]
    fn a_save_through_a_symbolic_link_is_refused() {
        let scratch = Scratch::new("link");
        let target = scratch.file("target.md", b"result\n");
        let link = scratch.0.join("x.cleaned.md");
        std::os::unix::fs::symlink(&target, &link).expect("link");
        assert_eq!(
            save_file(&link, None, Encoding::Utf8, "edited\n", None),
            Err(NotSaved::Link)
        );
        assert!(std::fs::symlink_metadata(&link)
            .expect("still there")
            .file_type()
            .is_symlink());
        assert_eq!(std::fs::read(&target).expect("read"), b"result\n");
    }

    /// A file that is gone has changed, and a file the size and bytes of
    /// what was seen has not.
    #[test]
    fn a_file_is_unchanged_only_while_its_bytes_are() {
        let scratch = Scratch::new("unchanged");
        let path = scratch.file("x.md", b"one\n");
        let seen = Stamp::read(&path).expect("stamp");
        assert!(unchanged(&path, &seen).expect("read"));
        std::fs::write(&path, b"two\n").expect("write");
        assert!(!unchanged(&path, &seen).expect("read"));
        std::fs::remove_file(&path).expect("remove");
        assert!(
            !unchanged(&path, &seen).expect("read"),
            "a file that is gone"
        );
    }

    /// The last edit wins with saves in flight, a quiet an edit overtook
    /// saves nothing, and a failure stops autosaving until a save
    /// succeeds (D415). Drop `again` from `ended` and the edit typed
    /// while a save ran is never saved: red.
    #[test]
    fn the_last_edit_wins_and_a_failure_stops_autosave() {
        let mut saver = Saver::new(true);
        let first = saver.edited().expect("autosaving");
        let second = saver.edited().expect("autosaving");
        assert!(!saver.quiet(first), "a quiet an edit overtook saved");
        assert!(
            saver.quiet(second),
            "the quiet after the last edit saved nothing"
        );
        assert!(saver.running());

        // Typed while it saves: not a second save beside it — after it.
        let third = saver.edited().expect("autosaving");
        assert!(!saver.quiet(third), "two saves at once");
        assert!(saver.ended(true), "the edit typed meanwhile was not saved");
        assert!(saver.running());
        assert!(!saver.ended(true), "saved again with nothing asked");

        // A failure stops autosaving, and what was asked meanwhile.
        let fourth = saver.edited().expect("autosaving");
        assert!(saver.quiet(fourth));
        assert!(!saver.ask(), "Save beside a running save");
        assert!(
            !saver.ended(false),
            "a failed save ran what was asked after it"
        );
        assert!(saver.stopped());
        assert_eq!(saver.edited(), None, "autosaved past a failure");
        // Save is the person's, and is tried; its success resumes.
        assert!(saver.ask());
        assert!(!saver.ended(true));
        assert!(!saver.stopped());
        let fifth = saver.edited().expect("autosaving again");
        assert!(saver.quiet(fifth));

        // Off: nothing saves on its own, and Save still does.
        let mut manual = Saver::new(false);
        assert_eq!(manual.edited(), None);
        assert!(!manual.quiet(1));
        assert!(manual.ask());
    }
}
