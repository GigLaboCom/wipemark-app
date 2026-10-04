//! JPEG and lossy WebP (E12-4): decoded, restored, re-encoded — JPEG at
//! quality 95, lossy WebP as lossless — with the metadata the scope keeps
//! carried over by `reframe`, and the report saying the picture was
//! re-encoded.

#[path = "../../wipemark-pixels/tests/support/mod.rs"]
mod pixels_support;

use pixels_support::{composite_at, picture, small_row, synthetic_catalogue, synthetic_v1, Kind};
use wipemark_image::{reframe, StripOptions};
use wipemark_picture::{
    clean, decode, encode_like, inspect, prove, psnr, Encoding, PictureError, PictureOptions,
    Proof, Source, Visible, JPEG_QUALITY, PSNR_FLOOR,
};
use wipemark_pixels::{Catalogue, ExamineOptions, Layout, PixelRect, Raster};

const W: u32 = 320;
const H: u32 = 240;

fn jpeg_of(raster: &Raster, grey: bool) -> Vec<u8> {
    let (bytes, colour): (Vec<u8>, image::ExtendedColorType) = if grey {
        (
            raster
                .samples()
                .chunks_exact(3)
                .map(|p| p[0] as u8)
                .collect(),
            image::ExtendedColorType::L8,
        )
    } else {
        (
            raster.samples().iter().map(|&s| s as u8).collect(),
            image::ExtendedColorType::Rgb8,
        )
    };
    let mut out = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 95)
        .encode(&bytes, raster.width(), raster.height(), colour)
        .unwrap();
    out
}

fn segment(marker: u8, payload: &[u8]) -> Vec<u8> {
    let mut s = vec![0xFF, marker];
    s.extend_from_slice(&((payload.len() + 2) as u16).to_be_bytes());
    s.extend_from_slice(payload);
    s
}

/// `segments` right after SOI.
fn jpeg_with(jpeg: &[u8], segments: &[Vec<u8>]) -> Vec<u8> {
    let mut out = jpeg[..2].to_vec();
    for s in segments {
        out.extend_from_slice(s);
    }
    out.extend_from_slice(&jpeg[2..]);
    out
}

/// Every marker segment before the first scan, whole.
fn segments_of(b: &[u8]) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    let mut pos = 2;
    while pos + 4 <= b.len() && b[pos] == 0xFF {
        let m = b[pos + 1];
        let len = u16::from_be_bytes([b[pos + 2], b[pos + 3]]) as usize;
        out.push(b[pos..pos + 2 + len].to_vec());
        if m == 0xDA {
            break;
        }
        pos += 2 + len;
    }
    out
}

/// The number of components the frame header declares.
fn components(b: &[u8]) -> u8 {
    segments_of(b)
        .into_iter()
        .find(|s| (0xC0..=0xC2).contains(&s[1]))
        .map(|s| s[9])
        .expect("a frame header")
}

fn decoded_rgb(bytes: &[u8]) -> Raster {
    let img = image::load_from_memory_with_format(bytes, image::ImageFormat::Jpeg)
        .unwrap()
        .to_rgb8();
    Raster::from_u8(img.width(), img.height(), Layout::Rgb8, img.as_raw()).unwrap()
}

/// PSNR over the pixels outside `rect`.
fn psnr_outside(a: &Raster, b: &Raster, rect: PixelRect) -> f64 {
    let (mut sum, mut n) = (0f64, 0f64);
    for (p, (x, y)) in a
        .samples()
        .chunks_exact(3)
        .zip(b.samples().chunks_exact(3))
        .enumerate()
    {
        let (px, py) = (p as u32 % W, p as u32 / W);
        if px >= rect.x && py >= rect.y && px < rect.x + rect.width && py < rect.y + rect.height {
            continue;
        }
        for k in 0..3 {
            let d = f64::from(x[k]) - f64::from(y[k]);
            sum += d * d;
            n += 1.0;
        }
    }
    if sum == 0.0 {
        f64::INFINITY
    } else {
        10.0 * (255.0f64 * 255.0 / (sum / n)).log10()
    }
}

