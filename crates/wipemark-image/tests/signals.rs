//! Each AI signal is found by `inspect`, removed by the default strip,
//! and gone from the second inspection; camera metadata is kept by the
//! default and removed by `AllMetadata`; colour is never removed.

mod support;

use support::*;
use wipemark_image::{
    inspect, strip, Generator, ImageContainer, ImageError, MetadataKind, Scope, Signal, SourceType,
    StripOptions, Unsupported, PIXEL_DOMAIN,
};

fn signals(bytes: &[u8]) -> Vec<Signal> {
    inspect(bytes)
        .unwrap()
        .findings
        .iter()
        .flat_map(|f| f.evidence.iter().map(|e| e.signal))
        .collect()
}

fn case(name: &str) -> Vec<u8> {
    all()
        .into_iter()
        .find(|c| c.name == name)
        .unwrap_or_else(|| panic!("no case {name}"))
        .bytes
}

const TRAINED: Signal = Signal::DigitalSourceType(SourceType::TrainedAlgorithmicMedia);

#[test]
fn each_ai_signal_is_found() {
    let expect: &[(&str, Signal)] = &[
        ("png-c2pa", Signal::C2paManifest),
        ("png-xmp-dst", TRAINED),
        (
            "png-parameters",
            Signal::GeneratorKey(Generator::StableDiffusionWebUi),
        ),
        (
            "png-parameters",
            Signal::GeneratorText(Generator::StableDiffusionWebUi),
        ),
        ("png-comfyui", Signal::GeneratorKey(Generator::ComfyUi)),
        ("png-software", Signal::GeneratorText(Generator::NovelAi)),
        (
            "png-ztxt-software",
            Signal::GeneratorText(Generator::AdobeFirefly),
        ),
        ("png-itxt-compressed-xmp", TRAINED),
        ("png-raw-iptc", TRAINED),
        ("jpeg-xmp-dst", TRAINED),
        ("jpeg-xmp-extended", TRAINED),
        ("jpeg-iptc-dst", TRAINED),
        (
            "jpeg-exif-infotext",
            Signal::GeneratorText(Generator::StableDiffusionWebUi),
        ),
        (
            "jpeg-com-midjourney",
            Signal::GeneratorText(Generator::Midjourney),
        ),
        ("jpeg-c2pa", Signal::C2paManifest),
        ("webp-c2pa", Signal::C2paManifest),
        ("webp-xmp-dst", TRAINED),
        (
            "webp-exif-infotext",
            Signal::GeneratorText(Generator::StableDiffusionWebUi),
        ),
        ("c2pa-jumbf.jpg", Signal::C2paManifest),
        ("xmp-provenance.jpg", Signal::C2paReference),
        ("xmp-provenance-url.png", Signal::C2paReference),
    ];
    for (name, signal) in expect {
        assert!(
            signals(&case(name)).contains(signal),
            "{name}: no {signal:?} in {:?}",
            signals(&case(name))
        );
    }
}

#[test]
fn each_ai_signal_is_removed_by_default_and_read_off_the_output() {
    for c in all().into_iter().filter(|c| c.ai) {
        let before = inspect(&c.bytes).unwrap();
        assert!(before.has_ai_metadata(), "{}", c.name);
        let (out, report) = strip(&c.bytes, &StripOptions::default()).unwrap();
        assert!(
            !report.still_has_ai_metadata,
            "{}: {:?}",
            c.name, report.kept
        );
        assert!(!report.still_has_c2pa, "{}", c.name);
        assert!(!inspect(&out).unwrap().has_ai_metadata(), "{}", c.name);
        assert!(
            report.removed.iter().all(|f| f.is_ai_provenance()),
            "{}",
            c.name
        );
    }
}

#[test]
fn the_report_agrees_with_a_fresh_inspection_of_the_output() {
    // Every default strip leaves nothing AI behind, so this cannot tell
    // a second inspection from a constant `false`; the unit test
    // `still_has_is_read_off_the_output` in lib.rs can.
    for c in all() {
        for scope in [Scope::AiProvenance, Scope::AllMetadata] {
            let (out, report) = strip(&c.bytes, &StripOptions { scope }).unwrap();
            let after = inspect(&out).unwrap();
            assert_eq!(report.still_has_c2pa, after.has_c2pa(), "{}", c.name);
            assert_eq!(
                report.still_has_ai_metadata,
                after.has_ai_metadata(),
                "{}",
                c.name
            );
        }
    }
}

