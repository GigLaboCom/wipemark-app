//! The protections the host verification of E11-1 found unguarded
//! (`docs/plan/E11-3-image-test-gaps.md`): an unknown critical PNG
//! chunk, a C2PA manifest across several APP11 segments, the MP Index
//! after a removal before it, and the orientation a removed EXIF block
//! carried. Each test goes red with its protection deleted
//! (`docs/plan/reports/E11-3-mutate.py`).

mod support;

use support::*;
use wipemark_image::{
    inspect, strip, ImageContainer, ImageError, MetadataKind, Scope, StripOptions, Unsupported,
};

const ALL: StripOptions = StripOptions {
    scope: Scope::AllMetadata,
};

fn default() -> StripOptions {
    StripOptions::default()
}

// ------------------------------------------------- unknown critical chunk

/// A critical chunk this build does not know — Apple's `CgBI` name, an
/// invented `ABCD` — is structure: by the PNG specification a decoder
/// that does not know it must refuse the file, so it is not decoration.
/// Never listed, and kept by every scope.
#[test]
fn an_unknown_critical_chunk_is_structure() {
    for ty in [b"CgBI", b"ABCD"] {
        let chunk = png_chunk(ty, b"\x50\x00\x20\x06");
        let bytes = png_with(&tiny_png(), &[chunk.clone(), text("Comment", "a holiday")]);
        let name = std::str::from_utf8(ty).unwrap();
        let report = inspect(&bytes).unwrap();
        assert!(
            report.findings.iter().all(|f| f.chunk != name),
            "{name} listed: {:?}",
            report.findings
        );
        let (out, stripped) = strip(&bytes, &ALL).unwrap();
        assert_eq!(stripped.removed.len(), 1, "{name}: {:?}", stripped.removed);
        assert_eq!(out, png_with(&tiny_png(), &[chunk]), "{name}");
    }
    // The rule is the case of the first letter: the same name ancillary
    // is metadata like any other unknown chunk.
    let bytes = png_with(&tiny_png(), &[png_chunk(b"aBCD", b"x")]);
    let report = inspect(&bytes).unwrap();
    assert_eq!(
        report.findings.iter().map(|f| f.kind).collect::<Vec<_>>(),
        [MetadataKind::Other]
    );
    assert_eq!(strip(&bytes, &ALL).unwrap().0, tiny_png());
}

// ------------------------------------------------ APP11 across segments

/// A JPEG XT box larger than one segment continues in the next under the
/// same box instance, and the continuation carries only the shared
/// header — no label. It is still the manifest, and leaves with it.
#[test]
fn a_c2pa_manifest_across_app11_segments_leaves_whole() {
    let [first, second] = app11_split([0, 1], true);
    // The fixture says what it claims: the label is in the first segment
    // alone, and the second's head has no `c2pa` for a reader to find.
    assert!(first.windows(4).any(|w| w == b"c2pa"));
    assert!(!second.windows(4).any(|w| w == b"c2pa"));

    let bytes = jpeg_with(&tiny_jpeg(), &[first, second]);
    let report = inspect(&bytes).unwrap();
    let app11: Vec<_> = report
        .findings
        .iter()
        .filter(|f| f.chunk == "APP11")
        .collect();
    assert_eq!(app11.len(), 2);
    assert!(
        app11
            .iter()
            .all(|f| f.kind == MetadataKind::C2pa && f.is_c2pa()),
        "{app11:?}"
    );
    let (out, stripped) = strip(&bytes, &default()).unwrap();
    assert_eq!(stripped.removed.len(), 2);
    assert!(!stripped.still_has_c2pa);
    assert_eq!(out, tiny_jpeg());
}

/// The grouping is by box instance, not "every JUMBF is C2PA": an
/// unlabelled box under another instance stays what it is.
#[test]
fn an_unlabelled_jumbf_of_another_instance_is_not_c2pa() {
    let [first, second] = app11_split([0, 1], true);
    let [other, _] = app11_split([0, 2], false);
    let bytes = jpeg_with(&tiny_jpeg(), &[first, other.clone(), second]);
    let report = inspect(&bytes).unwrap();
    let kinds: Vec<_> = report.findings.iter().map(|f| f.kind).collect();
    assert_eq!(
        kinds,
        [MetadataKind::C2pa, MetadataKind::Other, MetadataKind::C2pa]
    );
    assert!(!report.findings[1].is_ai_provenance());
    let (out, _) = strip(&bytes, &default()).unwrap();
    assert_eq!(out, jpeg_with(&tiny_jpeg(), &[other]));
}

