//! A picture file through both passes: a marked PNG and a marked lossless
//! WebP come back restored, losslessly, with every other sample, every
//! byte of colour and every kept block unchanged; a picture with no mark
//! is `strip`'s output to the byte; an animation is said not to be
//! examined. JPEG and lossy WebP are `tests/lossy.rs`.

#[path = "../../wipemark-pixels/tests/support/mod.rs"]
mod pixels_support;

use std::io::Cursor;

use pixels_support::{composite_at, picture, small_row, synthetic_catalogue, synthetic_v1, Kind};
use wipemark_image::{Scope, StripOptions};
use wipemark_picture::{
    clean, encode_like, inspect, prove, Encoding, NotExamined, PictureError, PictureOptions,
    PngInfo, Proof, Source, Visible,
};
use wipemark_pixels::{Catalogue, ExamineOptions, Layout, PixelRect, Raster};

const W: u32 = 320;
const H: u32 = 240;

// ------------------------------------------------------------- files

fn crc32(bytes: &[u8]) -> u32 {
    let mut c = 0xFFFF_FFFFu32;
    for &b in bytes {
        c ^= u32::from(b);
        for _ in 0..8 {
            c = if c & 1 == 1 {
                0xEDB8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
        }
    }
    !c
}

fn chunk(ty: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut c = (data.len() as u32).to_be_bytes().to_vec();
    c.extend_from_slice(ty);
    c.extend_from_slice(data);
    let mut crc = ty.to_vec();
    crc.extend_from_slice(data);
    c.extend_from_slice(&crc32(&crc).to_be_bytes());
    c
}

/// `chunks` after the `IHDR` of `png`.
fn with_chunks(png: &[u8], chunks: &[Vec<u8>]) -> Vec<u8> {
    let at = 8 + 12 + 13;
    let mut out = png[..at].to_vec();
    for c in chunks {
        out.extend_from_slice(c);
    }
    out.extend_from_slice(&png[at..]);
    out
}

fn png_of(raster: &Raster) -> Vec<u8> {
    let colour = if raster.layout() == Layout::Rgba8 {
        png::ColorType::Rgba
    } else {
        png::ColorType::Rgb
    };
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, raster.width(), raster.height());
        enc.set_color(colour);
        enc.set_depth(png::BitDepth::Eight);
        let mut w = enc.write_header().unwrap();
        let bytes: Vec<u8> = raster.samples().iter().map(|&s| s as u8).collect();
        w.write_image_data(&bytes).unwrap();
        w.finish().unwrap();
    }
    out
}

fn webp_of(raster: &Raster) -> Vec<u8> {
    let bytes: Vec<u8> = raster.samples().iter().map(|&s| s as u8).collect();
    let mut out = Vec::new();
    image_webp::WebPEncoder::new(&mut out)
        .encode(
            &bytes,
            raster.width(),
            raster.height(),
            image_webp::ColorType::Rgb8,
        )
        .unwrap();
    out
}

/// The output's pixels through decoders the code under test does not use
/// for its own comparison.
fn png_pixels(bytes: &[u8]) -> Vec<u8> {
    let mut d = png::Decoder::new(Cursor::new(bytes));
    d.set_transformations(png::Transformations::EXPAND);
    let mut r = d.read_info().unwrap();
    let mut buf = vec![0; r.output_buffer_size().unwrap()];
    let info = r.next_frame(&mut buf).unwrap();
    buf.truncate(info.buffer_size());
    buf
}

fn webp_pixels(bytes: &[u8]) -> Vec<u8> {
    let mut d = image_webp::WebPDecoder::new(Cursor::new(bytes)).unwrap();
    let mut buf = vec![0; d.output_buffer_size().unwrap()];
    d.read_image(&mut buf).unwrap();
    buf
}

fn png_chunk_list(b: &[u8]) -> Vec<([u8; 4], Vec<u8>)> {
    let mut out = Vec::new();
    let mut pos = 8;
    while pos + 12 <= b.len() {
        let len = u32::from_be_bytes(b[pos..pos + 4].try_into().unwrap()) as usize;
        let ty: [u8; 4] = b[pos + 4..pos + 8].try_into().unwrap();
        out.push((ty, b[pos..pos + 12 + len].to_vec()));
        pos += 12 + len;
    }
    out
}

