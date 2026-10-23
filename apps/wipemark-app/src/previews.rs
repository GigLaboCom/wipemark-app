//! What a picture with no file behind it looked like, kept for its row
//! (D823–D828).
//!
//! A picture an agent sends to `clean_image` is base64 in the call, and a
//! picture pasted into a window is bytes in memory: neither leaves a file
//! the table could read again after a restart, and a journal row is
//! metadata (D312). So the application keeps a **preview** of it — the
//! picture the row ended with, the result when one came back and otherwise
//! what arrived (D824) — decoded, turned upright, reduced and written again
//! from its pixels, so it carries no metadata at all. Two sizes, because
//! GPUI keeps every picture it draws decoded in its cache: [`SMALL_SIDE`]
//! for the table's cell, [`LARGE_SIDE`] for the hover card.
//!
//! # Where, and for how long
//!
//! `<data dir>/previews/<journal id>.jpg` and `<journal id>.small.jpg` —
//! PNG when the picture has alpha — each written as a dot temporary and
//! renamed. The id is the journal's, `AUTOINCREMENT`, so a preview never
//! lands on another row. A preview goes with its row ([`Previews::remove`],
//! which `Journal::remove` and `Journal::sweep` call), every one of them
//! with [`Previews::remove_all`], and a file whose row is gone with
//! [`Previews::sweep`] at launch (D825). Removing is `unlink`, not
//! shredding.
//!
//! # Never in anybody's way
//!
//! One thread, `wipemark-previews`, and a line of at most [`LINE`] jobs
//! (D827): a job that does not fit is dropped and said in the log, a
//! picture that will not decode is a log line, and nobody waits for
//! either. A removal is final for the life of the process: a job for a
//! removed row, or one asked for before [`Previews::remove_all`], is decided
//! under the lock that removes, and is never renamed into place.

use std::collections::HashSet;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::PngEncoder;
use image::{DynamicImage, ImageDecoder as _, ImageEncoder as _, ImageReader};

/// The long side of the preview the hover card shows: its 560×380 points
/// at twice the pixels, near enough, for a Retina screen.
pub const LARGE_SIDE: u32 = 1024;

/// The long side of the one the table's 128×56-point cell shows.
pub const SMALL_SIDE: u32 = 256;

/// Past this many pixels a picture is not decoded for a preview: a few
/// hundred kilobytes of PNG can claim a raster of gigabytes.
pub const MAX_PIXELS: u64 = 64_000_000;

/// JPEG's quality for a preview: small, and nothing a person looking at a
/// thumbnail would call blocky.
const JPEG_QUALITY: u8 = 82;

/// How many previews may wait to be made.
const LINE: usize = 32;

/// The largest file a preview is made from — the table's own limit for a
/// thumbnail.
const FILE_LIMIT: u64 = crate::preview::IMAGE_LIMIT;

/// Spotlight leaves a folder holding this file alone.
const NEVER_INDEX: &str = ".metadata_never_index";

/// What a preview is made from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// The picture, in hand: an MCP call's, a paste's.
    Bytes(Vec<u8>),
    /// A result a window wrote: read, at most [`FILE_LIMIT`] of it.
    File(PathBuf),
}

/// A row's kept preview, as the table draws it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Kept {
    /// For the cell.
    pub small: PathBuf,
    /// For the hover card.
    pub large: PathBuf,
}

/// How a preview is written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encoding {
    Jpeg,
    Png,
}

impl Encoding {
    fn extension(self) -> &'static str {
        match self {
            Self::Jpeg => "jpg",
            Self::Png => "png",
        }
    }
}

/// A preview, made: both sizes, in one encoding.
#[derive(Debug)]
pub struct Made {
    pub large: Vec<u8>,
    pub small: Vec<u8>,
    pub encoding: Encoding,
}

/// Why no preview was made.
#[derive(Debug)]
pub enum Unmade {
    Unreadable(String),
    TooLarge { width: u32, height: u32 },
    TooBig(u64),
    Unwritten(String),
}

