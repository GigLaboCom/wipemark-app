//! Malformed input is refused as a value naming what was wrong — never
//! a panic, never a partial output presented as clean.

mod support;

use support::*;
use wipemark_image::{inspect, strip, Defect, ImageContainer, ImageError, Scope, StripOptions};

/// Every injected case — the three that carry the real 7.7 KB manifest
/// rebuilt around the smallest C2PA JUMBF, so that cutting each at every
/// offset stays quadratic in hundreds of bytes rather than thousands —
/// and the one real file small enough.
fn small() -> Vec<(String, Vec<u8>)> {
    let mut v: Vec<_> = injected()
        .into_iter()
        .filter(|c| !c.name.ends_with("-c2pa"))
        .map(|c| (c.name.to_owned(), c.bytes))
        .collect();
    let j = tiny_jumbf();
    v.push((
        "png-c2pa".into(),
        png_with(&tiny_png(), &[png_chunk(b"caBX", &j)]),
    ));
    v.push((
        "jpeg-c2pa".into(),
        jpeg_with(&tiny_jpeg(), &[app11_jumbf(&j)]),
    ));
    v.push(("webp-c2pa".into(), webp_with(0, &[riff_chunk(b"C2PA", &j)])));
    v.push((
        "xmp-provenance-url.png".into(),
        fixture("xmp-provenance-url.png"),
    ));
    v.push(("tiny.jpg".into(), tiny_jpeg()));
    for (name, bytes) in &v {
        assert!(
            inspect(bytes).is_ok(),
            "{name} must be whole before it is cut"
        );
    }
    v
}

#[test]
fn the_tiny_manifest_is_c2pa_in_every_container() {
    for (name, bytes) in small().iter().filter(|(n, _)| n.ends_with("-c2pa")) {
        assert!(inspect(bytes).unwrap().has_c2pa(), "{name}");
    }
}

#[test]
fn every_truncation_is_refused() {
    for (name, bytes) in small() {
        for cut in 0..bytes.len() {
            let short = &bytes[..cut];
            assert!(
                inspect(short).is_err(),
                "{name} cut at {cut} was read as whole"
            );
            assert!(
                strip(short, &StripOptions::default()).is_err(),
                "{name} cut at {cut} was stripped as whole"
            );
        }
    }
}

#[test]
fn no_mutation_panics() {
    // Each byte flipped, zeroed and maxed; both scopes. A result either
    // way is fine; a panic is the failure.
    for (_, bytes) in small() {
        for at in 0..bytes.len() {
            for value in [bytes[at] ^ 0xFF, 0x00, 0xFF, bytes[at].wrapping_add(1)] {
                let mut m = bytes.clone();
                m[at] = value;
                let _ = inspect(&m);
                let _ = strip(&m, &StripOptions::default());
                let _ = strip(
                    &m,
                    &StripOptions {
                        scope: Scope::AllMetadata,
                    },
                );
            }
        }
    }
}

fn defect(bytes: &[u8]) -> Defect {
    match inspect(bytes) {
        Err(ImageError::Malformed { defect, .. }) => defect,
        other => panic!("expected a defect, got {other:?}"),
    }
}

#[test]
fn a_jpeg_without_eoi_is_refused() {
    let mut j = tiny_jpeg();
    j.truncate(j.len() - 2);
    assert_eq!(defect(&j), Defect::NoEnd);
}

#[test]
fn a_webp_whose_riff_size_lies_is_refused() {
    let mut w = webp_with(0, &[]);
    let lie = u32::from_le_bytes(w[4..8].try_into().unwrap()) + 100;
    w[4..8].copy_from_slice(&lie.to_le_bytes());
    assert!(matches!(defect(&w), Defect::RiffSize { .. }));
}

#[test]
fn a_png_chunk_that_runs_past_the_end_is_refused() {
    let mut p = tiny_png();
    // IHDR's length, raised so the chunk overruns everything after it.
    p[8..12].copy_from_slice(&0x7000_0000u32.to_be_bytes());
    assert_eq!(defect(&p), Defect::Truncated);
    p[8..12].copy_from_slice(&0x8000_0000u32.to_be_bytes());
    assert_eq!(defect(&p), Defect::BadLength);
}

#[test]
fn a_png_that_does_not_start_with_ihdr_is_refused() {
    let p = tiny_png();
    let mut q = p[..8].to_vec();
    q.extend_from_slice(&png_chunk(b"tEXt", b"k\0v"));
    q.extend_from_slice(&p[8..]);
    assert_eq!(defect(&q), Defect::HeaderNotFirst);
}

#[test]
fn a_compressed_text_that_cannot_be_read_is_refused() {
    let mut data = b"parameters\0\0".to_vec();
    data.extend_from_slice(b"\x78\x9cthis is not deflate");
    let p = png_with(&tiny_png(), &[png_chunk(b"zTXt", &data)]);
    assert_eq!(defect(&p), Defect::Inflate);
    assert!(strip(&p, &StripOptions::default()).is_err());
}

#[test]
fn a_stray_byte_between_jpeg_segments_is_refused() {
    // After SOI and the 69-byte DQT segment.
    let j = tiny_jpeg();
    let mut k = j[..71].to_vec();
    k.push(0x00);
    k.extend_from_slice(&j[71..]);
    assert_eq!(defect(&k), Defect::BadMarker(0x00));
}

#[test]
fn what_is_not_an_image_is_refused_as_such() {
    assert_eq!(inspect(b""), Err(ImageError::UnknownContainer));
    assert_eq!(inspect(b"%PDF-1.7"), Err(ImageError::UnknownContainer));
    assert_eq!(
        inspect(b"MM\0*\0\0\0\x08"),
        Err(ImageError::NotYet(ImageContainer::Tiff))
    );
}
