//! A picture file to the **stored** raster (D157): no colour management,
//! no EXIF rotation, alpha kept, 16 bits kept. A palette or grey picture
//! is expanded to RGB(A) for the maths; what it was is remembered, so the
//! encoder can go back to it when the new values allow.

use std::io::Cursor;

use wipemark_image::ImageContainer;
use wipemark_pixels::{Fidelity, Layout, Raster};
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
    }))
}

fn jpeg_decode(bytes: &[u8]) -> Result<Decoded, PictureError> {
    let bad = || refused(ImageContainer::Jpeg);
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
    })
}
