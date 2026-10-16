//! Opacity maps, and the `.wma` file that carries one ("Wipemark
//! alpha"): the magic `WMA1`, then `u16 width`, `u16 height` and `u8
//! depth` (8 or 16), little-endian, then `width × height` samples,
//! row-major, each `depth` bits little-endian. `α = sample / (2^depth −
//! 1)`. No codec is needed to read one, which is the point of it.

/// The four bytes every `.wma` starts with.
pub const MAGIC: &[u8; 4] = b"WMA1";
const HEADER: usize = 4 + 2 + 2 + 1;

/// A mark's opacity, one value per pixel in [0, 1], row-major.
#[derive(Debug, Clone, PartialEq)]
pub struct AlphaMap {
    width: u32,
    height: u32,
    values: Vec<f32>,
}

/// Why an opacity map could not be read or made.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum WmaError {
    #[error("not an opacity map: the file does not start with WMA1")]
    Magic,
    #[error("an opacity map of depth {0}; only 8 and 16 are read")]
    Depth(u8),
    #[error("an opacity map with a zero dimension")]
    ZeroDimension,
    #[error("an opacity map whose samples are {got} bytes where {needed} were declared")]
    Size { needed: usize, got: usize },
    #[error("an opacity value outside 0 to 1")]
    Range,
}

impl AlphaMap {
    /// A map over `values`, which must be `width × height` long, each in
    /// [0, 1].
    pub fn new(width: u32, height: u32, values: Vec<f32>) -> Result<Self, WmaError> {
        if width == 0 || height == 0 {
            return Err(WmaError::ZeroDimension);
        }
        let needed = width as usize * height as usize;
        if values.len() != needed {
            return Err(WmaError::Size {
                needed,
                got: values.len(),
            });
        }
        if values.iter().any(|v| !(0.0..=1.0).contains(v)) {
            return Err(WmaError::Range);
        }
        Ok(AlphaMap {
            width,
            height,
            values,
        })
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn values(&self) -> &[f32] {
        &self.values
    }

    /// `α` at `(x, y)`; zero outside the map.
    pub fn get(&self, x: i64, y: i64) -> f32 {
        if x < 0 || y < 0 || x >= i64::from(self.width) || y >= i64::from(self.height) {
            return 0.0;
        }
        self.values[y as usize * self.width as usize + x as usize]
    }

    /// The largest value in the map.
    pub fn peak(&self) -> f32 {
        self.values.iter().copied().fold(0.0, f32::max)
    }

    /// Read a `.wma`.
    pub fn read(bytes: &[u8]) -> Result<Self, WmaError> {
        if bytes.len() < HEADER || &bytes[..4] != MAGIC {
            return Err(WmaError::Magic);
        }
        let width = u32::from(u16::from_le_bytes([bytes[4], bytes[5]]));
        let height = u32::from(u16::from_le_bytes([bytes[6], bytes[7]]));
        let depth = bytes[8];
        let per = match depth {
            8 => 1,
            16 => 2,
            other => return Err(WmaError::Depth(other)),
        };
        if width == 0 || height == 0 {
            return Err(WmaError::ZeroDimension);
        }
        let body = &bytes[HEADER..];
        let needed = width as usize * height as usize * per;
        if body.len() != needed {
            return Err(WmaError::Size {
                needed,
                got: body.len(),
            });
        }
        let values = if per == 1 {
            body.iter()
                .map(|&s| f32::from(s) / f32::from(u8::MAX))
                .collect()
        } else {
            body.chunks_exact(2)
                .map(|s| f32::from(u16::from_le_bytes([s[0], s[1]])) / f32::from(u16::MAX))
                .collect()
        };
        AlphaMap::new(width, height, values)
    }

    /// Write a `.wma` at `depth` (8 or 16), each value rounded to the
    /// nearest sample. A map wider or taller than 65535 cannot be written.
    pub fn write(&self, depth: u8) -> Result<Vec<u8>, WmaError> {
        let (Ok(width), Ok(height)) = (u16::try_from(self.width), u16::try_from(self.height))
        else {
            return Err(WmaError::Size {
                needed: usize::from(u16::MAX),
                got: self.width.max(self.height) as usize,
            });
        };
        let mut out = MAGIC.to_vec();
        out.extend_from_slice(&width.to_le_bytes());
        out.extend_from_slice(&height.to_le_bytes());
        out.push(depth);
        match depth {
            8 => out.extend(
                self.values
                    .iter()
                    .map(|v| (v * f32::from(u8::MAX)).round() as u8),
            ),
            16 => {
                for v in &self.values {
                    out.extend_from_slice(
                        &((v * f32::from(u16::MAX)).round() as u16).to_le_bytes(),
                    );
                }
            }
            other => return Err(WmaError::Depth(other)),
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_map_survives_a_round_trip_at_either_depth() {
        let map = AlphaMap::new(3, 2, vec![0.0, 0.5, 1.0, 0.25, 0.75, 0.125]).unwrap();
        for depth in [8, 16] {
            let back = AlphaMap::read(&map.write(depth).unwrap()).unwrap();
            assert_eq!((back.width(), back.height()), (3, 2));
            let step = if depth == 8 {
                1.0 / 255.0
            } else {
                1.0 / 65535.0
            };
            for (a, b) in map.values().iter().zip(back.values()) {
                assert!((a - b).abs() <= step, "{depth}: {a} {b}");
            }
        }
        // An 8-bit sample is read back to the very value written.
        let eight = AlphaMap::read(b"WMA1\x02\x00\x01\x00\x08\x80\xff").unwrap();
        assert_eq!(eight.values(), [128.0 / 255.0, 1.0]);
        assert_eq!(AlphaMap::read(&eight.write(8).unwrap()).unwrap(), eight);
    }

    #[test]
    fn a_bad_map_is_refused_by_name() {
        assert_eq!(
            AlphaMap::read(b"WMA2\x01\x00\x01\x00\x08\x00"),
            Err(WmaError::Magic)
        );
        assert_eq!(
            AlphaMap::read(b"WMA1\x01\x00\x01\x00\x0c\x00"),
            Err(WmaError::Depth(12))
        );
        assert_eq!(
            AlphaMap::read(b"WMA1\x00\x00\x01\x00\x08"),
            Err(WmaError::ZeroDimension)
        );
        assert_eq!(
            AlphaMap::read(b"WMA1\x02\x00\x01\x00\x08\x00"),
            Err(WmaError::Size { needed: 2, got: 1 })
        );
        assert_eq!(AlphaMap::read(b"WMA1"), Err(WmaError::Magic));
        assert_eq!(AlphaMap::new(1, 1, vec![1.5]), Err(WmaError::Range));
        let whole = AlphaMap::new(2, 2, vec![0.1; 4])
            .unwrap()
            .write(16)
            .unwrap();
        for cut in 0..whole.len() {
            assert!(AlphaMap::read(&whole[..cut]).is_err(), "{cut}");
        }
    }
}