fn options(catalogue: &Catalogue) -> PictureOptions<'_> {
    PictureOptions {
        scope: wipemark_image::Scope::AiProvenance,
        catalogue: Some(catalogue),
    }
}

const XMP_AI: &str = "<x:xmpmeta xmlns:x=\"adobe:ns:meta/\"><rdf:RDF \
    xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\"><rdf:Description \
    xmlns:Iptc4xmpExt=\"http://iptc.org/std/Iptc4xmpExt/2008-02-29/\" \
    Iptc4xmpExt:DigitalSourceType=\"http://cv.iptc.org/newscodes/digitalsourcetype/trainedAlgorithmicMedia\"/>\
    </rdf:RDF></x:xmpmeta>";

#[test]
fn a_marked_jpeg_is_restored_and_re_encoded() {
    let catalogue = synthetic_catalogue();
    let rect = small_row(W, H, 48);
    let exif = segment(0xE1, b"Exif\0\0MM\0*\0\0\0\x08\0\0\0\0\0\0");
    let icc = segment(0xE2, b"ICC_PROFILE\0\x01\x01not a real profile");
    let mut xmp_payload = b"http://ns.adobe.com/xap/1.0/\0".to_vec();
    xmp_payload.extend_from_slice(XMP_AI.as_bytes());
    let xmp = segment(0xE1, &xmp_payload);
    for (kind, seed) in [(Kind::Gradient, 1u64), (Kind::Flat, 2)] {
        let original = picture(kind, W, H, seed, Layout::Rgb8);
        let mut marked = original.clone();
        composite_at(&mut marked, &synthetic_v1().small, rect);
        let bytes = jpeg_with(
            &jpeg_of(&marked, false),
            &[exif.clone(), icc.clone(), xmp.clone()],
        );
        let (out, report) = clean(&bytes, &options(&catalogue)).unwrap();
        let name = format!("{kind:?}");
        assert_eq!(
            report.encoding,
            Encoding::Jpeg {
                quality: JPEG_QUALITY
            },
            "{name}: {:?}",
            report.visible
        );
        assert!(!report.marks_left(), "{name}");
        match &report.visible {
            Visible::Examined { report, restorable } => {
                assert!(*restorable);
                assert_eq!(report.restored.len(), 1, "{name}");
                assert!(
                    !report.restored[0].exact,
                    "{name}: a lossy source is never exact"
                );
            }
            Visible::NotExamined(why) => panic!("{name}: {why:?}"),
        }
        // Outside the mark, within the encoder's tolerance of the picture.
        let p = psnr_outside(&decoded_rgb(&out), &original, rect);
        println!("{name}: PSNR outside the mark, two generations at quality 95: {p:.2} dB");
        assert!(p >= PSNR_FLOOR, "{name}: {p:.2} dB");
        // Inside, the mark is gone by the verifier's own test.
        let again = inspect(&out, &options(&catalogue)).unwrap();
        if let Visible::Examined { report, .. } = &again.visible {
            assert!(
                report.found.iter().all(|f| f.verified().is_none()),
                "{name}"
            );
        }
        // The camera data and the colour profile are the original's to
        // the byte; the AI XMP is gone.
        let segs = segments_of(&out);
        assert!(segs.contains(&exif) && segs.contains(&icc), "{name}");
        assert!(!segs.contains(&xmp), "{name}");
        assert!(!report.metadata.still_has_ai_metadata);
    }
}