impl std::fmt::Display for Unmade {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unreadable(error) => write!(f, "not a picture this build decodes: {error}"),
            Self::TooLarge { width, height } => {
                write!(f, "{width}×{height} is past the preview's pixel limit")
            }
            Self::TooBig(size) => write!(f, "a file of {size} bytes is past the preview's limit"),
            Self::Unwritten(error) => write!(f, "the preview could not be encoded: {error}"),
        }
    }
}

/// A preview of `bytes`: decoded, turned the way its EXIF orientation says,
/// the first frame of an animation, reduced to [`LARGE_SIDE`] and
/// [`SMALL_SIDE`] on the long side and never enlarged, and written as a
/// JPEG — a PNG when the picture has alpha. Blocking, and the cost of a
/// decode.
pub fn make(bytes: &[u8]) -> Result<Made, Unmade> {
    let unreadable = |error: image::ImageError| Unmade::Unreadable(error.to_string());
    let reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|error| Unmade::Unreadable(error.to_string()))?;
    let mut decoder = reader.into_decoder().map_err(unreadable)?;
    let (width, height) = decoder.dimensions();
    if u64::from(width) * u64::from(height) > MAX_PIXELS {
        return Err(Unmade::TooLarge { width, height });
    }
    let orientation = decoder.orientation().map_err(unreadable)?;
    let mut picture = DynamicImage::from_decoder(decoder).map_err(unreadable)?;
    picture.apply_orientation(orientation);
    let encoding = if picture.color().has_alpha() {
        Encoding::Png
    } else {
        Encoding::Jpeg
    };
    let large = reduced(&picture, LARGE_SIDE);
    let small = reduced(&large, SMALL_SIDE);
    Ok(Made {
        large: encode(&large, encoding)?,
        small: encode(&small, encoding)?,
        encoding,
    })
}

fn reduced(picture: &DynamicImage, side: u32) -> DynamicImage {
    if picture.width() <= side && picture.height() <= side {
        picture.clone()
    } else {
        picture.thumbnail(side, side)
    }
}

fn encode(picture: &DynamicImage, encoding: Encoding) -> Result<Vec<u8>, Unmade> {
    let unwritten = |error: image::ImageError| Unmade::Unwritten(error.to_string());
    let mut out = Vec::new();
    match encoding {
        // Eight bits and no alpha, whatever the picture had: a JPEG holds
        // nothing else.
        Encoding::Jpeg => {
            let rgb = picture.to_rgb8();
            JpegEncoder::new_with_quality(&mut out, JPEG_QUALITY)
                .encode_image(&rgb)
                .map_err(unwritten)?;
        }
        Encoding::Png => {
            let rgba = picture.to_rgba8();
            PngEncoder::new(&mut out)
                .write_image(
                    rgba.as_raw(),
                    rgba.width(),
                    rgba.height(),
                    image::ExtendedColorType::Rgba8,
                )
                .map_err(unwritten)?;
        }
    }
    Ok(out)
}

/// The kept previews: one folder, one thread that writes into it.
#[derive(Clone)]
pub struct Previews {
    folder: Arc<Folder>,
    jobs: flume::Sender<Job>,
}

impl std::fmt::Debug for Previews {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Previews")
            .field("dir", &self.folder.dir)
            .finish_non_exhaustive()
    }
}

/// The folder, and what decides whether a finished job may go into it.
struct Folder {
    dir: PathBuf,
    state: Mutex<State>,
}

#[derive(Default)]
struct State {
    /// Moved on by [`Previews::remove_all`]: a job asked for under an
    /// earlier one is dropped.
    epoch: u64,
    /// Rows whose preview was removed: no job puts one back.
    removed: HashSet<i64>,
}

enum Job {
    Keep {
        id: i64,
        source: Source,
        epoch: u64,
    },
    #[cfg(test)]
    Settled(flume::Sender<()>),
}