/// Inside `rect` within a level of `original`, outside it identical.
fn compare(name: &str, out: &[u8], original: &Raster, rect: PixelRect) {
    let c = original.layout().channels();
    for (p, (a, b)) in out
        .chunks_exact(c)
        .zip(original.samples().chunks_exact(c))
        .enumerate()
    {
        let (x, y) = (p as u32 % W, p as u32 / W);
        let inside =
            x >= rect.x && y >= rect.y && x < rect.x + rect.width && y < rect.y + rect.height;
        for k in 0..c {
            let d = u16::from(a[k]).abs_diff(b[k]);
            if inside {
                assert!(d <= 1, "{name}: ({x}, {y}) off by {d}");
            } else {
                assert_eq!(d, 0, "{name}: ({x}, {y}) moved outside the mark");
            }
        }
    }
}

fn options(catalogue: &Catalogue) -> PictureOptions<'_> {
    PictureOptions {
        scope: Scope::AiProvenance,
        catalogue: Some(catalogue),
    }
}

fn marked(kind: Kind, seed: u64) -> (Raster, Raster) {
    let original = picture(kind, W, H, seed, Layout::Rgb8);
    let mut marked = original.clone();
    composite_at(&mut marked, &synthetic_v1().small, small_row(W, H, 48));
    (original, marked)
}

// ------------------------------------------------------------- tests

#[test]
fn a_marked_png_is_restored_and_nothing_else_moves() {
    let catalogue = synthetic_catalogue();
    let iccp = chunk(
        b"iCCP",
        b"profile\0\0\x78\x01\x01\x00\x00\xff\xff\x00\x00\x00\x01",
    );
    let comment = chunk(b"tEXt", b"Comment\0a holiday");
    let manifest = chunk(b"caBX", b"jumb c2pa");
    for (kind, seed) in [
        (Kind::Fractal, 1),
        (Kind::Gradient, 2),
        (Kind::ValueNoise, 3),
    ] {
        let (original, marked) = marked(kind, seed);
        let bytes = with_chunks(
            &png_of(&marked),
            &[iccp.clone(), comment.clone(), manifest.clone()],
        );
        let (out, report) = clean(&bytes, &options(&catalogue)).unwrap();
        let name = format!("{kind:?}");
        compare(&name, &png_pixels(&out), &original, small_row(W, H, 48));
        let chunks = png_chunk_list(&out);
        assert!(
            chunks.iter().any(|(_, c)| *c == iccp),
            "{name}: the colour profile moved"
        );
        assert!(
            chunks.iter().any(|(_, c)| *c == comment),
            "{name}: the comment moved"
        );
        assert!(
            !chunks.iter().any(|(t, _)| t == b"caBX"),
            "{name}: C2PA outlived the pixels it signed"
        );
        assert!(!report.metadata.still_has_c2pa);
        assert_eq!(
            report.encoding,
            Encoding::Png {
                colour_changed: false,
                interlace_dropped: false
            }
        );
        assert!(!report.marks_left(), "{name}");
        match &report.visible {
            Visible::Examined { report, restorable } => {
                assert!(*restorable);
                assert_eq!(report.restored.len(), 1, "{name}");
                assert!(report.restored[0].exact, "{name}");
            }
            Visible::NotExamined(why) => panic!("{name}: {why:?}"),
        }
        // Re-detection on the output finds nothing that verifies.
        let again = inspect(&out, &options(&catalogue)).unwrap();
        if let Visible::Examined { report, .. } = &again.visible {
            assert!(
                report.found.iter().all(|f| f.verified().is_none()),
                "{name}"
            );
        }
    }
}

#[test]
fn a_marked_webp_is_restored_losslessly() {
    let catalogue = synthetic_catalogue();
    let (original, marked) = marked(Kind::Fractal, 11);
    let bytes = webp_of(&marked);
    let (out, report) = clean(&bytes, &options(&catalogue)).unwrap();
    compare("webp", &webp_pixels(&out), &original, small_row(W, H, 48));
    assert_eq!(
        report.encoding,
        Encoding::WebPLossless { from_lossy: false }
    );
    assert!(!report.marks_left());
}