#[test]
fn a_grey_jpeg_stays_grey() {
    let catalogue = synthetic_catalogue();
    let grey: Vec<u16> = (0..W * H)
        .flat_map(|i| {
            let v = (40 + (i % W) * 120 / W + (i / W) * 60 / H) as u16;
            [v, v, v]
        })
        .collect();
    let mut marked = Raster::new(W, H, Layout::Rgb8, grey).unwrap();
    composite_at(&mut marked, &synthetic_v1().small, small_row(W, H, 48));
    let bytes = jpeg_of(&marked, true);
    assert_eq!(components(&bytes), 1);
    let (out, report) = clean(&bytes, &options(&catalogue)).unwrap();
    assert!(
        matches!(report.encoding, Encoding::Jpeg { .. }),
        "{:?}",
        report.visible
    );
    assert_eq!(components(&out), 1, "a grey picture came out in colour");
    // And through encode_like directly.
    let (bytes, _) = encode_like(&Source::Jpeg { components: 1 }, &marked).unwrap();
    assert_eq!(components(&bytes), 1);
}

/// The real lossy WebP of the fixtures, restored pixels or not: written
/// back as lossless VP8L, its colour profile and EXIF the original's to
/// the byte, its decode the new image's exactly.
#[test]
fn a_lossy_webp_is_written_lossless() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/image/exif-xmp.webp"
    );
    let bytes = std::fs::read(path).unwrap();
    let decoded = decode(&bytes, wipemark_image::ImageContainer::WebP)
        .unwrap()
        .unwrap();
    assert!(matches!(decoded.source, Source::WebP { lossy: true, .. }));
    let (new, encoding) = encode_like(&decoded.source, &decoded.raster).unwrap();
    assert_eq!(encoding, Encoding::WebPLossless { from_lossy: true });
    let (out, _) = reframe(&bytes, &new, &StripOptions::default()).unwrap();
    let chunks = riff_chunks(&out);
    let types: Vec<[u8; 4]> = chunks.iter().map(|(t, _)| *t).collect();
    assert!(types.contains(b"VP8L") && !types.contains(b"VP8 ") && !types.contains(b"ALPH"));
    for kept in [b"ICCP", b"EXIF"] {
        let before = riff_chunks(&bytes)
            .into_iter()
            .find(|(t, _)| t == kept)
            .unwrap()
            .1;
        assert!(chunks.iter().any(|(_, c)| *c == before), "{kept:?} moved");
    }
    let again = decode(&out, wipemark_image::ImageContainer::WebP)
        .unwrap()
        .unwrap();
    assert_eq!(again.raster, decoded.raster);
    let size = u32::from_le_bytes(out[4..8].try_into().unwrap()) as usize;
    assert_eq!(size + 8, out.len());
}

fn riff_chunks(b: &[u8]) -> Vec<([u8; 4], Vec<u8>)> {
    let mut out = Vec::new();
    let mut pos = 12;
    while pos + 8 <= b.len() {
        let ty: [u8; 4] = b[pos..pos + 4].try_into().unwrap();
        let len = u32::from_le_bytes(b[pos + 4..pos + 8].try_into().unwrap()) as usize;
        let end = (pos + 8 + len + (len & 1)).min(b.len());
        out.push((ty, b[pos..end].to_vec()));
        pos = end;
    }
    out
}

/// A lossy output far from the restored picture is refused by the proof.
#[test]
fn the_lossy_proof_refuses_a_distant_output() {
    let catalogue = synthetic_catalogue();
    let restored = picture(Kind::Gradient, W, H, 61, Layout::Rgb8);
    let other = picture(Kind::Fractal, W, H, 62, Layout::Rgb8);
    let jpeg = wipemark_image::ImageContainer::Jpeg;
    let out = jpeg_of(&other, false);
    assert_eq!(
        prove(
            &out,
            jpeg,
            &restored,
            &restored,
            &[],
            &catalogue,
            &ExamineOptions::default()
        ),
        Err(PictureError::Proof(Proof::Samples))
    );
    let out = jpeg_of(&restored, false);
    assert!(psnr(&decoded_rgb(&out), &restored).unwrap() >= PSNR_FLOOR);
    assert_eq!(
        prove(
            &out,
            jpeg,
            &restored,
            &restored,
            &[],
            &catalogue,
            &ExamineOptions::default()
        ),
        Ok(())
    );
}