impl Previews {
    /// The previews in `dir`, which is made the first time one is kept;
    /// `kept` is told the row of every preview once both its files are in
    /// place.
    pub fn new(dir: PathBuf, kept: impl Fn(i64) + Send + 'static) -> Self {
        let folder = Arc::new(Folder {
            dir,
            state: Mutex::default(),
        });
        let (jobs, line) = flume::bounded::<Job>(LINE);
        let working = Arc::clone(&folder);
        let spawned = std::thread::Builder::new()
            .name("wipemark-previews".to_owned())
            .spawn(move || {
                for job in line.iter() {
                    match job {
                        Job::Keep { id, source, epoch } => {
                            if working.make(id, source, epoch) {
                                kept(id);
                            }
                        }
                        #[cfg(test)]
                        Job::Settled(done) => {
                            let _ = done.send(());
                        }
                    }
                }
            });
        if let Err(error) = spawned {
            tracing::error!(%error, "the previews' thread could not start");
        }
        Self { folder, jobs }
    }

    /// Make row `id`'s preview of `source`, on the previews' thread — or
    /// drop it, said in the log, when [`LINE`] are already waiting.
    pub fn keep(&self, id: i64, source: Source) {
        let epoch = self.folder.lock().epoch;
        match self.jobs.try_send(Job::Keep { id, source, epoch }) {
            Ok(()) => {}
            Err(flume::TrySendError::Full(_)) => {
                tracing::warn!(
                    id,
                    waiting = LINE,
                    "a preview was not made: the line is full"
                );
            }
            Err(flume::TrySendError::Disconnected(_)) => {
                tracing::warn!(id, "a preview was not made: the previews' thread is gone");
            }
        }
    }

    /// Row `id`'s preview, when both its files are there. Blocking: two
    /// looks at the disk.
    pub fn of(&self, id: i64) -> Option<Kept> {
        ["jpg", "png"].into_iter().find_map(|extension| {
            let kept = Kept {
                small: self.folder.dir.join(format!("{id}.small.{extension}")),
                large: self.folder.dir.join(format!("{id}.{extension}")),
            };
            (kept.small.is_file() && kept.large.is_file()).then_some(kept)
        })
    }

    /// Take row `id`'s preview away, and never make it again in this
    /// process. Blocking.
    pub fn remove(&self, id: i64) {
        let mut state = self.folder.lock();
        state.removed.insert(id);
        self.folder.remove_files(id);
    }

    /// Take every preview away, and drop every one asked for before; the
    /// number of rows that had one. Blocking.
    pub fn remove_all(&self) -> usize {
        let mut state = self.folder.lock();
        state.epoch += 1;
        let mut rows = HashSet::new();
        for (path, name) in self.folder.files() {
            // A temporary is its job's: the job drops it when it finds the
            // epoch moved on, and a launch's sweep takes one a crash left.
            if name.starts_with('.') {
                continue;
            }
            if remove_file(&path) {
                if let Some(id) = row_of(&name) {
                    rows.insert(id);
                }
            }
        }
        rows.len()
    }

    /// Take every file whose row is not in `rows`, and every temporary — at
    /// launch, before anything is kept. The number of files taken. Blocking.
    pub fn sweep(&self, rows: &HashSet<i64>) -> usize {
        let _state = self.folder.lock();
        let mut taken = 0;
        for (path, name) in self.folder.files() {
            let orphan = match row_of(&name) {
                Some(id) => !rows.contains(&id),
                None => name.ends_with(".tmp"),
            };
            if orphan && remove_file(&path) {
                taken += 1;
            }
        }
        taken
    }

    /// How many rows have a preview, and the bytes every file of the folder
    /// takes. Blocking: a listing and a look at each file.
    pub fn usage(&self) -> (usize, u64) {
        let mut rows = HashSet::new();
        let mut bytes = 0;
        for (path, name) in self.folder.files() {
            if name.starts_with('.') {
                continue;
            }
            if let Some(id) = row_of(&name) {
                rows.insert(id);
            }
            bytes += std::fs::metadata(&path).map_or(0, |meta| meta.len());
        }
        (rows.len(), bytes)
    }

    /// Answered once every job asked before it is done — for a test.
    #[cfg(test)]
    pub fn settled(&self) -> flume::Receiver<()> {
        let (done, settled) = flume::bounded(1);
        let _ = self.jobs.send(Job::Settled(done));
        settled
    }
}

