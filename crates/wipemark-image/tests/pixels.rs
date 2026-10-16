//! The gate (OV §11): pixels never change. For every fixture and both
//! scopes, the decoded raster's sha256 is identical before and after
//! `strip`, and the image-data bytes — IDAT, the scans, VP8/VP8L — are
//! byte-identical, read by walkers that share no code with the crate.

mod support;

use support::{all, image_data, raster};
use wipemark_image::{inspect, strip, Scope, StripOptions};

const SCOPES: [Scope; 2] = [Scope::AiProvenance, Scope::AllMetadata];

#[test]
fn pixels_never_change() {
    for case in all() {
        let before = raster(&case.bytes);
        for scope in SCOPES {
            let (out, _) = strip(&case.bytes, &StripOptions { scope })
                .unwrap_or_else(|e| panic!("{}: {e}", case.name));
            assert_eq!(
                raster(&out),
                before,
                "{} under {scope:?}: the raster moved",
                case.name
            );
        }
    }
}

#[test]
fn image_data_is_byte_identical() {
    for case in all() {
        let before = image_data(&case.bytes);
        assert!(
            !before.is_empty(),
            "{}: the walker found no image data",
            case.name
        );
        for scope in SCOPES {
            let (out, _) = strip(&case.bytes, &StripOptions { scope }).unwrap();
            assert_eq!(image_data(&out), before, "{} under {scope:?}", case.name);
        }
    }
}

#[test]
fn strip_is_idempotent() {
    for case in all() {
        for scope in SCOPES {
            let options = StripOptions { scope };
            let (once, _) = strip(&case.bytes, &options).unwrap();
            let (twice, report) = strip(&once, &options).unwrap();
            assert_eq!(twice, once, "{} under {scope:?}", case.name);
            assert!(
                report.removed.is_empty(),
                "{}: a second pass removed something",
                case.name
            );
        }
    }
}

#[test]
fn something_to_strip_makes_the_file_shorter_and_nothing_to_strip_keeps_it_whole() {
    for case in all() {
        let (out, report) = strip(&case.bytes, &StripOptions::default()).unwrap();
        if case.ai {
            assert!(!report.removed.is_empty(), "{}: nothing removed", case.name);
            assert!(out.len() < case.bytes.len(), "{}", case.name);
        } else {
            assert!(
                report.removed.is_empty(),
                "{}: {:?}",
                case.name,
                report.removed
            );
            assert_eq!(
                out, case.bytes,
                "{}: nothing to strip, yet a byte moved",
                case.name
            );
        }
    }
}

#[test]
fn every_removed_block_is_exactly_the_bytes_that_went() {
    // What the report says was removed is what is missing, and nothing
    // else is: the output is the input with those ranges cut out —
    // except a WebP's RIFF size and VP8X flags, which `webp.rs` owns.
    for case in all() {
        let (out, report) = strip(
            &case.bytes,
            &StripOptions {
                scope: Scope::AllMetadata,
            },
        )
        .unwrap();
        let mut expected = Vec::new();
        let mut at = 0usize;
        for f in &report.removed {
            let start = usize::try_from(f.offset).unwrap();
            expected.extend_from_slice(&case.bytes[at..start]);
            at = start + usize::try_from(f.len).unwrap();
        }
        expected.extend_from_slice(&case.bytes[at..]);
        assert_eq!(out.len(), expected.len(), "{}", case.name);
        let differing: Vec<usize> = (0..out.len()).filter(|&i| out[i] != expected[i]).collect();
        if case.bytes.starts_with(b"RIFF") {
            assert!(
                differing.iter().all(|&i| (4..8).contains(&i) || i == 20),
                "{}: bytes other than the RIFF size and the VP8X flags moved: {differing:?}",
                case.name
            );
        } else {
            assert!(differing.is_empty(), "{}: {differing:?}", case.name);
        }
        // And the report reads off the output.
        assert_eq!(
            report.kept,
            inspect(&out).unwrap().findings,
            "{}",
            case.name
        );
    }
}