/// A picture with no mark is not re-encoded: the output is the metadata
/// pass's, byte for byte.
#[test]
fn nothing_restored_is_strip_byte_for_byte() {
    let catalogue = synthetic_catalogue();
    let plain = picture(Kind::ValueNoise, W, H, 21, Layout::Rgb8);
    for bytes in [
        with_chunks(&png_of(&plain), &[chunk(b"tEXt", b"Software\0NovelAI")]),
        webp_of(&plain),
    ] {
        for scope in Scope::ALL {
            let (out, report) = clean(
                &bytes,
                &PictureOptions {
                    scope,
                    catalogue: Some(&catalogue),
                },
            )
            .unwrap();
            let (stripped, _) = wipemark_image::strip(&bytes, &StripOptions { scope }).unwrap();
            assert_eq!(out, stripped);
            assert_eq!(report.encoding, Encoding::Unchanged);
            assert!(report.examined());
        }
    }
}

/// A palette stays a palette when every colour is in it, and becomes RGB
/// — said — when one is not.
#[test]
fn a_palette_png_stays_a_palette_when_it_can() {
    let info = PngInfo {
        colour: png::ColorType::Indexed,
        depth: png::BitDepth::Four,
        palette: Some(vec![0, 0, 0, 255, 255, 255, 10, 20, 30]),
        trns: None,
        interlaced: true,
    };
    let source = Source::Png(info);
    let inside =
        Raster::from_u8(3, 1, Layout::Rgb8, &[10, 20, 30, 255, 255, 255, 0, 0, 0]).unwrap();
    let (bytes, encoding) = encode_like(&source, &inside).unwrap();
    assert_eq!(bytes[8 + 8 + 9], 3, "colour type");
    assert_eq!(bytes[8 + 8 + 8], 4, "bit depth");
    assert_eq!(
        encoding,
        Encoding::Png {
            colour_changed: false,
            interlace_dropped: true
        }
    );
    assert_eq!(png_pixels(&bytes), [10, 20, 30, 255, 255, 255, 0, 0, 0]);

    let outside =
        Raster::from_u8(3, 1, Layout::Rgb8, &[10, 20, 31, 255, 255, 255, 0, 0, 0]).unwrap();
    let (bytes, encoding) = encode_like(&source, &outside).unwrap();
    assert_eq!(bytes[8 + 8 + 9], 2, "an RGB picture");
    assert!(matches!(
        encoding,
        Encoding::Png {
            colour_changed: true,
            ..
        }
    ));
    assert_eq!(png_pixels(&bytes), [10, 20, 31, 255, 255, 255, 0, 0, 0]);
}

/// Grey stays grey, sub-byte depth included, when every pixel is grey.
#[test]
fn a_grey_png_stays_grey_when_it_can() {
    let info = PngInfo {
        colour: png::ColorType::Grayscale,
        depth: png::BitDepth::Two,
        palette: None,
        trns: None,
        interlaced: false,
    };
    let source = Source::Png(info);
    let grey = Raster::from_u8(
        4,
        1,
        Layout::Rgb8,
        &[0, 0, 0, 85, 85, 85, 170, 170, 170, 255, 255, 255],
    )
    .unwrap();
    let (bytes, encoding) = encode_like(&source, &grey).unwrap();
    assert_eq!((bytes[8 + 8 + 8], bytes[8 + 8 + 9]), (2, 0));
    assert!(matches!(
        encoding,
        Encoding::Png {
            colour_changed: false,
            ..
        }
    ));
    // 86 has no code at two bits: written as RGB at eight bits, and said.
    let off = Raster::from_u8(1, 1, Layout::Rgb8, &[86, 86, 86]).unwrap();
    let (bytes, encoding) = encode_like(&source, &off).unwrap();
    assert_eq!((bytes[8 + 8 + 8], bytes[8 + 8 + 9]), (8, 2));
    assert!(matches!(
        encoding,
        Encoding::Png {
            colour_changed: true,
            ..
        }
    ));
}