impl Folder {
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Make one preview and put it in place; whether it is there.
    fn make(&self, id: i64, source: Source, epoch: u64) -> bool {
        let bytes = match source {
            Source::Bytes(bytes) => bytes,
            Source::File(path) => match read_capped(&path) {
                Ok(bytes) => bytes,
                Err(unmade) => {
                    tracing::info!(id, why = %unmade, "no preview was made");
                    return false;
                }
            },
        };
        let made = match make(&bytes) {
            Ok(made) => made,
            Err(unmade) => {
                tracing::info!(id, why = %unmade, "no preview was made");
                return false;
            }
        };
        match self.put(id, &made, epoch) {
            Ok(true) => {
                tracing::info!(
                    id,
                    encoding = made.encoding.extension(),
                    large = made.large.len(),
                    small = made.small.len(),
                    "a preview was kept"
                );
                true
            }
            Ok(false) => {
                tracing::info!(
                    id,
                    "a preview was dropped: it was removed while it was made"
                );
                false
            }
            Err(error) => {
                tracing::warn!(%error, id, "a preview could not be written");
                false
            }
        }
    }

    /// Write both files as temporaries, then — under the lock that removes,
    /// and only if nothing removed this row or every row since the job was
    /// asked — rename them into place over any earlier preview of the row.
    fn put(&self, id: i64, made: &Made, epoch: u64) -> std::io::Result<bool> {
        std::fs::create_dir_all(&self.dir)?;
        let never = self.dir.join(NEVER_INDEX);
        if !never.exists() {
            std::fs::write(&never, b"")?;
        }
        let small = self.temporary(id, &made.small)?;
        let large = match self.temporary(id, &made.large) {
            Ok(large) => large,
            Err(error) => {
                remove_file(&small);
                return Err(error);
            }
        };
        let state = self.lock();
        if state.epoch != epoch || state.removed.contains(&id) {
            remove_file(&small);
            remove_file(&large);
            return Ok(false);
        }
        self.remove_files(id);
        let extension = made.encoding.extension();
        let placed = std::fs::rename(&small, self.dir.join(format!("{id}.small.{extension}")))
            .and_then(|()| std::fs::rename(&large, self.dir.join(format!("{id}.{extension}"))));
        if let Err(error) = placed {
            remove_file(&small);
            remove_file(&large);
            self.remove_files(id);
            return Err(error);
        }
        Ok(true)
    }

    /// `bytes` in a dot file of the folder's own, named for nobody else.
    fn temporary(&self, id: i64, bytes: &[u8]) -> std::io::Result<PathBuf> {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = self
            .dir
            .join(format!(".{id}.{}.{n}.tmp", std::process::id()));
        std::fs::write(&path, bytes)?;
        Ok(path)
    }

    /// Every file a preview of row `id` can be.
    fn remove_files(&self, id: i64) {
        for extension in ["jpg", "png"] {
            remove_file(&self.dir.join(format!("{id}.{extension}")));
            remove_file(&self.dir.join(format!("{id}.small.{extension}")));
        }
    }

    /// The folder's files and their names, but the one that keeps
    /// Spotlight out. Nothing when there is no folder.
    fn files(&self) -> Vec<(PathBuf, String)> {
        let Ok(entries) = std::fs::read_dir(&self.dir) else {
            return Vec::new();
        };
        entries
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
            .filter_map(|entry| {
                let name = entry.file_name().into_string().ok()?;
                (name != NEVER_INDEX).then(|| (entry.path(), name))
            })
            .collect()
    }
}

/// The row a preview's file name says: `<id>.jpg`, `<id>.small.png`.
fn row_of(name: &str) -> Option<i64> {
    let (id, rest) = name.split_once('.')?;
    if matches!(rest, "jpg" | "png" | "small.jpg" | "small.png") {
        id.parse().ok()
    } else {
        None
    }
}

/// Remove a file; whether it was there to remove.
fn remove_file(path: &Path) -> bool {
    match std::fs::remove_file(path) {
        Ok(()) => true,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
        Err(error) => {
            tracing::warn!(%error, "a preview's file could not be removed");
            false
        }
    }
}

