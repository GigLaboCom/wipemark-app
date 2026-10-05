//! `reframe`: the new file's structure inside the original's metadata
//! and rendering. Framing a file in itself is stripping it; framing a
//! re-encoded picture keeps every byte of colour and of kept metadata,
//! takes every byte of image data from the new file, and refuses what it
//! cannot carry.

mod support;

use support::*;
use wipemark_image::{
    inspect, reframe, strip, ImageContainer, ImageError, MetadataKind, Scope, StripOptions,
    Unsupported,
};

fn scope(scope: Scope) -> StripOptions {
    StripOptions { scope }
}

/// `reframe(x, x) == strip(x)`, bytes and report, for every fixture and
/// both scopes.
#[test]
fn framing_a_file_in_itself_is_stripping_it() {
    for case in all() {
        for s in Scope::ALL {
            let stripped = strip(&case.bytes, &scope(s)).unwrap();
            let framed = reframe(&case.bytes, &case.bytes, &scope(s))
                .unwrap_or_else(|e| panic!("{} {s:?}: {e}", case.name));
            assert_eq!(framed.0, stripped.0, "{} {s:?}", case.name);
            assert_eq!(framed.1, stripped.1, "{} {s:?}", case.name);
        }
    }
}

/// The same pixels, compressed differently: a stand-in for a restored
/// picture the encoder wrote.
fn png_encoded(
    pixels: &[u8],
    w: u32,
    h: u32,
    colour: png::ColorType,
    level: png::Compression,
) -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, w, h);
        enc.set_color(colour);
        enc.set_depth(png::BitDepth::Eight);
        enc.set_compression(level);
        let mut writer = enc.write_header().unwrap();
        writer.write_image_data(pixels).unwrap();
        writer.finish().unwrap();
    }
    out
}

fn rgba_pixels() -> Vec<u8> {
    (0..5 * 4 * 4).map(|i| (i * 37 % 251) as u8).collect()
}

/// A re-encoded PNG: its IDAT is the new file's, its colour profile and
/// its kept text are the original's to the byte, and C2PA is gone.
#[test]
fn a_re_encoded_png_keeps_its_colour_and_its_metadata() {
    let iccp = png_chunk(
        b"iCCP",
        b"profile\0\0\x78\x01\x01\x00\x00\xff\xff\x00\x00\x00\x01",
    );
    let comment = text("Comment", "a holiday");
    let original = png_with(
        &tiny_png(),
        &[
            iccp.clone(),
            comment.clone(),
            png_chunk(b"caBX", &tiny_jumbf()),
        ],
    );
    let new = png_encoded(
        &rgba_pixels(),
        5,
        4,
        png::ColorType::Rgba,
        png::Compression::Fast,
    );
    let (out, report) = reframe(&original, &new, &scope(Scope::AiProvenance)).unwrap();
    let chunks = png_chunks(&out);
    assert!(
        chunks.iter().any(|(_, c)| *c == iccp),
        "the colour profile moved"
    );
    assert!(
        chunks.iter().any(|(_, c)| *c == comment),
        "the comment moved"
    );
    assert!(!chunks.iter().any(|(t, _)| t == b"caBX"));
    assert_eq!(
        image_data(&out),
        image_data(&new),
        "the IDAT is not the new file's"
    );
    assert_eq!(raster(&out), raster(&new));
    assert!(!report.still_has_c2pa);
    assert_eq!(report.removed.len(), 1);
}

/// A palette picture whose new pixels are written as RGB loses the chunks
/// whose bytes speak of the palette, and says so.
#[test]
fn a_palette_png_that_became_rgb_loses_what_spoke_of_the_palette() {
    let mut original = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut original, 2, 2);
        enc.set_color(png::ColorType::Indexed);
        enc.set_depth(png::BitDepth::Eight);
        enc.set_palette(vec![0u8, 0, 0, 255, 255, 255]);
        enc.set_trns(vec![255u8, 128]);
        let mut writer = enc.write_header().unwrap();
        writer.write_image_data(&[0, 1, 1, 0]).unwrap();
        writer.finish().unwrap();
    }
    let original = png_with(
        &original,
        &[
            png_chunk(b"sBIT", &[8, 8, 8]),
            png_chunk(b"bKGD", &[1]),
            text("Comment", "kept"),
        ],
    );
    // The PLTE and tRNS the encoder wrote sit after IHDR; the injected
    // chunks are before them, which the specification allows for these.
    let rgba = [
        0u8, 0, 0, 255, 250, 250, 250, 128, 250, 250, 250, 128, 0, 0, 0, 255,
    ];
    let new = png_encoded(
        &rgba,
        2,
        2,
        png::ColorType::Rgba,
        png::Compression::default(),
    );
    let (out, report) = reframe(&original, &new, &scope(Scope::AiProvenance)).unwrap();
    let types: Vec<[u8; 4]> = png_chunks(&out).into_iter().map(|(t, _)| t).collect();
    for gone in [b"PLTE", b"tRNS", b"bKGD", b"sBIT"] {
        assert!(
            !types.contains(gone),
            "{:?} survived",
            std::str::from_utf8(gone)
        );
    }
    assert!(types.contains(b"tEXt"));
    let removed: Vec<&str> = report.removed.iter().map(|f| f.chunk.as_str()).collect();
    assert_eq!(removed, ["sBIT", "bKGD"]);
    assert_eq!(raster(&out), raster(&new));
}

