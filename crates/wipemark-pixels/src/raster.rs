//! The decoded picture this crate works on: the **stored** samples, as a
//! file holds them — no colour management, no EXIF rotation, alpha kept
//! as it is (D157). Decoding a file into one is `wipemark-picture`'s.

/// How the samples of a [`Raster`] are laid out: interleaved, row-major,
/// colour first and alpha last.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layout {
    Rgb8,
    Rgba8,
    Rgb16,
    Rgba16,
}

impl Layout {
    /// Samples per pixel.
    pub fn channels(self) -> usize {
        match self {
            Layout::Rgb8 | Layout::Rgb16 => 3,
            Layout::Rgba8 | Layout::Rgba16 => 4,
        }
    }

    /// Whether the last sample of a pixel is alpha.
    pub fn has_alpha(self) -> bool {
        matches!(self, Layout::Rgba8 | Layout::Rgba16)
    }

    /// The largest sample: 255 or 65535.
    pub fn max(self) -> u16 {
        match self {
            Layout::Rgb8 | Layout::Rgba8 => u16::from(u8::MAX),
            Layout::Rgb16 | Layout::Rgba16 => u16::MAX,
        }
    }

    /// The id the report writes. A format.
    pub fn id(self) -> &'static str {
        match self {
            Layout::Rgb8 => "rgb8",
            Layout::Rgba8 => "rgba8",
            Layout::Rgb16 => "rgb16",
            Layout::Rgba16 => "rgba16",
        }
    }
}

/// Why a raster could not be made.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum RasterError {
    #[error("a raster needs a width and a height above zero")]
    ZeroDimension,
    #[error("{got} samples for a raster that needs {needed}")]
    Length { needed: usize, got: usize },
    #[error("a sample of {value} in an 8-bit layout")]
    Range { value: u16 },
}

/// A decoded picture: `width × height` pixels of `layout`, every sample a
/// `u16` so the 16-bit layouts need no second type. An 8-bit layout holds
/// 0–255.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Raster {
    width: u32,
    height: u32,
    layout: Layout,
    samples: Vec<u16>,
}

impl Raster {
    /// A raster over `samples`, which must hold exactly `width × height ×
    /// channels` values, each within the layout's range.
    pub fn new(
        width: u32,
        height: u32,
        layout: Layout,
        samples: Vec<u16>,
    ) -> Result<Self, RasterError> {
        if width == 0 || height == 0 {
            return Err(RasterError::ZeroDimension);
        }
        let needed = (width as usize)
            .checked_mul(height as usize)
            .and_then(|n| n.checked_mul(layout.channels()))
            .ok_or(RasterError::Length {
                needed: usize::MAX,
                got: samples.len(),
            })?;
        if samples.len() != needed {
            return Err(RasterError::Length {
                needed,
                got: samples.len(),
            });
        }
        if let Some(&value) = samples.iter().find(|&&s| s > layout.max()) {
            return Err(RasterError::Range { value });
        }
        Ok(Raster {
            width,
            height,
            layout,
            samples,
        })
    }

    /// An 8-bit raster from bytes.
    pub fn from_u8(
        width: u32,
        height: u32,
        layout: Layout,
        bytes: &[u8],
    ) -> Result<Self, RasterError> {
        Raster::new(
            width,
            height,
            layout,
            bytes.iter().map(|&b| u16::from(b)).collect(),
        )
    }

    /// An 8-bit RGB raster from samples a caller in this crate built in
    /// range and to size (`Planes::to_rgb`): what `new` would check.
    pub(crate) fn trusted_rgb8(width: u32, height: u32, samples: Vec<u16>) -> Self {
        debug_assert_eq!(samples.len(), width as usize * height as usize * 3);
        debug_assert!(samples.iter().all(|&s| s <= 255));
        Raster {
            width,
            height,
            layout: Layout::Rgb8,
            samples,
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn layout(&self) -> Layout {
        self.layout
    }

    pub fn samples(&self) -> &[u16] {
        &self.samples
    }

    /// The samples, for a writer that keeps the layout's range — the
    /// restoration and the test compositor.
    pub(crate) fn samples_mut(&mut self) -> &mut [u16] {
        &mut self.samples
    }

    pub fn into_samples(self) -> Vec<u16> {
        self.samples
    }

    /// The index of pixel `(x, y)`'s first sample.
    pub(crate) fn at(&self, x: u32, y: u32) -> usize {
        (y as usize * self.width as usize + x as usize) * self.layout.channels()
    }

    /// Rec. 601 luma of pixel `(x, y)` over the stored values, in [0, 1]
    /// — for proposing and verifying, never for restoring.
    pub fn luma(&self, x: u32, y: u32) -> f32 {
        let i = self.at(x, y);
        luma_of(&self.samples[i..i + 3], f32::from(self.layout.max()))
    }

    /// The luma of every pixel, row-major.
    pub(crate) fn luma_plane(&self) -> Vec<f32> {
        let max = f32::from(self.layout.max());
        let c = self.layout.channels();
        self.samples
            .chunks_exact(c)
            .map(|p| luma_of(&p[..3], max))
            .collect()
    }
}

/// Rec. 601 weights, as the reference detector uses them.
pub(crate) const LUMA: [f32; 3] = [0.299, 0.587, 0.114];

fn luma_of(rgb: &[u16], max: f32) -> f32 {
    (LUMA[0] * f32::from(rgb[0]) + LUMA[1] * f32::from(rgb[1]) + LUMA[2] * f32::from(rgb[2])) / max
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_raster_refuses_what_it_cannot_hold() {
        assert_eq!(
            Raster::new(0, 4, Layout::Rgb8, vec![]),
            Err(RasterError::ZeroDimension)
        );
        assert_eq!(
            Raster::new(2, 2, Layout::Rgb8, vec![0; 11]),
            Err(RasterError::Length {
                needed: 12,
                got: 11
            })
        );
        assert_eq!(
            Raster::new(1, 1, Layout::Rgb8, vec![0, 256, 0]),
            Err(RasterError::Range { value: 256 })
        );
        assert!(Raster::new(1, 1, Layout::Rgb16, vec![0, 65535, 0]).is_ok());
    }

    #[test]
    fn luma_is_rec_601_over_the_stored_values() {
        let r = Raster::from_u8(2, 1, Layout::Rgba8, &[255, 255, 255, 7, 255, 0, 0, 255]).unwrap();
        assert!((r.luma(0, 0) - 1.0).abs() < 1e-6);
        assert!((r.luma(1, 0) - 0.299).abs() < 1e-6);
        assert_eq!(r.luma_plane().len(), 2);
    }
}