// ------------------------------------------------------------ MP Index

/// A JPEG with an MPF index: `before` after SOI, then the header, then the
/// rest of the first picture, then a second picture — every size and
/// offset in the index true.
fn mpf_jpeg(big: bool, before: &[Vec<u8>]) -> Vec<u8> {
    let picture = tiny_jpeg();
    let build = |primary: u32, offset: u32| {
        let mut segments = before.to_vec();
        segments.push(app2_mpf_index(big, primary, picture.len() as u32, offset));
        let mut file = jpeg_with(&picture, &segments);
        file.extend_from_slice(&picture);
        file
    };
    let probe = build(0, 0);
    let primary = probe.len() - picture.len();
    let tiff = tiff_of(&probe);
    build(primary as u32, (primary - tiff) as u32)
}

/// Where the MPF header's TIFF stream starts — found by walking the
/// segments, not by the crate's parser.
fn tiff_of(b: &[u8]) -> usize {
    let mut pos = 2;
    loop {
        let len = u16::from_be_bytes([b[pos + 2], b[pos + 3]]) as usize;
        if b[pos + 1] == 0xE2 && &b[pos + 4..pos + 8] == b"MPF\0" {
            return pos + 8;
        }
        pos += 2 + len;
    }
}

fn field(b: &[u8], at: usize, big: bool) -> u32 {
    let bytes: [u8; 4] = b[at..at + 4].try_into().unwrap();
    if big {
        u32::from_be_bytes(bytes)
    } else {
        u32::from_le_bytes(bytes)
    }
}

/// After a removal before the header, the index's size of the first
/// picture is the first picture's size again, in the index's own byte
/// order; the second picture's size and offset are untouched and the
/// offset still lands on its SOI; and no other byte moved.
#[test]
fn the_mp_index_follows_a_removal_before_it() {
    let picture = tiny_jpeg();
    for big in [true, false] {
        let bytes = mpf_jpeg(big, &[app11_jumbf(&tiny_jumbf()), app1_xmp(&xmp_ai())]);
        let tiff = tiff_of(&bytes);
        let primary = bytes.len() - picture.len();
        assert_eq!(field(&bytes, tiff + MPF_PRIMARY_SIZE, big), primary as u32);
        let offset = field(&bytes, tiff + MPF_SECONDARY_OFFSET, big) as usize;
        assert_eq!(&bytes[tiff + offset..tiff + offset + 2], [0xFF, 0xD8]);

        let (out, report) = strip(&bytes, &default()).unwrap();
        assert_eq!(report.removed.len(), 2, "big {big}");
        let tiff_out = tiff_of(&out);
        let primary_out = out.len() - picture.len();
        assert_eq!(
            field(&out, tiff_out + MPF_PRIMARY_SIZE, big),
            primary_out as u32,
            "big {big}: the first picture's size is stale"
        );
        assert_eq!(
            field(&out, tiff_out + MPF_SECONDARY_OFFSET, big) as usize,
            offset
        );
        assert_eq!(
            field(&out, tiff_out + MPF_SECONDARY_SIZE, big) as usize,
            picture.len()
        );
        assert_eq!(&out[tiff_out + offset..], &picture[..]);

        // The output is the input with the reported ranges cut out, and
        // those four bytes.
        let mut expected = Vec::new();
        let mut at = 0usize;
        for f in &report.removed {
            expected.extend_from_slice(&bytes[at..f.offset as usize]);
            at = (f.offset + f.len) as usize;
        }
        expected.extend_from_slice(&bytes[at..]);
        assert_eq!(out.len(), expected.len());
        let moved: Vec<usize> = (0..out.len()).filter(|&i| out[i] != expected[i]).collect();
        let size = tiff_out + MPF_PRIMARY_SIZE..tiff_out + MPF_PRIMARY_SIZE + 4;
        assert!(
            moved.iter().all(|i| size.contains(i)),
            "big {big}: {moved:?}"
        );
        assert_eq!(raster(&out), raster(&bytes));
    }
}