#[test]
fn non_ai_metadata_is_kept_by_default_and_removed_by_all() {
    for name in ["png-camera", "jpeg-camera", "webp-camera", "exif-xmp.webp"] {
        let bytes = case(name);
        let report = inspect(&bytes).unwrap();
        assert!(!report.has_ai_metadata(), "{name}: {:?}", report.findings);
        assert!(!report.findings.is_empty(), "{name}");

        let (out, stripped) = strip(&bytes, &StripOptions::default()).unwrap();
        assert_eq!(out, bytes, "{name}");
        assert!(stripped.removed.is_empty());

        let (_, all) = strip(
            &bytes,
            &StripOptions {
                scope: Scope::AllMetadata,
            },
        )
        .unwrap();
        assert!(!all.removed.is_empty(), "{name}");
        assert!(
            all.kept.iter().all(|f| f.kind == MetadataKind::Rendering),
            "{name}: AllMetadata kept {:?}",
            all.kept
        );
    }
}

#[test]
fn colour_is_never_removed() {
    for name in [
        "png-camera",
        "jpeg-camera",
        "xmp-provenance.jpg",
        "exif-xmp.webp",
    ] {
        let bytes = case(name);
        let rendering = |fs: &[wipemark_image::MetadataFinding]| {
            fs.iter()
                .filter(|f| f.kind == MetadataKind::Rendering)
                .count()
        };
        let before = rendering(&inspect(&bytes).unwrap().findings);
        assert!(before > 0, "{name} has no colour block to protect");
        let (_, report) = strip(
            &bytes,
            &StripOptions {
                scope: Scope::AllMetadata,
            },
        )
        .unwrap();
        assert_eq!(rendering(&report.kept), before, "{name}");
    }
}

#[test]
fn the_real_files_say_what_they_carry() {
    // A C2PA manifest in APP11 beside a camera EXIF: the manifest goes,
    // the EXIF stays.
    let c2pa = inspect(&fixture("c2pa-jumbf.jpg")).unwrap();
    let kinds: Vec<_> = c2pa
        .findings
        .iter()
        .map(|f| (f.kind, f.chunk.as_str()))
        .collect();
    assert_eq!(
        kinds,
        [(MetadataKind::C2pa, "APP11"), (MetadataKind::Exif, "APP1")]
    );
    let (_, r) = strip(&fixture("c2pa-jumbf.jpg"), &StripOptions::default()).unwrap();
    assert_eq!(r.removed.len(), 1);
    assert_eq!(
        r.kept.iter().map(|f| f.kind).collect::<Vec<_>>(),
        [MetadataKind::Exif]
    );

    // A manifest already removed, still pointed at from XMP: the packet
    // goes; Photoshop's IPTC, the ICC profile and EXIF stay.
    let (_, r) = strip(&fixture("xmp-provenance.jpg"), &StripOptions::default()).unwrap();
    assert_eq!(
        r.removed.iter().map(|f| f.kind).collect::<Vec<_>>(),
        [MetadataKind::Xmp]
    );
    let kept: Vec<_> = r.kept.iter().map(|f| f.kind).collect();
    assert_eq!(
        kept,
        [
            MetadataKind::Exif,
            MetadataKind::Iptc,
            MetadataKind::Rendering
        ]
    );

    // A WebP photo with EXIF, XMP and Photoshop's own chunk: nothing AI.
    let webp = inspect(&fixture("exif-xmp.webp")).unwrap();
    let chunks: Vec<_> = webp.findings.iter().map(|f| f.chunk.as_str()).collect();
    assert_eq!(chunks, ["ICCP", "EXIF", "XMP ", "PSAI"]);
}

#[test]
fn the_report_never_quotes_the_value() {
    // Evidence names the signature that matched; the prompt is the
    // user's text and never appears in a report.
    let r = inspect(&case("png-parameters")).unwrap();
    for e in r.findings.iter().flat_map(|f| &f.evidence) {
        assert!(!e.matched.contains("cat in a hat"));
        assert!(!e.field.contains("cat in a hat"));
    }
}