fn read_capped(path: &Path) -> Result<Vec<u8>, Unmade> {
    let size = std::fs::metadata(path)
        .map_err(|error| Unmade::Unreadable(error.to_string()))?
        .len();
    if size > FILE_LIMIT {
        return Err(Unmade::TooBig(size));
    }
    std::fs::read(path).map_err(|error| Unmade::Unreadable(error.to_string()))
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};

    use image::{DynamicImage, GenericImageView as _, ImageEncoder as _, RgbImage, RgbaImage};

    use super::{
        make, row_of, Encoding, Previews, Source, Unmade, LARGE_SIDE, NEVER_INDEX, SMALL_SIDE,
    };

    /// A scratch folder that takes its files away with it.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(label: &str) -> Self {
            static NEXT: AtomicU32 = AtomicU32::new(0);
            let dir = std::env::temp_dir().join(format!(
                "wipemark-previews-{label}-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir_all(&dir).expect("scratch directory");
            Self(dir)
        }

        fn names(&self) -> Vec<String> {
            names(&self.0)
        }
    }

    /// The names in a folder, sorted.
    fn names(dir: &std::path::Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .filter_map(|entry| entry.file_name().into_string().ok())
                    .collect()
            })
            .unwrap_or_default();
        names.sort();
        names
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).ok();
        }
    }

    fn fixture(name: &str) -> Vec<u8> {
        let path = format!("{}/../../fixtures/image/{name}", env!("CARGO_MANIFEST_DIR"));
        std::fs::read(&path).unwrap_or_else(|error| panic!("{path}: {error}"))
    }

    fn decoded(bytes: &[u8]) -> DynamicImage {
        image::load_from_memory(bytes).expect("a preview decodes")
    }

    fn jpeg(width: u32, height: u32) -> Vec<u8> {
        let picture = RgbImage::from_fn(width, height, |x, y| {
            image::Rgb([(x * 7) as u8, (y * 11) as u8, 128])
        });
        let mut out = Vec::new();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 90)
            .encode_image(&picture)
            .expect("encodes");
        out
    }

    /// A PNG's chunk, with its CRC: the decoder checks them.
    fn chunk(png: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
        png.extend_from_slice(&(data.len() as u32).to_be_bytes());
        png.extend_from_slice(kind);
        png.extend_from_slice(data);
        let mut crc = 0xFFFF_FFFFu32;
        for &byte in kind.iter().chain(data) {
            crc ^= u32::from(byte);
            for _ in 0..8 {
                crc = if crc & 1 == 1 {
                    (crc >> 1) ^ 0xEDB8_8320
                } else {
                    crc >> 1
                };
            }
        }
        png.extend_from_slice(&(!crc).to_be_bytes());
    }

    /// D824: both sizes decode, neither past its side, the shape kept — and
    /// what is written is pixels and nothing else: a WebP carrying EXIF,
    /// XMP, an ICC profile and Photoshop's `PSAI` gives a preview in which
    /// the metadata reader finds no block at all.
    #[test]
    fn a_preview_is_two_sizes_of_pixels_and_no_metadata() {
        let source = fixture("exif-xmp.webp");
        assert!(
            !wipemark_image::inspect(&source)
                .expect("reads")
                .findings
                .is_empty(),
            "the fixture carries no metadata to lose"
        );
        let made = make(&source).expect("a preview");
        assert_eq!(made.encoding, Encoding::Jpeg);
        for (bytes, side) in [(&made.large, LARGE_SIDE), (&made.small, SMALL_SIDE)] {
            let picture = decoded(bytes);
            assert!(picture.width() <= side && picture.height() <= side);
            let report = wipemark_image::inspect(bytes).expect("the preview reads");
            assert!(report.findings.is_empty(), "{:?}", report.findings);
        }

        let wide = jpeg(2000, 500);
        let made = make(&wide).expect("a preview");
        assert_eq!(
            decoded(&made.large).dimensions(),
            (LARGE_SIDE, LARGE_SIDE / 4)
        );
        assert_eq!(
            decoded(&made.small).dimensions(),
            (SMALL_SIDE, SMALL_SIDE / 4)
        );
    }

    /// A picture smaller than a side is never enlarged to it.
    #[test]
    fn a_small_picture_is_never_enlarged() {
        let made = make(&jpeg(120, 80)).expect("a preview");
        assert_eq!(decoded(&made.large).dimensions(), (120, 80));
        assert_eq!(decoded(&made.small).dimensions(), (120, 80));
    }

    /// A picture with alpha keeps it, as a PNG; one without is a JPEG.
    #[test]
    fn a_picture_with_alpha_is_kept_as_a_png() {
        let clear = RgbaImage::from_fn(64, 32, |x, _| image::Rgba([10, 20, 30, (x * 4) as u8]));
        let mut png = Vec::new();
        image::codecs::png::PngEncoder::new(&mut png)
            .write_image(clear.as_raw(), 64, 32, image::ExtendedColorType::Rgba8)
            .expect("encodes");
        let made = make(&png).expect("a preview");
        assert_eq!(made.encoding, Encoding::Png);
        assert!(decoded(&made.large).color().has_alpha());
        assert_eq!(
            make(&jpeg(64, 32)).expect("a preview").encoding,
            Encoding::Jpeg
        );
    }

    /// A phone's JPEG stored on its side with an EXIF orientation of 6
    /// ("turn 90° clockwise to view") is kept upright: 40×20 stored, 20×40
    /// in the preview.
    #[test]
    fn a_preview_is_turned_the_way_its_exif_says() {
        let stored = jpeg(40, 20);
        let mut exif = b"Exif\0\0MM\0\x2A\0\0\0\x08\0\x01".to_vec();
        exif.extend_from_slice(&[0x01, 0x12, 0x00, 0x03, 0, 0, 0, 1, 0x00, 0x06, 0, 0]);
        exif.extend_from_slice(&[0, 0, 0, 0]);
        let mut turned = stored[..2].to_vec();
        turned.extend_from_slice(&[0xFF, 0xE1]);
        turned.extend_from_slice(&((exif.len() + 2) as u16).to_be_bytes());
        turned.extend_from_slice(&exif);
        turned.extend_from_slice(&stored[2..]);
        let made = make(&turned).expect("a preview");
        assert_eq!(decoded(&made.large).dimensions(), (20, 40));
    }

    /// A few bytes of PNG may claim 81 megapixels: no preview is decoded
    /// for it, and it says why.
    #[test]
    fn a_picture_past_the_pixel_limit_makes_no_preview() {
        let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
        let mut header = Vec::new();
        header.extend_from_slice(&9000u32.to_be_bytes());
        header.extend_from_slice(&9000u32.to_be_bytes());
        header.extend_from_slice(&[8, 2, 0, 0, 0]);
        chunk(&mut png, b"IHDR", &header);
        chunk(
            &mut png,
            b"IDAT",
            &[0x78, 0x9C, 0x03, 0x00, 0x00, 0x00, 0x00, 0x01],
        );
        chunk(&mut png, b"IEND", b"");
        assert!(
            matches!(
                make(&png),
                Err(Unmade::TooLarge {
                    width: 9000,
                    height: 9000
                })
            ),
            "{:?}",
            make(&png).map(|made| made.encoding)
        );
        assert!(matches!(make(b"hello"), Err(Unmade::Unreadable(_))));
    }

    /// D825: kept as two files named by the row, with Spotlight's marker
    /// beside them; read back by [`Previews::of`]; counted by
    /// [`Previews::usage`].
    #[test]
    fn a_kept_preview_is_two_files_named_by_its_row() {
        let scratch = Scratch::new("kept");
        let told = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let heard = std::sync::Arc::clone(&told);
        let previews = Previews::new(scratch.0.join("previews"), move |id| {
            heard.lock().unwrap().push(id);
        });
        assert_eq!(previews.of(7), None);
        assert_eq!(previews.usage(), (0, 0));
        previews.keep(7, Source::Bytes(jpeg(300, 200)));
        previews.settled().recv().expect("settled");
        let kept = previews.of(7).expect("a preview");
        assert_eq!(kept.small.file_name().unwrap(), "7.small.jpg");
        assert_eq!(kept.large.file_name().unwrap(), "7.jpg");
        assert_eq!(
            decoded(&std::fs::read(&kept.small).unwrap()).width(),
            SMALL_SIDE
        );
        assert_eq!(*told.lock().unwrap(), [7]);
        assert_eq!(
            names(&scratch.0.join("previews")),
            [NEVER_INDEX, "7.jpg", "7.small.jpg"]
        );
        let (rows, bytes) = previews.usage();
        assert_eq!(rows, 1);
        assert!(bytes > 0);

        // A file it cannot decode is no preview, and nobody is told.
        previews.keep(8, Source::Bytes(b"not a picture".to_vec()));
        previews.settled().recv().expect("settled");
        assert_eq!(previews.of(8), None);
        assert_eq!(*told.lock().unwrap(), [7]);
    }

    /// D825: a removed row's preview goes, and no job puts it back — not
    /// one asked before the removal, nor one asked after.
    #[test]
    fn a_removed_row_never_gets_its_preview_back() {
        let scratch = Scratch::new("removed");
        let previews = Previews::new(scratch.0.clone(), |_| {});
        previews.keep(3, Source::Bytes(jpeg(50, 50)));
        previews.settled().recv().expect("settled");
        assert!(previews.of(3).is_some());
        previews.remove(3);
        assert_eq!(previews.of(3), None);
        previews.keep(3, Source::Bytes(jpeg(50, 50)));
        previews.settled().recv().expect("settled");
        assert_eq!(previews.of(3), None, "a removed row's preview came back");

        // A job finished after the removal is not put in place.
        let made = make(&jpeg(50, 50)).expect("a preview");
        previews.remove(4);
        assert!(!previews.folder.put(4, &made, 0).expect("writes"));
        assert_eq!(previews.of(4), None);
        assert_eq!(scratch.names(), [NEVER_INDEX], "a temporary was left");
    }

    /// D826: Delete takes every preview, says how many rows had one, and a
    /// job asked before it is dropped when it finishes after; one asked
    /// after is kept.
    #[test]
    fn delete_takes_every_preview_and_every_one_in_flight() {
        let scratch = Scratch::new("delete");
        let previews = Previews::new(scratch.0.clone(), |_| {});
        for id in [1, 2] {
            previews.keep(id, Source::Bytes(jpeg(40, 40)));
        }
        previews.settled().recv().expect("settled");
        let made = make(&jpeg(40, 40)).expect("a preview");
        let epoch = previews.folder.lock().epoch;
        assert_eq!(previews.remove_all(), 2);
        assert_eq!(previews.of(1), None);
        assert_eq!(previews.of(2), None);
        assert!(
            !previews.folder.put(5, &made, epoch).expect("writes"),
            "a preview asked for before the delete was put in place"
        );
        assert_eq!(previews.of(5), None);
        previews.keep(6, Source::Bytes(jpeg(40, 40)));
        previews.settled().recv().expect("settled");
        assert!(previews.of(6).is_some());
        assert_eq!(previews.remove_all(), 1);
        assert_eq!(scratch.names(), [NEVER_INDEX]);
    }

    /// D825: at launch, a file whose row is gone and any temporary go; the
    /// rows' previews, the marker and what is not a preview stay.
    #[test]
    fn a_launch_sweeps_what_no_row_owns() {
        let scratch = Scratch::new("sweep");
        for name in [
            "1.jpg",
            "1.small.jpg",
            "2.png",
            "2.small.png",
            ".2.99.0.tmp",
            NEVER_INDEX,
            "notes.txt",
        ] {
            std::fs::write(scratch.0.join(name), b"x").expect("write");
        }
        let previews = Previews::new(scratch.0.clone(), |_| {});
        assert_eq!(previews.sweep(&HashSet::from([1])), 3);
        assert_eq!(
            scratch.names(),
            [NEVER_INDEX, "1.jpg", "1.small.jpg", "notes.txt"]
        );
        assert!(previews.of(1).is_some());
    }

    #[test]
    fn a_file_name_says_its_row() {
        assert_eq!(row_of("12.jpg"), Some(12));
        assert_eq!(row_of("12.small.png"), Some(12));
        assert_eq!(row_of(".12.1.0.tmp"), None);
        assert_eq!(row_of("12.gif"), None);
        assert_eq!(row_of("x.jpg"), None);
        assert_eq!(row_of(NEVER_INDEX), None);
    }
}
