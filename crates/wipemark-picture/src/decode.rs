//! A picture file to the **stored** raster (D157): no colour management,
//! no EXIF rotation, alpha kept, 16 bits kept. A palette or grey picture
//! is expanded to RGB(A) for the maths; what it was is remembered, so the
//! encoder can go back to it when the new values allow.

use std::io::Cursor;

use wipemark_image::ImageContainer;
use wipemark_pixels::{Fidelity, Layout, Plane, Planes, Quant, Raster, Sampling};
use zune_jpeg::zune_core::colorspace::ColorSpace;
use zune_jpeg::zune_core::options::DecoderOptions;

use crate::PictureError;

/// What a PNG was, beyond its samples.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PngInfo {
    pub colour: png::ColorType,
    pub depth: png::BitDepth,
    pub palette: Option<Vec<u8>>,
    pub trns: Option<Vec<u8>>,
    pub interlaced: bool,
}

/// Which file the raster came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    Png(PngInfo),
    WebP {
        lossy: bool,
        alpha: bool,
    },
    /// A JPEG of 1 (grey), 3 (YCbCr) or 4 (CMYK/YCCK) components.
    Jpeg {
        components: u8,
    },
}

/// A decoded picture.
#[derive(Debug, Clone, PartialEq)]
pub struct Decoded {
    pub raster: Raster,
    pub fidelity: Fidelity,
    pub source: Source,
    /// A YCbCr JPEG's stored planes (D491): Y, Cb and Cr at their own
    /// resolution and the quantisation tables, from a second decoder over
    /// the same bytes. Filled only by [`decode_with_planes`]: [`decode`]
    /// leaves it `None`, because the second decode costs ×1.43 of the
    /// first on a 2048 JPEG (E12-R3's raw-output report; ×1.40 on R3's
    /// own patch) and only a verified mark on a JPEG needs them. `None`
    /// too for every other picture — grey, CMYK, an RGB-coded JPEG, PNG,
    /// WebP — and for a JPEG whose planes could not be read, which is not
    /// a refusal: the raster above is the decoder's own either way, and
    /// nothing reads the planes yet.
    pub planes: Option<Planes>,
}

/// A picture whose pixels are not examined, and why.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Skip {
    /// An animated PNG or WebP: one frame is not the picture.
    Animated,
}

fn refused(container: ImageContainer) -> PictureError {
    PictureError::Decode { container }
}

/// The stored raster of `bytes`, a picture `wipemark_image` already
/// placed as `container`.
pub fn decode(
    bytes: &[u8],
    container: ImageContainer,
) -> Result<Result<Decoded, Skip>, PictureError> {
    match container {
        ImageContainer::Png => png_decode(bytes),
        ImageContainer::WebP => webp_decode(bytes),
        ImageContainer::Jpeg => jpeg_decode(bytes).map(Ok),
        other => Err(refused(other)),
    }
}

/// [`decode`], and for a three-component JPEG its stored planes too
/// ([`Decoded::planes`]): the raster is `decode`'s, the planes are read by
/// a second decoder over the same bytes. For the caller that needs them —
/// the planar inverse, once a mark on a JPEG is verified — and nobody else.
pub fn decode_with_planes(
    bytes: &[u8],
    container: ImageContainer,
) -> Result<Result<Decoded, Skip>, PictureError> {
    let mut decoded = decode(bytes, container)?;
    if let Ok(d) = &mut decoded {
        if d.source == (Source::Jpeg { components: 3 }) {
            d.planes = jpeg_planes(bytes);
        }
    }
    Ok(decoded)
}

