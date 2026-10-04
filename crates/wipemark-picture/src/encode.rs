//! A restored raster back to a file of the original's kind — a minimal
//! one, which `wipemark_image::reframe` then puts the original's metadata
//! and rendering around.
//!
//! * **PNG** at the original's colour type and bit depth whenever the new
//!   values allow it: a palette stays a palette when every restored colour
//!   is in it, grey stays grey when every pixel is grey (and sub-byte grey
//!   keeps its depth when every value has a code there). Otherwise RGB(A)
//!   8 or 16, and the report says the colour type changed. Interlace is
//!   not written, and the report says so.
//! * **WebP** lossless (`VP8L`), whatever the original was: `image-webp`
//!   has no lossy encoder, and the owner's answer to Q-V3 is lossless out.

use std::collections::HashMap;

use wipemark_image::ImageContainer;
use wipemark_pixels::{Layout, Raster};

use crate::decode::{PngInfo, Source};
use crate::PictureError;

/// How the picture was written back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encoding {
    /// Nothing was restored: the output is the metadata pass's alone, and
    /// no pixel was re-encoded.
    Unchanged,
    Png {
        /// The colour type or bit depth is not the original's.
        colour_changed: bool,
        /// The original was interlaced; the output is not.
        interlace_dropped: bool,
    },
    WebPLossless {
        /// The original was lossy (`VP8`); the output is `VP8L` (Q-V3).
        from_lossy: bool,
    },
    Jpeg {
        /// The JPEG quality the picture was re-encoded at.
        quality: u8,
    },
}

impl Encoding {
    /// The id the report writes. A format.
    pub fn id(self) -> &'static str {
        match self {
            Encoding::Unchanged => "unchanged",
            Encoding::Png { .. } => "png",
            Encoding::WebPLossless { .. } => "webp-lossless",
            Encoding::Jpeg { .. } => "jpeg",
        }
    }
}

/// `raster`, written as a file of `source`'s kind.
pub fn encode_like(source: &Source, raster: &Raster) -> Result<(Vec<u8>, Encoding), PictureError> {
    match source {
        Source::Png(info) => png_encode(info, raster),
        Source::WebP { lossy, .. } => {
            webp_encode(raster).map(|b| (b, Encoding::WebPLossless { from_lossy: *lossy }))
        }
        Source::Jpeg => Err(PictureError::Encode {
            container: ImageContainer::Jpeg,
        }),
    }
}

fn depth_bits(depth: png::BitDepth) -> u8 {
    match depth {
        png::BitDepth::One => 1,
        png::BitDepth::Two => 2,
        png::BitDepth::Four => 4,
        png::BitDepth::Eight => 8,
        png::BitDepth::Sixteen => 16,
    }
}

/// Values of `bits` bits, packed most significant first, a row at a time.
fn pack(values: &[u8], width: usize, bits: u8) -> Vec<u8> {
    if bits == 8 {
        return values.to_vec();
    }
    let per = (8 / bits) as usize;
    let mut out = Vec::with_capacity(values.len() / per + values.len() / width.max(1) + 1);
    for row in values.chunks(width.max(1)) {
        for group in row.chunks(per) {
            let mut byte = 0u8;
            for (i, &v) in group.iter().enumerate() {
                byte |= v << (8 - bits as usize * (i + 1));
            }
            out.push(byte);
        }
    }
    out
}

fn write_png(
    width: u32,
    height: u32,
    colour: png::ColorType,
    depth: png::BitDepth,
    palette: Option<&[u8]>,
    trns: Option<&[u8]>,
    data: &[u8],
) -> Result<Vec<u8>, PictureError> {
    let bad = |_| PictureError::Encode {
        container: ImageContainer::Png,
    };
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, width, height);
        enc.set_color(colour);
        enc.set_depth(depth);
        if let Some(p) = palette {
            enc.set_palette(p.to_vec());
        }
        if let Some(t) = trns {
            enc.set_trns(t.to_vec());
        }
        let mut writer = enc.write_header().map_err(bad)?;
        writer.write_image_data(data).map_err(bad)?;
        writer.finish().map_err(bad)?;
    }
    Ok(out)
}

