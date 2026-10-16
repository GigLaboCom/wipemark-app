//! The one EXIF field this crate reads: IFD0's Orientation (tag
//! `0x0112`). Everything else in EXIF is searched as bytes, never
//! parsed (`signatures.rs`); this is read only so a strip can say that
//! the rotation a picture relied on went with a block it removed. A
//! block is still kept or removed whole.

/// The EXIF Orientation tag (TIFF/EP, CIPA DC-008 §4.6.4).
const ORIENTATION: u16 = 0x0112;
/// TIFF type `SHORT`.
const SHORT: u16 = 3;

/// The Orientation in an EXIF payload, when it is one that turns or
/// mirrors the picture — 2 to 8. `None` for 1 (as stored), for a value
/// outside 1–8 (no viewer acts on it), for no tag, and for anything that
/// does not read. `data` is a TIFF stream, with or without the
/// `Exif\0\0` that JPEG and some WebP and PNG writers put in front.
pub(crate) fn orientation(data: &[u8]) -> Option<u16> {
    let tiff = data.strip_prefix(b"Exif\0\0").unwrap_or(data);
    let big = match tiff.get(..4)? {
        b"MM\0*" => true,
        b"II*\0" => false,
        _ => return None,
    };
    let u16_at = |at: usize| -> Option<u16> {
        let b = tiff.get(at..at.checked_add(2)?)?;
        Some(if big {
            u16::from_be_bytes([b[0], b[1]])
        } else {
            u16::from_le_bytes([b[0], b[1]])
        })
    };
    let u32_at = |at: usize| -> Option<u32> {
        let b = tiff.get(at..at.checked_add(4)?)?;
        Some(if big {
            u32::from_be_bytes([b[0], b[1], b[2], b[3]])
        } else {
            u32::from_le_bytes([b[0], b[1], b[2], b[3]])
        })
    };
    let ifd = usize::try_from(u32_at(4)?).ok()?;
    let count = usize::from(u16_at(ifd)?);
    for i in 0..count {
        let entry = ifd.checked_add(2)?.checked_add(i.checked_mul(12)?)?;
        let tag = u16_at(entry)?;
        if tag != ORIENTATION {
            continue;
        }
        if u16_at(entry + 2)? != SHORT || u32_at(entry + 4)? != 1 {
            return None;
        }
        let value = u16_at(entry + 8)?;
        return (2..=8).contains(&value).then_some(value);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::orientation;

    /// A TIFF with one IFD0 entry, in either byte order.
    fn tiff(big: bool, tag: u16, ty: u16, count: u32, value: u16) -> Vec<u8> {
        let mut t = if big {
            b"MM\0*\0\0\0\x08".to_vec()
        } else {
            b"II*\0\x08\0\0\0".to_vec()
        };
        let u16b = |v: u16| {
            if big {
                v.to_be_bytes()
            } else {
                v.to_le_bytes()
            }
        };
        let u32b = |v: u32| {
            if big {
                v.to_be_bytes()
            } else {
                v.to_le_bytes()
            }
        };
        t.extend_from_slice(&u16b(1));
        t.extend_from_slice(&u16b(tag));
        t.extend_from_slice(&u16b(ty));
        t.extend_from_slice(&u32b(count));
        t.extend_from_slice(&u16b(value));
        t.extend_from_slice(&[0, 0]);
        t.extend_from_slice(&u32b(0));
        t
    }

    #[test]
    fn the_orientation_is_read_in_either_byte_order_and_behind_the_exif_header() {
        assert_eq!(orientation(&tiff(true, 0x0112, 3, 1, 6)), Some(6));
        assert_eq!(orientation(&tiff(false, 0x0112, 3, 1, 8)), Some(8));
        let mut app1 = b"Exif\0\0".to_vec();
        app1.extend_from_slice(&tiff(false, 0x0112, 3, 1, 3));
        assert_eq!(orientation(&app1), Some(3));
    }

    #[test]
    fn what_is_not_a_rotation_is_not_reported() {
        // As stored, out of range, another tag, another type.
        assert_eq!(orientation(&tiff(true, 0x0112, 3, 1, 1)), None);
        assert_eq!(orientation(&tiff(true, 0x0112, 3, 1, 9)), None);
        assert_eq!(orientation(&tiff(true, 0x0112, 3, 1, 0)), None);
        assert_eq!(orientation(&tiff(true, 0x010F, 3, 1, 6)), None);
        assert_eq!(orientation(&tiff(true, 0x0112, 4, 1, 6)), None);
    }

    #[test]
    fn a_tiff_that_does_not_read_is_no_orientation_and_no_panic() {
        let whole = tiff(true, 0x0112, 3, 1, 6);
        for cut in 0..whole.len() {
            let _ = orientation(&whole[..cut]);
        }
        // An IFD offset past the end, and an entry count past it.
        assert_eq!(orientation(b"MM\0*\xff\xff\xff\xff"), None);
        assert_eq!(orientation(b"MM\0*\0\0\0\x08\xff\xff"), None);
        assert_eq!(orientation(b"not a tiff"), None);
    }
}