fn png_decode(bytes: &[u8]) -> Result<Result<Decoded, Skip>, PictureError> {
    let bad = || refused(ImageContainer::Png);
    // What it is, untransformed.
    let raw = png::Decoder::new(Cursor::new(bytes))
        .read_info()
        .map_err(|_| bad())?;
    let info = raw.info();
    if info.animation_control.is_some() {
        return Ok(Err(Skip::Animated));
    }
    let png_info = PngInfo {
        colour: info.color_type,
        depth: info.bit_depth,
        palette: info.palette.as_ref().map(|p| p.to_vec()),
        trns: info.trns.as_ref().map(|t| t.to_vec()),
        interlaced: info.interlaced,
    };
    // Its samples, expanded: a palette and tRNS to RGB(A), grey below 8
    // bits to 8.
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::EXPAND);
    let mut reader = decoder.read_info().map_err(|_| bad())?;
    let mut buf = vec![0; reader.output_buffer_size().ok_or_else(bad)?];
    let frame = reader.next_frame(&mut buf).map_err(|_| bad())?;
    let buf = &buf[..frame.buffer_size()];
    let sixteen = frame.bit_depth == png::BitDepth::Sixteen;
    let values: Vec<u16> = if sixteen {
        buf.chunks_exact(2)
            .map(|b| u16::from_be_bytes([b[0], b[1]]))
            .collect()
    } else {
        buf.iter().map(|&b| u16::from(b)).collect()
    };
    let channels = frame.color_type.samples();
    let alpha = matches!(channels, 2 | 4);
    let mut samples = Vec::with_capacity(values.len() / channels * if alpha { 4 } else { 3 });
    for p in values.chunks_exact(channels) {
        match channels {
            1 => samples.extend_from_slice(&[p[0], p[0], p[0]]),
            2 => samples.extend_from_slice(&[p[0], p[0], p[0], p[1]]),
            3 => samples.extend_from_slice(p),
            _ => samples.extend_from_slice(&p[..4]),
        }
    }
    let layout = match (sixteen, alpha) {
        (false, false) => Layout::Rgb8,
        (false, true) => Layout::Rgba8,
        (true, false) => Layout::Rgb16,
        (true, true) => Layout::Rgba16,
    };
    let raster = Raster::new(frame.width, frame.height, layout, samples).map_err(|_| bad())?;
    Ok(Ok(Decoded {
        raster,
        fidelity: Fidelity::Lossless,
        source: Source::Png(png_info),
        planes: None,
    }))
}

fn webp_decode(bytes: &[u8]) -> Result<Result<Decoded, Skip>, PictureError> {
    let bad = || refused(ImageContainer::WebP);
    let mut decoder = image_webp::WebPDecoder::new(Cursor::new(bytes)).map_err(|_| bad())?;
    if decoder.is_animated() {
        return Ok(Err(Skip::Animated));
    }
    let lossy = decoder.is_lossy();
    let alpha = decoder.has_alpha();
    let (w, h) = decoder.dimensions();
    let mut buf = vec![0; decoder.output_buffer_size().ok_or_else(bad)?];
    decoder.read_image(&mut buf).map_err(|_| bad())?;
    let layout = if alpha { Layout::Rgba8 } else { Layout::Rgb8 };
    let raster = Raster::from_u8(w, h, layout, &buf).map_err(|_| bad())?;
    Ok(Ok(Decoded {
        raster,
        fidelity: if lossy {
            Fidelity::Lossy
        } else {
            Fidelity::Lossless
        },
        source: Source::WebP { lossy, alpha },
        planes: None,
    }))
}

/// A scan the walk finds damaged ([`crate::scan::walk`]) is not decoded
/// at all: the decoder would recover from it without saying so, and a
/// restoration over what it filled in, re-encoded, would hand back a
/// picture the file never held. Refused here, the metadata is still
/// cleaned and the pixels are said not to have been examined.
fn jpeg_decode(bytes: &[u8]) -> Result<Decoded, PictureError> {
    let bad = || refused(ImageContainer::Jpeg);
    if crate::scan::walk(bytes) == crate::scan::Scan::Damaged {
        return Err(bad());
    }
    let options = DecoderOptions::default().jpeg_set_out_colorspace(ColorSpace::RGB);
    let mut decoder = zune_jpeg::JpegDecoder::new_with_options(Cursor::new(bytes), options);
    let pixels = decoder.decode().map_err(|_| bad())?;
    let info = decoder.info().ok_or_else(bad)?;
    let raster = Raster::from_u8(
        u32::from(info.width),
        u32::from(info.height),
        Layout::Rgb8,
        &pixels,
    )
    .map_err(|_| bad())?;
    Ok(Decoded {
        raster,
        fidelity: Fidelity::Lossy,
        source: Source::Jpeg {
            components: info.components,
        },
        planes: None,
    })
}