fn png_encode(info: &PngInfo, raster: &Raster) -> Result<(Vec<u8>, Encoding), PictureError> {
    let (w, h) = (raster.width(), raster.height());
    let layout = raster.layout();
    let c = layout.channels();
    let samples = raster.samples();
    let eight = matches!(layout, Layout::Rgb8 | Layout::Rgba8);
    let interlace_dropped = info.interlaced;
    let done = |bytes: Vec<u8>, colour_changed: bool| {
        Ok((
            bytes,
            Encoding::Png {
                colour_changed,
                interlace_dropped,
            },
        ))
    };
    let rgba_at = |p: usize| -> [u16; 4] {
        let s = &samples[p * c..p * c + c];
        [s[0], s[1], s[2], if c == 4 { s[3] } else { layout.max() }]
    };
    let n = (w * h) as usize;

    // A palette, kept when every colour is in it.
    if let (png::ColorType::Indexed, Some(palette), true) = (info.colour, &info.palette, eight) {
        let mut index: HashMap<[u16; 4], u8> = HashMap::new();
        for (i, rgb) in palette.chunks_exact(3).enumerate() {
            let a = info
                .trns
                .as_ref()
                .and_then(|t| t.get(i))
                .copied()
                .unwrap_or(255);
            index
                .entry([
                    u16::from(rgb[0]),
                    u16::from(rgb[1]),
                    u16::from(rgb[2]),
                    u16::from(a),
                ])
                .or_insert(i as u8);
        }
        let indices: Option<Vec<u8>> = (0..n).map(|p| index.get(&rgba_at(p)).copied()).collect();
        if let Some(indices) = indices {
            let bits = depth_bits(info.depth);
            let data = pack(&indices, w as usize, bits);
            let bytes = write_png(
                w,
                h,
                png::ColorType::Indexed,
                info.depth,
                Some(palette),
                info.trns.as_deref(),
                &data,
            )?;
            return done(bytes, false);
        }
    }

    // Grey, kept when every pixel is grey and every alpha is what the
    // original could say.
    let grey = (0..n).all(|p| {
        let [r, g, b, _] = rgba_at(p);
        r == g && g == b
    });
    if grey {
        match info.colour {
            png::ColorType::Grayscale if c == 3 => {
                let bits = depth_bits(info.depth);
                if bits >= 8 {
                    let data: Vec<u8> = if bits == 16 {
                        (0..n).flat_map(|p| rgba_at(p)[0].to_be_bytes()).collect()
                    } else {
                        (0..n).map(|p| rgba_at(p)[0] as u8).collect()
                    };
                    return done(
                        write_png(
                            w,
                            h,
                            png::ColorType::Grayscale,
                            info.depth,
                            None,
                            None,
                            &data,
                        )?,
                        false,
                    );
                }
                // Sub-byte grey: every value must have a code at that depth.
                let top = (1u16 << bits) - 1;
                let codes: Option<Vec<u8>> = (0..n)
                    .map(|p| {
                        let v = rgba_at(p)[0];
                        let code = (u32::from(v) * u32::from(top) + 127) / 255;
                        (code * 255 / u32::from(top) == u32::from(v)).then_some(code as u8)
                    })
                    .collect();
                if let Some(codes) = codes {
                    let data = pack(&codes, w as usize, bits);
                    return done(
                        write_png(
                            w,
                            h,
                            png::ColorType::Grayscale,
                            info.depth,
                            None,
                            None,
                            &data,
                        )?,
                        false,
                    );
                }
            }
            png::ColorType::GrayscaleAlpha if c == 4 => {
                let data: Vec<u8> = if eight {
                    (0..n)
                        .flat_map(|p| {
                            let [g, _, _, a] = rgba_at(p);
                            [g as u8, a as u8]
                        })
                        .collect()
                } else {
                    (0..n)
                        .flat_map(|p| {
                            let [g, _, _, a] = rgba_at(p);
                            let (g, a) = (g.to_be_bytes(), a.to_be_bytes());
                            [g[0], g[1], a[0], a[1]]
                        })
                        .collect()
                };
                return done(
                    write_png(
                        w,
                        h,
                        png::ColorType::GrayscaleAlpha,
                        info.depth,
                        None,
                        None,
                        &data,
                    )?,
                    false,
                );
            }
            _ => {}
        }
    }

    // RGB(A) at the raster's depth.
    let colour = if c == 4 {
        png::ColorType::Rgba
    } else {
        png::ColorType::Rgb
    };
    let (depth, data): (png::BitDepth, Vec<u8>) = if eight {
        (
            png::BitDepth::Eight,
            samples.iter().map(|&s| s as u8).collect(),
        )
    } else {
        (
            png::BitDepth::Sixteen,
            samples.iter().flat_map(|s| s.to_be_bytes()).collect(),
        )
    };
    let same = info.colour == colour && info.depth == depth;
    done(write_png(w, h, colour, depth, None, None, &data)?, !same)
}

fn webp_encode(raster: &Raster) -> Result<Vec<u8>, PictureError> {
    let bad = |_| PictureError::Encode {
        container: ImageContainer::WebP,
    };
    let colour = match raster.layout() {
        Layout::Rgb8 => image_webp::ColorType::Rgb8,
        Layout::Rgba8 => image_webp::ColorType::Rgba8,
        Layout::Rgb16 | Layout::Rgba16 => {
            return Err(PictureError::Encode {
                container: ImageContainer::WebP,
            })
        }
    };
    let bytes: Vec<u8> = raster.samples().iter().map(|&s| s as u8).collect();
    let mut out = Vec::new();
    image_webp::WebPEncoder::new(&mut out)
        .encode(&bytes, raster.width(), raster.height(), colour)
        .map_err(bad)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::pack;

    #[test]
    fn sub_byte_values_are_packed_most_significant_first_by_row() {
        assert_eq!(
            pack(&[1, 0, 1, 1, 0, 0, 0, 1, 1], 9, 1),
            [0b1011_0001, 0b1000_0000]
        );
        assert_eq!(pack(&[3, 2, 1, 0, 3], 5, 2), [0b1110_0100, 0b1100_0000]);
        // A new row starts a new byte.
        assert_eq!(pack(&[1, 1, 1, 1], 2, 4), [0x11, 0x11]);
    }
}