/// Animation cannot be framed around one new picture.
#[test]
fn an_animated_png_is_refused() {
    let apng = png_with(
        &tiny_png(),
        &[png_chunk(b"acTL", &[0, 0, 0, 1, 0, 0, 0, 0])],
    );
    assert!(matches!(
        reframe(&apng, &tiny_png(), &StripOptions::default()),
        Err(ImageError::Unsupported {
            what: Unsupported::Reframe,
            ..
        })
    ));
    assert!(matches!(
        reframe(&tiny_png(), &tiny_jpeg(), &StripOptions::default()),
        Err(ImageError::Unsupported {
            what: Unsupported::Reframe,
            ..
        })
    ));
}

/// A WebP framed around new image data keeps its colour profile and its
/// camera EXIF, loses the AI XMP and its flag, and counts its RIFF size.
#[test]
fn a_webp_framed_keeps_its_flags_true_and_its_size_counted() {
    let exif = riff_chunk(b"EXIF", &exif_tiff("Canon", b""));
    let iccp = riff_chunk(b"ICCP", b"not a real profile");
    let original = webp_with(
        FLAG_EXIF | FLAG_XMP | FLAG_ICC,
        &[
            iccp.clone(),
            exif.clone(),
            riff_chunk(b"XMP ", xmp_ai().as_bytes()),
        ],
    );
    let pixels: Vec<u8> = (0..TINY_W * TINY_H * 4)
        .map(|i| (i * 13 % 251) as u8)
        .collect();
    let mut new = Vec::new();
    image_webp::WebPEncoder::new(&mut new)
        .encode(&pixels, TINY_W, TINY_H, image_webp::ColorType::Rgba8)
        .unwrap();
    let (out, report) = reframe(&original, &new, &StripOptions::default()).unwrap();
    let chunks = riff_chunks(&out);
    assert!(chunks.iter().any(|(_, c)| *c == iccp));
    assert!(chunks.iter().any(|(_, c)| *c == exif));
    assert!(!chunks.iter().any(|(t, _)| t == b"XMP "));
    assert_eq!(out[20] & FLAG_XMP, 0, "the XMP flag outlived its chunk");
    assert_eq!(out[20] & FLAG_EXIF, FLAG_EXIF);
    let size = u32::from_le_bytes(out[4..8].try_into().unwrap()) as usize;
    assert_eq!(size + 8, out.len());
    assert_eq!(image_data(&out), image_data(&new));
    assert_eq!(raster(&out), raster(&new));
    assert!(!report.still_has_ai_metadata);
}

/// A JPEG framed around another encoder's scans: the camera EXIF, the ICC
/// profile and the comment are the original's to the byte, the scans are
/// the new file's, and the AI XMP is gone.
#[test]
fn a_jpeg_framed_takes_the_new_scans_and_keeps_its_camera_data() {
    let exif = app1_exif(&exif_tiff("Canon", b""));
    let icc = segment(0xE2, b"ICC_PROFILE\0\x01\x01not a real profile");
    let comment = segment(0xFE, b"a holiday");
    let original = jpeg_with(
        &tiny_jpeg(),
        &[
            exif.clone(),
            icc.clone(),
            comment.clone(),
            app1_xmp(&xmp_ai()),
        ],
    );
    let new = base_jpeg();
    let (out, report) = reframe(&original, &new, &StripOptions::default()).unwrap();
    let segments: Vec<Vec<u8>> = jpeg_segments(&out)
        .into_iter()
        .map(|(m, p)| segment(m, &p))
        .collect();
    for kept in [&exif, &icc, &comment] {
        assert!(segments.contains(kept));
    }
    assert_eq!(
        jpeg_scans(&out),
        jpeg_scans(&new),
        "the scans are not the new file's"
    );
    assert_eq!(raster(&out), raster(&new));
    assert!(!report.still_has_ai_metadata);
    assert_eq!(
        report.removed.iter().map(|f| f.kind).collect::<Vec<_>>(),
        [MetadataKind::Xmp]
    );
    assert_eq!(inspect(&out).unwrap().container, ImageContainer::Jpeg);
}

/// An MPF file's offsets would point into the old pictures.
#[test]
fn an_mpf_jpeg_is_refused() {
    let original = jpeg_with(&tiny_jpeg(), &[app2_mpf()]);
    assert!(matches!(
        reframe(&original, &base_jpeg(), &StripOptions::default()),
        Err(ImageError::Unsupported {
            what: Unsupported::MultiPicture,
            ..
        })
    ));
}
