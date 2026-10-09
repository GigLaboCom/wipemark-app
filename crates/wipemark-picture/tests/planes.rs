//! A JPEG's stored planes (E12-R3, D301, D302): Y, Cb and Cr at their own
//! resolution from the `zune-jpeg` fork's `decode_planes`, held to the
//! decoder's own RGB raster (upsampled by `Planes::to_rgb`), to the
//! picture they were encoded from, and to the quantisation tables read
//! out of the file by a marker walk that shares no code with the decoder.
//! And the RGB raster every other path reads did not move.
//!
//! The fixtures are `fixtures/image/jpeg-planes/` (`make.py`, Pillow
//! 12.3.0), odd sizes so the MCU padding has to be cropped, and the
//! Gemini crops' JPEGs in `fixtures/image/gemini/`.

use std::io::Cursor;
use std::path::PathBuf;

use sha2::{Digest, Sha256};
use wipemark_image::ImageContainer;
use wipemark_picture::{decode, decode_with_planes, Decoded, PictureError, Skip};
use wipemark_pixels::{Plane, Planes, Quant, Raster, Sampling};

fn read(path: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/image")
        .join(path);
    std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

type Decode = fn(&[u8], ImageContainer) -> Result<Result<Decoded, Skip>, PictureError>;

/// `path` through `how`: [`decode`], the product's road, or
/// [`decode_with_planes`].
fn decoded_by(path: &str, how: Decode) -> Decoded {
    let bytes = read(path);
    let container = ImageContainer::sniff(&bytes).unwrap_or_else(|| panic!("{path}: no container"));
    how(&bytes, container)
        .unwrap_or_else(|e| panic!("{path}: {e}"))
        .unwrap_or_else(|skip| panic!("{path}: {skip:?}"))
}

fn decoded(path: &str) -> Decoded {
    decoded_by(path, decode)
}

/// The three-component JPEGs at odd sizes, and the Gemini crops' JPEGs
/// (1025, odd; 1040, a whole number of 16-pixel MCUs).
const COLOUR: [&str; 19] = [
    "jpeg-planes/rgb-37x23-q90-444.jpg",
    "jpeg-planes/rgb-37x23-q90-422.jpg",
    "jpeg-planes/rgb-37x23-q90-420.jpg",
    "jpeg-planes/rgb-129x65-q85-444.jpg",
    "jpeg-planes/rgb-129x65-q85-422.jpg",
    "jpeg-planes/rgb-129x65-q85-420.jpg",
    "jpeg-planes/rgb-129x65-q90-420-progressive.jpg",
    "jpeg-planes/rgb-37x23-q90-444-progressive.jpg",
    "jpeg-planes/rgb-129x65-q90-420-restart.jpg",
    "jpeg-planes/rgb-37x23-q90-420-one-dqt.jpg",
    "jpeg-planes/rgb-37x23-dqt16-420.jpg",
    "jpeg-planes/ycc-37x23-q100-420.jpg",
    "jpeg-planes/ycc-129x65-q100-420.jpg",
    "gemini/fine-1040-q98-444.jpg",
    "gemini/thinking-1040-q95-420.jpg",
    "gemini/torch-1025-q95-420.jpg",
    "gemini/torch-1025-q95-444.jpg",
    "gemini/victory-1025-q95-420.jpg",
    "gemini/victory-1040-q95-420.jpg",
];

const GREY: [&str; 2] = [
    "jpeg-planes/grey-37x23-q90.jpg",
    "jpeg-planes/grey-129x65-q85.jpg",
];

/// The one even size: 38 × 24 at 4:2:0, where the decoder's upsampler
/// reads a chroma sample from the MCU padding.
const EVEN: &str = "jpeg-planes/even-38x24-q90-420.jpg";

fn planes_of(path: &str) -> (Decoded, Planes) {
    let d = decoded_by(path, decode_with_planes);
    let planes = d
        .planes
        .clone()
        .unwrap_or_else(|| panic!("{path}: no planes"));
    (d, planes)
}

/// A grey JPEG's one plane, straight from the fork (`Decoded` leaves
/// grey's planes out, §4.3): what `Planes::to_rgb` does with `Gray`.
fn grey_planes(path: &str) -> Planes {
    let bytes = read(path);
    let stored = zune_jpeg::JpegDecoder::new(Cursor::new(&bytes[..]))
        .decode_planes()
        .unwrap_or_else(|e| panic!("{path}: {e:?}"));
    assert_eq!(stored.components.len(), 1, "{path}");
    let y = &stored.components[0];
    let luma = stored.qt[usize::from(y.qt_index)].unwrap();
    Planes::new(
        stored.width as u32,
        stored.height as u32,
        Sampling::Gray,
        Plane::new(
            y.width as u32,
            y.height as u32,
            y.samples.iter().map(|&s| u16::from(s)).collect(),
        )
        .unwrap(),
        None,
        None,
        Quant { luma, chroma: None },
    )
    .unwrap()
}

/// Every pixel where two rasters differ, with the largest difference.
fn differences(a: &Raster, b: &Raster) -> (Vec<(u32, u32)>, u16) {
    assert_eq!((a.width(), a.height()), (b.width(), b.height()));
    let mut at = Vec::new();
    let mut most = 0;
    for (i, (pa, pb)) in a
        .samples()
        .chunks_exact(3)
        .zip(b.samples().chunks_exact(3))
        .enumerate()
    {
        let d = pa
            .iter()
            .zip(pb)
            .map(|(&x, &y)| x.abs_diff(y))
            .max()
            .unwrap();
        if d > 0 {
            at.push(((i as u32) % a.width(), (i as u32) / a.width()));
            most = most.max(d);
        }
    }
    (at, most)
}

#[test]
fn the_planes_upsampled_are_the_decoders_rgb() {
    // Byte for byte, with no tolerance: the restatement is the decoder's
    // integer arithmetic in its order (vertical, then horizontal, for
    // 4:2:0), and an odd side never asks for a sample in the padding.
    for path in COLOUR {
        let (d, planes) = planes_of(path);
        let (at, most) = differences(&planes.to_rgb(), &d.raster);
        assert!(
            at.is_empty(),
            "{path}: {} pixels differ, by up to {most}, first at {:?}",
            at.len(),
            at.first()
        );
    }
    for path in GREY {
        let d = decoded(path);
        assert_eq!(grey_planes(path).to_rgb(), d.raster, "{path}");
    }
}

#[test]
fn an_even_side_differs_only_where_the_decoder_read_the_padding() {
    // At 38 × 24, 4:2:0, the last column and the last row are odd
    // outputs whose far neighbour is chroma sample 19 / row 12 — past the
    // plane, in the MCU padding, which the decoder reads and the cropped
    // plane does not have (it repeats sample 18 / row 11). Everywhere else
    // the two agree to the byte. `docs/architecture/zune-jpeg-pin.md`.
    let (d, planes) = planes_of(EVEN);
    let (w, h) = (d.raster.width(), d.raster.height());
    let (at, most) = differences(&planes.to_rgb(), &d.raster);
    for &(x, y) in &at {
        assert!(
            x == w - 1 || y == h - 1,
            "{EVEN}: differs inside, at ({x}, {y})"
        );
    }
    eprintln!(
        "{EVEN}: {} of {} edge pixels differ, by up to {most} levels",
        at.len(),
        w + h - 1
    );
}

/// `make.py`'s `ycc()`: the known YCbCr picture. Change both or neither.
fn ycc(x: u32, y: u32) -> [u32; 3] {
    [
        40 + (x * 5 + y * 3) % 170,
        64 + (x * 3 + y * 2) % 128,
        192 - (x * 2 + y * 5) % 128,
    ]
}

/// How far a stored 4:2:0 chroma sample may lie from the mean of the four
/// full-resolution samples it stands for, at quality 100 (every table
/// entry 1): half a level of the encoder's rounded mean, and the DCT's
/// rounding. `[tunable]`
const Q100_CHROMA_LEVELS: u32 = 2;

#[test]
fn a_420_plane_is_the_averaged_chroma_at_quality_100() {
    for path in [
        "jpeg-planes/ycc-37x23-q100-420.jpg",
        "jpeg-planes/ycc-129x65-q100-420.jpg",
    ] {
        let (d, planes) = planes_of(path);
        let (w, h) = (d.raster.width(), d.raster.height());
        assert_eq!(planes.sampling(), Sampling::H420, "{path}");
        let mut worst = 0.0_f64;
        for (channel, plane) in [(1, planes.cb().unwrap()), (2, planes.cr().unwrap())] {
            assert_eq!(
                (plane.width(), plane.height()),
                (w.div_ceil(2), h.div_ceil(2)),
                "{path}"
            );
            for cy in 0..plane.height() {
                for cx in 0..plane.width() {
                    // The 2 × 2 block, cut at the picture's edge — which
                    // is what the encoder's edge replication averages.
                    let (mut sum, mut n) = (0, 0);
                    for y in (2 * cy)..(2 * cy + 2).min(h) {
                        for x in (2 * cx)..(2 * cx + 2).min(w) {
                            sum += ycc(x, y)[channel];
                            n += 1;
                        }
                    }
                    let stored = u32::from(plane.get(cx, cy));
                    let off = (stored * n).abs_diff(sum);
                    // |stored − sum/n| ≤ L, without a division.
                    assert!(
                        off <= Q100_CHROMA_LEVELS * n,
                        "{path}: channel {channel} at ({cx}, {cy}) is {stored}, the mean {sum}/{n}"
                    );
                    worst = worst.max(f64::from(off) / f64::from(n));
                }
            }
        }
        // And luma is the picture's own, at full resolution.
        let y = planes.y();
        assert_eq!((y.width(), y.height()), (w, h), "{path}");
        for py in 0..h {
            for px in 0..w {
                let off = u32::from(y.get(px, py)).abs_diff(ycc(px, py)[0]);
                assert!(off <= Q100_CHROMA_LEVELS, "{path}: Y at ({px}, {py})");
            }
        }
        eprintln!("{path}: chroma within {worst:.2} of the 2 × 2 means");
    }
}

/// The zigzag scan: position `i` of a DQT segment is natural index
/// `ZIGZAG[i]` (ITU T.81, figure A.6).
const ZIGZAG: [usize; 64] = [
    0, 1, 8, 16, 9, 2, 3, 10, 17, 24, 32, 25, 18, 11, 4, 5, 12, 19, 26, 33, 40, 48, 41, 34, 27, 20,
    13, 6, 7, 14, 21, 28, 35, 42, 49, 56, 57, 50, 43, 36, 29, 22, 15, 23, 30, 37, 44, 51, 58, 59,
    52, 45, 38, 31, 39, 46, 53, 60, 61, 54, 47, 55, 62, 63,
];

/// What a file says before its first scan, read by walking its markers:
/// every quantisation table by slot (natural order), each component's
/// table, the frame's kind, the DQT segments and their precisions, and
/// the restart interval.
#[derive(Debug, Default)]
struct Header {
    tables: [Option<[u16; 64]>; 4],
    /// (id, h, v, table) per component.
    components: Vec<(u8, u8, u8, u8)>,
    sof: u8,
    dqt_segments: usize,
    precisions: Vec<u8>,
    restart: u16,
}

fn header(bytes: &[u8]) -> Header {
    let mut h = Header::default();
    assert_eq!(&bytes[..2], &[0xFF, 0xD8]);
    let mut at = 2;
    loop {
        assert_eq!(bytes[at], 0xFF, "a marker at {at}");
        let marker = bytes[at + 1];
        let len = usize::from(u16::from_be_bytes([bytes[at + 2], bytes[at + 3]]));
        let body = &bytes[at + 4..at + 2 + len];
        match marker {
            0xDB => {
                h.dqt_segments += 1;
                let mut i = 0;
                while i < body.len() {
                    let (precision, slot) = (body[i] >> 4, usize::from(body[i] & 0x0F));
                    h.precisions.push(precision);
                    i += 1;
                    let mut table = [0u16; 64];
                    for z in ZIGZAG {
                        table[z] = if precision == 0 {
                            i += 1;
                            u16::from(body[i - 1])
                        } else {
                            i += 2;
                            u16::from_be_bytes([body[i - 2], body[i - 1]])
                        };
                    }
                    h.tables[slot] = Some(table);
                }
            }
            0xC0..=0xC2 => {
                h.sof = marker - 0xC0;
                for c in body[6..].chunks_exact(3) {
                    h.components.push((c[0], c[1] >> 4, c[1] & 0x0F, c[2]));
                }
            }
            0xDD => h.restart = u16::from_be_bytes([body[0], body[1]]),
            0xDA => return h,
            _ => {}
        }
        at += 2 + len;
    }
}

#[test]
fn the_quantisation_tables_are_the_files() {
    for path in COLOUR.iter().chain([&EVEN]) {
        let file = header(&read(path));
        let (_, planes) = planes_of(path);
        let (y, cb, cr) = (file.components[0], file.components[1], file.components[2]);
        assert_eq!(cb.3, cr.3, "{path}: Cb and Cr share a table");
        // At quality 100 both tables are all ones; everywhere else they
        // differ, which is what tells a swapped index.
        if !path.contains("q100") {
            assert_ne!(
                file.tables[usize::from(y.3)],
                file.tables[usize::from(cb.3)],
                "{path}: the test needs two tables that differ"
            );
        }
        assert_eq!(
            *planes.quant(),
            Quant {
                luma: file.tables[usize::from(y.3)].unwrap(),
                chroma: file.tables[usize::from(cb.3)],
            },
            "{path}"
        );
    }
    for path in GREY {
        let file = header(&read(path));
        assert_eq!(
            Some(grey_planes(path).quant().luma),
            file.tables[usize::from(file.components[0].3)],
            "{path}"
        );
    }
}

#[test]
fn the_fixtures_are_what_their_names_say() {
    // What each variant is there to exercise, read off the file.
    let sampling = |path: &str| {
        let c = header(&read(path)).components;
        Sampling::of([c[0].1, c[1].1, c[2].1], [c[0].2, c[1].2, c[2].2])
    };
    assert_eq!(
        sampling("jpeg-planes/rgb-37x23-q90-444.jpg"),
        Sampling::H444
    );
    assert_eq!(
        sampling("jpeg-planes/rgb-37x23-q90-422.jpg"),
        Sampling::H422
    );
    assert_eq!(
        sampling("jpeg-planes/rgb-37x23-q90-420.jpg"),
        Sampling::H420
    );
    for path in COLOUR.iter().chain([&EVEN]) {
        assert_eq!(planes_of(path).1.sampling(), sampling(path), "{path}");
    }
    for path in [
        "jpeg-planes/rgb-129x65-q90-420-progressive.jpg",
        "jpeg-planes/rgb-37x23-q90-444-progressive.jpg",
    ] {
        assert_eq!(header(&read(path)).sof, 2, "{path}");
    }
    assert!(header(&read("jpeg-planes/rgb-129x65-q90-420-restart.jpg")).restart > 0);
    let one = header(&read("jpeg-planes/rgb-37x23-q90-420-one-dqt.jpg"));
    assert_eq!((one.dqt_segments, one.precisions.len()), (1, 2));
    let two = header(&read("jpeg-planes/rgb-37x23-q90-420.jpg"));
    assert_eq!((two.dqt_segments, two.precisions.len()), (2, 2));
    let sixteen = header(&read("jpeg-planes/rgb-37x23-dqt16-420.jpg"));
    assert_eq!((sixteen.sof, sixteen.precisions.clone()), (1, vec![1, 0]));
    assert!(sixteen.tables[0].unwrap().iter().any(|&q| q > 255));
    for path in GREY {
        assert_eq!(header(&read(path)).components.len(), 1, "{path}");
    }
}

/// `decode(..).raster` for every picture `wipemark-picture` reads in
/// these suites: width, height, layout, and the sha256 of its samples as
/// little-endian `u16`. Pinned with the code of `4b5ba17` and the
/// unpatched `zune-jpeg` 0.5.15 (on aarch64, so through its NEON
/// paths), before the fork was in: the planes leave the raster alone.
#[rustfmt::skip]
const RASTERS: [(&str, u32, u32, &str, &str); 30] = [
    ("gemini/anchor-green-1025.png", 1025, 1025, "rgb8", "cf193b9d75627133e6aeb127b0198e66c54cb9adc4b03ff4dd39825125cab5f8"),
    ("gemini/crying-1025.png", 1025, 1025, "rgb8", "c077b299e257d3db88db664f22a4e69749b895a5836f1877d9b1b79ecb11bb77"),
    ("gemini/crying-transparent-1025.png", 1025, 1025, "rgba8", "55e7a8ac94ab504e53b9b98b8935172d72431d8dc7f8f1ebb841a173d0abd85e"),
    ("gemini/cut-out-confetti-256.webp", 256, 256, "rgba8", "3b9207a0f779c562cf7c1b8ac43c3198a2324a83bd527ab0cbd18021f8e8343b"),
    ("gemini/fine-1040-q98-444.jpg", 1040, 1040, "rgb8", "bd54b44e42d9d9eb8b0bcd9c1396a324de09d09c7a8bb3a5f345f263e7a28b09"),
    ("gemini/scroll-1040-q90.webp", 1040, 1040, "rgb8", "541e90bc18e40d123cd59561f50fe69a6c06e9e2f95b9c4fa934f74bc46d1574"),
    ("gemini/thinking-1040-q95-420.jpg", 1040, 1040, "rgb8", "5137fd95f14e159ed80172bb05750947bbd1c7e23b224e6f3ad04440edc0ad21"),
    ("gemini/torch-1025-q95-420.jpg", 1025, 1025, "rgb8", "b227e2b28b5c719745de4a01277415d5f7c1941b0d6382b87a91d9c5880a9446"),
    ("gemini/torch-1025-q95-444.jpg", 1025, 1025, "rgb8", "1da3fe0d52b94448777afecfab783307865e32d665c384e7ddceebf1315b01c6"),
    ("gemini/torch-1025.png", 1025, 1025, "rgb8", "27fec97703e26865cab73416a134850e2ab17c7c0062b3c53b5d28f60b02402e"),
    ("gemini/victory-1025-q95-420.jpg", 1025, 1025, "rgb8", "50390f719721377672431221b331264ad2e7d5a3cda5d328c1d87ed8bde8cadc"),
    ("gemini/victory-1025-q98-420.jpg", 1025, 1025, "rgb8", "5902e0f1619a7d51d6032e85c739799efc00dad0a75bc28d3964e10b7f50c5d0"),
    ("gemini/victory-1025.png", 1025, 1025, "rgb8", "6774a0fe97ad994dbd5406052c962d57b52c444fd99ac3f78d55b0b4372a0995"),
    ("gemini/victory-1040-q95-420.jpg", 1040, 1040, "rgb8", "78dfceefb028d3076b0e5ba854634b35c0fcdec98b875e25ce6dedd0fda5e853"),
    ("jpeg-planes/even-38x24-q90-420.jpg", 38, 24, "rgb8", "4d2d833596fcc90f2a4348f2c153222841481560730e86b0a62679aebeafc2cd"),
    ("jpeg-planes/grey-129x65-q85.jpg", 129, 65, "rgb8", "a4d692ca3b7d338a9b265a2d8b31dfd88263639e949e22c54ac91d4d08eab130"),
    ("jpeg-planes/grey-37x23-q90.jpg", 37, 23, "rgb8", "23312f5b18fbbc075641a1ae01fce21515360f0226d619e5a1a8def41a5e914c"),
    ("jpeg-planes/rgb-129x65-q85-420.jpg", 129, 65, "rgb8", "f8c87172f218b4ec17b4640f8b84018573c78ce1dfa44887428d31a6155ad335"),
    ("jpeg-planes/rgb-129x65-q85-422.jpg", 129, 65, "rgb8", "338eb0d80b49504071a13f34349502e38ca98d20e73f1318689f8c87992a0078"),
    ("jpeg-planes/rgb-129x65-q85-444.jpg", 129, 65, "rgb8", "7f8f59ce34704078d6a1c03252d213f2606631966397f51622c82a9280a01013"),
    ("jpeg-planes/rgb-129x65-q90-420-progressive.jpg", 129, 65, "rgb8", "cc86a92f514880a9e9718792da63ccda3d29ab2370046203cd029c18b9a0f536"),
    ("jpeg-planes/rgb-129x65-q90-420-restart.jpg", 129, 65, "rgb8", "cc86a92f514880a9e9718792da63ccda3d29ab2370046203cd029c18b9a0f536"),
    ("jpeg-planes/rgb-37x23-dqt16-420.jpg", 37, 23, "rgb8", "495f731102b3be93e87e2264751ab2ec2c5c3edb00bff9f5834f2702df8836b4"),
    ("jpeg-planes/rgb-37x23-q90-420-one-dqt.jpg", 37, 23, "rgb8", "a535b7b24670938d7b34ae5ee1442bf3055555dde37319e4b28bfc69e89cbebd"),
    ("jpeg-planes/rgb-37x23-q90-420.jpg", 37, 23, "rgb8", "a535b7b24670938d7b34ae5ee1442bf3055555dde37319e4b28bfc69e89cbebd"),
    ("jpeg-planes/rgb-37x23-q90-422.jpg", 37, 23, "rgb8", "eecfffdef941be1dfe28b3e0054a233309616f93a4c7ab390121c348a44477e4"),
    ("jpeg-planes/rgb-37x23-q90-444-progressive.jpg", 37, 23, "rgb8", "ae82b4665a757c8794e813ceb5bce20ea16133a2bf56e4fa1552e78e40b66473"),
    ("jpeg-planes/rgb-37x23-q90-444.jpg", 37, 23, "rgb8", "ae82b4665a757c8794e813ceb5bce20ea16133a2bf56e4fa1552e78e40b66473"),
    ("jpeg-planes/ycc-129x65-q100-420.jpg", 129, 65, "rgb8", "8135b6891b47407e18f04d0246d46b7a618380c83c9c9c3a50b0b75c10e76e67"),
    ("jpeg-planes/ycc-37x23-q100-420.jpg", 37, 23, "rgb8", "3c0a573affdaa5cd409bfa652b3dc948ed99b6f04e006b4e9fa12e53a947ccf7"),
];

#[test]
fn the_rgb_raster_did_not_move() {
    // Through both roads: `decode`, which every surface takes, and
    // `decode_with_planes`, whose raster is the same.
    let roads: [Decode; 2] = [decode, decode_with_planes];
    for ((path, w, h, layout, sha), how) in
        RASTERS.into_iter().flat_map(|r| roads.map(|how| (r, how)))
    {
        let d = decoded_by(path, how);
        let mut hash = Sha256::new();
        for s in d.raster.samples() {
            hash.update(s.to_le_bytes());
        }
        assert_eq!(
            (
                d.raster.width(),
                d.raster.height(),
                d.raster.layout().id(),
                format!("{:x}", hash.finalize()).as_str()
            ),
            (w, h, layout, sha),
            "{path}"
        );
    }
}