/// An index whose size cannot be read, or would go below zero, refuses a
/// removal before it — rather than leaving a field nobody checked. With
/// nothing to remove the file comes back whole.
#[test]
fn an_mp_index_that_cannot_be_read_refuses_a_removal() {
    let mut past = b"MPF\0MM\0*".to_vec();
    past.extend_from_slice(&0xFFFFu32.to_be_bytes());
    let mut order = b"MPF\0XX\0*".to_vec();
    order.extend_from_slice(&8u32.to_be_bytes());
    order.extend_from_slice(&[0, 0]);
    let unreadable = [segment(0xE2, &past), segment(0xE2, &order)];
    for mpf in unreadable {
        let bytes = jpeg_with(&tiny_jpeg(), &[app1_xmp(&xmp_ai()), mpf.clone()]);
        let at = 2 + app1_xmp(&xmp_ai()).len() as u64;
        assert_eq!(
            strip(&bytes, &default()).unwrap_err(),
            ImageError::Unsupported {
                container: ImageContainer::Jpeg,
                offset: at,
                what: Unsupported::MultiPicture,
            }
        );
        let whole = jpeg_with(&tiny_jpeg(), &[mpf]);
        assert_eq!(strip(&whole, &ALL).unwrap().0, whole);
    }

    // A size smaller than what a removal takes: refused, not wrapped.
    let segment = app2_mpf_index(true, 1, 0, 0);
    let bytes = jpeg_with(&tiny_jpeg(), &[app1_xmp(&xmp_ai()), segment]);
    assert!(matches!(
        strip(&bytes, &default()),
        Err(ImageError::Unsupported {
            what: Unsupported::MultiPicture,
            ..
        })
    ));
}

// ---------------------------------------------------------- orientation

fn ai_exif(big: bool, orientation: u16) -> Vec<u8> {
    exif_oriented(big, orientation, &user_comment_unicode(INFOTEXT))
}

fn exif_header(tiff: &[u8]) -> Vec<u8> {
    let mut b = b"Exif\0\0".to_vec();
    b.extend_from_slice(tiff);
    b
}

/// An EXIF block that names a generator leaves whole under the default
/// scope, its Orientation with it, and the report says which — in every
/// container, in either byte order, behind an `Exif\0\0` or not.
#[test]
fn a_removed_orientation_is_reported() {
    let png = tiny_png();
    let cases: Vec<(&str, Vec<u8>, u16)> = vec![
        (
            "jpeg-be",
            jpeg_with(&tiny_jpeg(), &[app1_exif(&ai_exif(true, 6))]),
            6,
        ),
        (
            "jpeg-le",
            jpeg_with(&tiny_jpeg(), &[app1_exif(&ai_exif(false, 8))]),
            8,
        ),
        (
            "png-exif",
            png_with(&png, &[png_chunk(b"eXIf", &ai_exif(true, 3))]),
            3,
        ),
        (
            "png-raw-profile",
            png_with(
                &png,
                &[ztxt(
                    "Raw profile type exif",
                    &raw_profile("exif", &exif_header(&ai_exif(false, 5))),
                )],
            ),
            5,
        ),
        (
            "webp-exif",
            webp_with(
                FLAG_EXIF,
                &[riff_chunk(b"EXIF", &exif_header(&ai_exif(true, 7)))],
            ),
            7,
        ),
    ];
    for (name, bytes, orientation) in cases {
        let (_, report) = strip(&bytes, &default()).unwrap();
        assert_eq!(
            report.removed.iter().map(|f| f.kind).collect::<Vec<_>>(),
            [MetadataKind::Exif],
            "{name}: the fixture's EXIF is not AI provenance"
        );
        assert_eq!(report.orientation_removed, Some(orientation), "{name}");
        assert!(
            report
                .to_json()
                .contains(&format!("\"orientation_removed\":{orientation},")),
            "{name}"
        );
    }

    // A camera's oriented EXIF: kept by default, and nothing is said;
    // taken by `AllMetadata`, and said.
    let camera = jpeg_with(
        &tiny_jpeg(),
        &[app1_exif(&exif_oriented(true, 6, b"")), app1_xmp(&xmp_ai())],
    );
    let (_, kept) = strip(&camera, &default()).unwrap();
    assert_eq!(kept.removed.len(), 1);
    assert_eq!(kept.orientation_removed, None);
    let (_, taken) = strip(&camera, &ALL).unwrap();
    assert_eq!(taken.orientation_removed, Some(6));
}

/// Orientation 1 is "as stored": no rotation went, and none is reported.
/// Nor is a value no viewer acts on.
#[test]
fn orientation_one_is_not_a_rotation() {
    for value in [1, 0, 9] {
        let bytes = jpeg_with(&tiny_jpeg(), &[app1_exif(&ai_exif(true, value))]);
        let (_, report) = strip(&bytes, &default()).unwrap();
        assert_eq!(report.removed.len(), 1, "{value}");
        assert_eq!(report.orientation_removed, None, "{value}");
    }
}