#[test]
fn vp8x_flags_follow_what_remains() {
    for (name, flag) in [
        ("webp-xmp-dst", FLAG_XMP),
        ("webp-exif-infotext", FLAG_EXIF),
    ] {
        let bytes = case(name);
        assert_eq!(bytes[20] & flag, flag, "{name}: the fixture sets the flag");
        let (out, _) = strip(&bytes, &StripOptions::default()).unwrap();
        assert_eq!(out[20] & flag, 0, "{name}: the flag outlived its chunk");
        assert_eq!(out[20] | flag, bytes[20], "{name}: another flag moved");
        let size = u32::from_le_bytes(out[4..8].try_into().unwrap()) as usize;
        assert_eq!(size + 8, out.len(), "{name}: the RIFF size is wrong");
    }
    // The real WebP: everything but ICCP goes, and both bits with it.
    let bytes = fixture("exif-xmp.webp");
    let (out, _) = strip(
        &bytes,
        &StripOptions {
            scope: Scope::AllMetadata,
        },
    )
    .unwrap();
    assert_eq!(out[20], bytes[20] & !(FLAG_EXIF | FLAG_XMP));
    assert_eq!(out[20] & FLAG_ICC, FLAG_ICC);
}

#[test]
fn a_flag_that_was_already_wrong_is_not_this_passes_to_fix() {
    // EXIF flagged but absent; a C2PA chunk removed. The EXIF bit stays
    // as the input had it.
    let bytes = webp_with(FLAG_EXIF, &[riff_chunk(b"C2PA", &jumbf())]);
    let (out, _) = strip(&bytes, &StripOptions::default()).unwrap();
    assert_eq!(out[20], bytes[20]);
}

#[test]
fn jpeg_xmp_leaves_whole() {
    // The DST is in the extension only; the main packet goes with it.
    let r = inspect(&case("jpeg-xmp-extended")).unwrap();
    let xmp: Vec<_> = r
        .findings
        .iter()
        .filter(|f| f.kind == MetadataKind::Xmp)
        .collect();
    assert_eq!(xmp.len(), 2);
    assert!(xmp.iter().all(|f| f.is_ai_provenance()));
    let (out, report) = strip(&case("jpeg-xmp-extended"), &StripOptions::default()).unwrap();
    assert_eq!(report.removed.len(), 2);
    assert_eq!(out, tiny_jpeg());
}

#[test]
fn mpf_offsets_are_never_moved() {
    let jpg = tiny_jpeg();
    // A manifest after the MPF header: removing it would move the
    // secondary pictures. Refused, and nothing is written.
    let after = jpeg_with(&jpg, &[app2_mpf(), app11_jumbf(&jumbf())]);
    let err = strip(&after, &StripOptions::default()).unwrap_err();
    assert!(matches!(
        err,
        ImageError::Unsupported {
            container: ImageContainer::Jpeg,
            what: Unsupported::MultiPicture,
            ..
        }
    ));
    // Before it: every offset MPF holds moves together. Allowed.
    let before = jpeg_with(&jpg, &[app11_jumbf(&jumbf()), app2_mpf()]);
    let (out, report) = strip(&before, &StripOptions::default()).unwrap();
    assert!(!report.still_has_c2pa);
    assert_eq!(out, jpeg_with(&jpg, &[app2_mpf()]));
    // And what follows EOI in an MPF file is a picture, kept by every scope.
    let mut with_trailer = jpeg_with(&jpg, &[app2_mpf()]);
    with_trailer.extend_from_slice(&tiny_jpeg());
    let (out, _) = strip(
        &with_trailer,
        &StripOptions {
            scope: Scope::AllMetadata,
        },
    )
    .unwrap();
    assert_eq!(out, with_trailer);
}

#[test]
fn a_trailer_without_mpf_is_metadata() {
    let mut png = tiny_png();
    png.extend_from_slice(b"appended bytes");
    let r = inspect(&png).unwrap();
    assert_eq!(r.findings.last().map(|f| f.chunk.as_str()), Some("trailer"));
    let (out, _) = strip(
        &png,
        &StripOptions {
            scope: Scope::AllMetadata,
        },
    )
    .unwrap();
    assert_eq!(out, tiny_png());
    let (out, _) = strip(&png, &StripOptions::default()).unwrap();
    assert_eq!(out, png);
}

#[test]
fn every_report_keeps_the_pixel_domain_on_the_third_shelf() {
    for c in all() {
        let r = inspect(&c.bytes).unwrap();
        assert!(r.not_established.contains(&PIXEL_DOMAIN), "{}", c.name);
        for scope in [Scope::AiProvenance, Scope::AllMetadata] {
            let (_, s) = strip(&c.bytes, &StripOptions { scope }).unwrap();
            assert!(s.not_established.contains(&PIXEL_DOMAIN), "{}", c.name);
            assert_eq!(s.not_established.len(), 3, "{}", c.name);
        }
    }
}