/// A three-component YCbCr JPEG's stored planes, read by a second decoder
/// over the same bytes — a second entropy pass — through upstream's raw
/// output (`JpegDecoder::raw_output`, D490): one plane per component, the
/// IDCT's clamped output before upsampling and colour conversion, handed
/// out padded to whole 8 × 8 blocks and cropped here to its logical
/// `ceil(W·h/h_max) × ceil(H·v/v_max)`; and each component's quantisation
/// table, in natural order, through the fork's one getter
/// (`RawDecodeSession::quantization_tables`). `None` when the components
/// are not YCbCr (an Adobe RGB JPEG), when the decoder refuses (a height
/// given by a DNL marker among the rest), when Cb and Cr are not
/// quantised by one table (`Quant` holds one chroma table), or when the
/// sizes do not make [`Planes`].
fn jpeg_planes(bytes: &[u8]) -> Option<Planes> {
    let mut decoder = zune_jpeg::JpegDecoder::new(Cursor::new(bytes));
    decoder.decode_headers().ok()?;
    if decoder.input_colorspace()? != ColorSpace::YCbCr {
        return None;
    }
    let (width, height) = decoder.dimensions()?;
    let mut raw = decoder.raw_output();
    if raw.num_components()? != 3 {
        return None;
    }
    let layout = raw.layout()?;
    // The getter hands out each component's table by value, not the slot
    // it was read from: Cb and Cr share one when its values are one.
    let [luma, cb_table, cr_table] = <[[u16; 64]; 3]>::try_from(raw.quantization_tables()?).ok()?;
    if cb_table != cr_table {
        return None;
    }
    let mut padded: Vec<Vec<u8>> = layout[..3].iter().map(|p| vec![0; p.byte_size]).collect();
    let mut buffers: Vec<&mut [u8]> = padded.iter_mut().map(Vec::as_mut_slice).collect();
    raw.decode_into_planes(&mut buffers).ok()?;
    let plane = |info: &zune_jpeg::PlaneInfo, padded: &[u8]| {
        // `allocated_height` rows of `stride` bytes; what lies past
        // `width` and below `height` is the blocks' padding. Row by row
        // into a buffer allocated once: collected through a `flat_map`,
        // which has no length to allocate by, the crop cost more than the
        // decode (E12-R3's raw-output report).
        let mut samples = Vec::with_capacity(info.width * info.height);
        for row in padded.chunks_exact(info.stride).take(info.height) {
            samples.extend(row[..info.width].iter().map(|&s| u16::from(s)));
        }
        Plane::new(
            u32::try_from(info.width).ok()?,
            u32::try_from(info.height).ok()?,
            samples,
        )
        .ok()
    };
    let factor = |f: usize| u8::try_from(f).ok();
    let sampling = Sampling::of(
        [
            factor(layout[0].horizontal_sampling_factor)?,
            factor(layout[1].horizontal_sampling_factor)?,
            factor(layout[2].horizontal_sampling_factor)?,
        ],
        [
            factor(layout[0].vertical_sampling_factor)?,
            factor(layout[1].vertical_sampling_factor)?,
            factor(layout[2].vertical_sampling_factor)?,
        ],
    );
    Planes::new(
        u32::try_from(width).ok()?,
        u32::try_from(height).ok()?,
        sampling,
        plane(&layout[0], &padded[0])?,
        Some(plane(&layout[1], &padded[1])?),
        Some(plane(&layout[2], &padded[2])?),
        Quant {
            luma,
            chroma: Some(cb_table),
        },
    )
    .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> Vec<u8> {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/image")
            .join(name);
        std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
    }

    fn planes(name: &str, container: ImageContainer) -> Option<Planes> {
        let bytes = fixture(name);
        let without = decode(&bytes, container)
            .unwrap_or_else(|e| panic!("{name}: {e}"))
            .unwrap_or_else(|skip| panic!("{name}: {skip:?}"));
        // The product's road takes none: nothing reads them yet, and the
        // second decode is not free.
        assert_eq!(without.planes, None, "{name}: decode took the planes");
        let with = decode_with_planes(&bytes, container)
            .unwrap_or_else(|e| panic!("{name}: {e}"))
            .unwrap_or_else(|skip| panic!("{name}: {skip:?}"));
        assert_eq!(with.raster, without.raster, "{name}");
        with.planes
    }

    #[test]
    fn a_png_has_no_planes_and_a_lossy_jpeg_has_them() {
        // A YCbCr JPEG at each sampling has them, baseline or progressive.
        for (name, sampling) in [
            ("jpeg-planes/rgb-37x23-q90-444.jpg", Sampling::H444),
            ("jpeg-planes/rgb-37x23-q90-422.jpg", Sampling::H422),
            ("jpeg-planes/rgb-37x23-q90-420.jpg", Sampling::H420),
            (
                "jpeg-planes/rgb-129x65-q90-420-progressive.jpg",
                Sampling::H420,
            ),
        ] {
            let planes = planes(name, ImageContainer::Jpeg)
                .unwrap_or_else(|| panic!("{name}: a YCbCr JPEG has its planes"));
            assert_eq!(planes.sampling(), sampling, "{name}");
        }
        // Grey, PNG and WebP — lossless or lossy — have none.
        for (name, container) in [
            ("jpeg-planes/grey-37x23-q90.jpg", ImageContainer::Jpeg),
            ("gemini/torch-1025.png", ImageContainer::Png),
            ("gemini/scroll-1040-q90.webp", ImageContainer::WebP),
            ("gemini/cut-out-confetti-256.webp", ImageContainer::WebP),
        ] {
            assert_eq!(planes(name, container), None, "{name}");
        }
    }
}