/// An animated PNG's pixels are not examined, and the report says so.
#[test]
fn an_animated_png_is_not_examined_and_says_so() {
    let catalogue = synthetic_catalogue();
    let mut bytes = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut bytes, 4, 4);
        enc.set_color(png::ColorType::Rgb);
        enc.set_depth(png::BitDepth::Eight);
        enc.set_animated(1, 0).unwrap();
        let mut w = enc.write_header().unwrap();
        w.write_image_data(&[7u8; 48]).unwrap();
        w.finish().unwrap();
    }
    let (out, report) = clean(&bytes, &options(&catalogue)).unwrap();
    assert_eq!(report.visible, Visible::NotExamined(NotExamined::Animated));
    // Not examined is not clean: a surface reads it as inconclusive.
    assert!(report.inconclusive());
    let inspection = wipemark_picture::inspect(&bytes, &options(&catalogue)).unwrap();
    assert!(inspection.inconclusive());
    assert_eq!(
        out,
        wipemark_image::strip(&bytes, &StripOptions::default())
            .unwrap()
            .0
    );
    assert!(!report.marks_left());
}

/// The JSON carries both passes and the picture's shelf, in ASCII.
#[test]
fn the_picture_report_json_carries_both_passes_and_the_shelf() {
    let catalogue = synthetic_catalogue();
    let (_, marked) = marked(Kind::Fractal, 41);
    let (_, report) = clean(&png_of(&marked), &options(&catalogue)).unwrap();
    let json = report.to_json();
    assert!(json.is_ascii(), "{json}");
    for key in [
        "{\"container\":\"png\",\"still_has_ai_metadata\":false,",
        "\"orientation_removed\":null,\"visible\":{\"examined\":true,\"restorable\":true,\"found\":[",
        "\"encoding\":{\"kind\":\"png\",\"colour_changed\":false,\"interlace_dropped\":false}",
        "\"marks_left\":false,",
        "\"not_established\":[\"invisible-pixel-marks\",\"vendor-detector-evasion\",\"human-authorship\",\"unknown-mark-schemes\"]}",
    ] {
        assert!(json.contains(key), "{key} not in {json}");
    }
}

/// The proof refuses each output `clean` must never hand back: one that
/// does not decode to the restored picture, one that moved a sample
/// outside the restored rectangle, one on which the mark still verifies.
#[test]
fn the_proof_refuses_what_must_never_be_written() {
    let catalogue = synthetic_catalogue();
    let options = ExamineOptions::default();
    let rgb = Source::Png(PngInfo {
        colour: png::ColorType::Rgb,
        depth: png::BitDepth::Eight,
        palette: None,
        trns: None,
        interlaced: false,
    });
    let png = wipemark_image::ImageContainer::Png;
    let rect = small_row(W, H, 48);
    let (original, marked) = marked(Kind::Gradient, 51);

    // An output of other pixels than the restored ones.
    let (out, _) = encode_like(&rgb, &marked).unwrap();
    assert_eq!(
        prove(&out, png, &marked, &original, &[rect], &catalogue, &options),
        Err(PictureError::Proof(Proof::Samples))
    );

    // A sample moved outside the rectangle.
    let mut moved: Vec<u16> = original.samples().to_vec();
    moved[0] ^= 1;
    let moved = Raster::new(W, H, Layout::Rgb8, moved).unwrap();
    let (out, _) = encode_like(&rgb, &moved).unwrap();
    assert_eq!(
        prove(&out, png, &original, &moved, &[rect], &catalogue, &options),
        Err(PictureError::Proof(Proof::Outside))
    );

    // The mark is still there.
    let (out, _) = encode_like(&rgb, &marked).unwrap();
    assert_eq!(
        prove(&out, png, &marked, &marked, &[rect], &catalogue, &options),
        Err(PictureError::Proof(Proof::StillVerifies))
    );

    // And what clean would write passes.
    let (out, _) = encode_like(&rgb, &original).unwrap();
    assert_eq!(
        prove(&out, png, &marked, &original, &[rect], &catalogue, &options),
        Ok(())
    );
}
