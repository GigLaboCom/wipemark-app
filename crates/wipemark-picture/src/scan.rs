//! Whether a baseline JPEG's scan is whole — read without decoding a
//! pixel.
//!
//! A decoder that meets a scan it cannot follow recovers: it fills what
//! is missing and carries on, and zune-jpeg does so without saying so,
//! strict mode included. A restoration over what it filled in, re-encoded,
//! would hand back a picture the file never held. So the scan is walked
//! here first, code by code, against the frame it belongs to: every
//! Huffman code must decode, the frame's blocks must all be read before the
//! data runs out, the restart markers must come where the interval says,
//! and the scan must end there — its last byte padded with 1-bits, nothing
//! after it. A bit flipped in a Huffman code desynchronises the walk and
//! fails one of those; a bit flipped in a coefficient's own magnitude bits
//! changes that one value and nothing else, and no walk can tell it from
//! the picture — about half a scan's bits are those. A progressive or
//! arithmetic-coded JPEG is not walked ([`Scan::NotWalked`]).

/// What the walk found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scan {
    /// Every block of every component read, ending where the scan ends.
    Whole,
    /// A code that does not decode, data that ran out, a restart out of
    /// place, or data left over.
    Damaged,
    /// Not a single-scan baseline or extended Huffman JPEG.
    NotWalked,
}

/// One Huffman table, canonical: per code length, the first code, the
/// last code and where its symbols start.
#[derive(Clone)]
struct Table {
    first: [i32; 17],
    last: [i32; 17],
    at: [usize; 17],
    symbols: Vec<u8>,
}

impl Table {
    fn new(counts: &[u8; 16], symbols: Vec<u8>) -> Option<Self> {
        let (mut first, mut last, mut at) = ([0i32; 17], [-1i32; 17], [0usize; 17]);
        let (mut code, mut k) = (0i32, 0usize);
        for len in 1..=16 {
            let n = usize::from(counts[len - 1]);
            first[len] = code;
            at[len] = k;
            if n > 0 {
                last[len] = code + n as i32 - 1;
            }
            code = (code + n as i32) << 1;
            k += n;
        }
        (k == symbols.len()).then_some(Table {
            first,
            last,
            at,
            symbols,
        })
    }
}

/// Bits of the entropy-coded data, `FF 00` unstuffed, stopping at a
/// marker.
struct Bits<'a> {
    b: &'a [u8],
    pos: usize,
    acc: u32,
    n: u32,
    /// A marker was reached, or the bytes ended.
    stopped: bool,
}

impl<'a> Bits<'a> {
    fn new(b: &'a [u8], pos: usize) -> Self {
        Bits {
            b,
            pos,
            acc: 0,
            n: 0,
            stopped: false,
        }
    }

    fn bit(&mut self) -> Option<u32> {
        if self.n == 0 {
            if self.stopped || self.pos >= self.b.len() {
                self.stopped = true;
                return None;
            }
            let byte = self.b[self.pos];
            if byte == 0xFF {
                match self.b.get(self.pos + 1) {
                    Some(0x00) => self.pos += 2,
                    _ => {
                        self.stopped = true;
                        return None;
                    }
                }
            } else {
                self.pos += 1;
            }
            self.acc = u32::from(byte);
            self.n = 8;
        }
        self.n -= 1;
        Some((self.acc >> self.n) & 1)
    }

    fn bits(&mut self, count: u8) -> Option<()> {
        for _ in 0..count {
            self.bit()?;
        }
        Some(())
    }

    fn decode(&mut self, t: &Table) -> Option<u8> {
        let mut code = 0i32;
        for len in 1..=16 {
            code = (code << 1) | self.bit()? as i32;
            if code <= t.last[len] {
                return t
                    .symbols
                    .get(t.at[len] + (code - t.first[len]) as usize)
                    .copied();
            }
        }
        None
    }

    /// The rest of the current byte is padding: all 1-bits.
    fn padded(&mut self) -> bool {
        let rest = self.n;
        let ones = (1u32 << rest) - 1;
        let ok = self.acc & ones == ones;
        self.n = 0;
        ok
    }
}

struct Component {
    id: u8,
    h: usize,
    v: usize,
}

/// Walk the first scan of `b`, a JPEG that the container parser read.
pub fn walk(b: &[u8]) -> Scan {
    let mut dc: [Option<Table>; 4] = [None, None, None, None];
    let mut ac: [Option<Table>; 4] = [None, None, None, None];
    let mut frame: Option<(usize, usize, Vec<Component>)> = None;
    let mut interval = 0usize;
    let mut pos = 2;
    while pos + 4 <= b.len() {
        if b[pos] != 0xFF {
            return Scan::NotWalked;
        }
        let marker = b[pos + 1];
        if marker == 0xFF {
            pos += 1;
            continue;
        }
        let len = usize::from(u16::from_be_bytes([b[pos + 2], b[pos + 3]]));
        let Some(body) = b.get(pos + 4..pos + 2 + len) else {
            return Scan::NotWalked;
        };
        match marker {
            0xC4 => {
                let mut p = 0;
                while p + 17 <= body.len() {
                    let (class, id) = (body[p] >> 4, usize::from(body[p] & 15));
                    let mut counts = [0u8; 16];
                    counts.copy_from_slice(&body[p + 1..p + 17]);
                    let n: usize = counts.iter().map(|&c| usize::from(c)).sum();
                    let Some(symbols) = body.get(p + 17..p + 17 + n) else {
                        return Scan::NotWalked;
                    };
                    let (Some(table), true) = (Table::new(&counts, symbols.to_vec()), id < 4)
                    else {
                        return Scan::NotWalked;
                    };
                    if class == 0 {
                        dc[id] = Some(table);
                    } else {
                        ac[id] = Some(table);
                    }
                    p += 17 + n;
                }
            }
            0xC0 | 0xC1 => {
                if body.len() < 6 {
                    return Scan::NotWalked;
                }
                let height = usize::from(u16::from_be_bytes([body[1], body[2]]));
                let width = usize::from(u16::from_be_bytes([body[3], body[4]]));
                let n = usize::from(body[5]);
                let mut components = Vec::new();
                for c in 0..n {
                    let Some(&[id, hv, _]) = body.get(6 + 3 * c..9 + 3 * c) else {
                        return Scan::NotWalked;
                    };
                    let (h, v) = (usize::from(hv >> 4), usize::from(hv & 15));
                    if h == 0 || v == 0 {
                        return Scan::NotWalked;
                    }
                    components.push(Component { id, h, v });
                }
                frame = Some((width, height, components));
            }
            // Progressive, lossless, arithmetic: not walked.
            0xC2 | 0xC3 | 0xC5..=0xC7 | 0xC9..=0xCB | 0xCD..=0xCF => return Scan::NotWalked,
            0xDD => {
                if body.len() < 2 {
                    return Scan::NotWalked;
                }
                interval = usize::from(u16::from_be_bytes([body[0], body[1]]));
            }
            0xDA => {
                let Some((width, height, components)) = &frame else {
                    return Scan::NotWalked;
                };
                return scan(
                    b,
                    pos + 2 + len,
                    body,
                    *width,
                    *height,
                    components,
                    &dc,
                    &ac,
                    interval,
                );
            }
            0xD9 => return Scan::NotWalked,
            _ => {}
        }
        pos += 2 + len;
    }
    Scan::NotWalked
}

#[allow(clippy::too_many_arguments)]
fn scan(
    b: &[u8],
    start: usize,
    header: &[u8],
    width: usize,
    height: usize,
    components: &[Component],
    dc: &[Option<Table>; 4],
    ac: &[Option<Table>; 4],
    interval: usize,
) -> Scan {
    let Some(&n) = header.first() else {
        return Scan::NotWalked;
    };
    let n = usize::from(n);
    // A scan of fewer components than the frame is a multi-scan file:
    // not walked.
    if n != components.len() || header.len() < 1 + 2 * n + 3 {
        return Scan::NotWalked;
    }
    let (hmax, vmax) = (
        components.iter().map(|c| c.h).max().unwrap_or(1),
        components.iter().map(|c| c.v).max().unwrap_or(1),
    );
    let mut order = Vec::new();
    for k in 0..n {
        let (id, tables) = (header[1 + 2 * k], header[2 + 2 * k]);
        let Some(c) = components.iter().find(|c| c.id == id) else {
            return Scan::NotWalked;
        };
        let (Some(d), Some(a)) = (
            dc.get(usize::from(tables >> 4)).and_then(Option::as_ref),
            ac.get(usize::from(tables & 15)).and_then(Option::as_ref),
        ) else {
            return Scan::Damaged;
        };
        order.push((c, d, a));
    }
    // Interleaved when there is more than one component; one component's
    // blocks are its own when there is not.
    let units = if n == 1 {
        let c = order[0].0;
        let cw = (width * c.h).div_ceil(hmax);
        let ch = (height * c.v).div_ceil(vmax);
        cw.div_ceil(8) * ch.div_ceil(8)
    } else {
        width.div_ceil(8 * hmax) * height.div_ceil(8 * vmax)
    };
    let mut bits = Bits::new(b, start);
    for unit in 0..units {
        if interval > 0 && unit > 0 && unit % interval == 0 {
            // The restart: the byte padded out, then exactly the next RSTn.
            if !bits.padded() {
                return Scan::Damaged;
            }
            let expected = 0xD0 + ((unit / interval - 1) % 8) as u8;
            if b.get(bits.pos) != Some(&0xFF) || b.get(bits.pos + 1) != Some(&expected) {
                return Scan::Damaged;
            }
            bits = Bits::new(b, bits.pos + 2);
        }
        for (c, d, a) in &order {
            let blocks = if n == 1 { 1 } else { c.h * c.v };
            for _ in 0..blocks {
                if block(&mut bits, d, a).is_none() {
                    return Scan::Damaged;
                }
            }
        }
    }
    // Ends here: the byte padded with 1-bits, then a marker.
    if !bits.padded() || b.get(bits.pos) != Some(&0xFF) {
        return Scan::Damaged;
    }
    match b.get(bits.pos + 1) {
        Some(0x00) | Some(0xD0..=0xD7) | None => Scan::Damaged,
        Some(_) => Scan::Whole,
    }
}

/// One 8×8 block's codes: a DC difference, then AC runs to an end of
/// block or the 63rd coefficient.
fn block(bits: &mut Bits<'_>, dc: &Table, ac: &Table) -> Option<()> {
    let s = bits.decode(dc)?;
    if s > 11 {
        return None;
    }
    bits.bits(s)?;
    let mut k = 1;
    while k < 64 {
        let rs = bits.decode(ac)?;
        let (r, s) = (rs >> 4, rs & 15);
        if s == 0 {
            if r == 15 {
                k += 16;
                continue;
            }
            return Some(());
        }
        k += usize::from(r);
        if k > 63 || s > 10 {
            return None;
        }
        bits.bits(s)?;
        k += 1;
    }
    (k == 64).then_some(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jpeg(width: u32, height: u32, quality: u8, grey: bool) -> Vec<u8> {
        let samples: Vec<u8> = (0..width * height * 3)
            .map(|i| ((i * 37 + (i / 7) * 11) % 251) as u8)
            .collect();
        let (bytes, colour) = if grey {
            (
                samples.iter().step_by(3).copied().collect::<Vec<u8>>(),
                image::ExtendedColorType::L8,
            )
        } else {
            (samples, image::ExtendedColorType::Rgb8)
        };
        let mut out = Vec::new();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, quality)
            .encode(&bytes, width, height, colour)
            .unwrap();
        out
    }

    fn scan_start(b: &[u8]) -> usize {
        let mut pos = 2;
        loop {
            let len = usize::from(u16::from_be_bytes([b[pos + 2], b[pos + 3]]));
            if b[pos + 1] == 0xDA {
                return pos + 2 + len;
            }
            pos += 2 + len;
        }
    }

    #[test]
    fn an_encoders_own_scan_is_whole() {
        for (w, h, q, grey) in [(64, 48, 95, false), (33, 17, 85, false), (40, 40, 90, true)] {
            assert_eq!(
                walk(&jpeg(w, h, q, grey)),
                Scan::Whole,
                "{w}x{h} q{q} grey {grey}"
            );
        }
    }

    /// Single-bit flips across a scan (every fourth byte): the ones in
    /// Huffman codes break the walk; the ones in coefficients' magnitude
    /// bits cannot, and are about half (of every flip of this scan, 27 138
    /// of 48 154 are damage).
    #[test]
    fn a_flipped_code_breaks_the_walk() {
        let b = jpeg(64, 48, 95, false);
        let (start, end) = (scan_start(&b), b.len() - 2);
        let (mut damaged, mut total) = (0, 0);
        for at in (start..end).step_by(4) {
            if b[at] == 0xFF || b[at - 1] == 0xFF {
                continue;
            }
            for bit in 0..8 {
                let mut x = b.clone();
                x[at] ^= 1 << bit;
                if x[at] == 0xFF {
                    continue;
                }
                total += 1;
                damaged += usize::from(walk(&x) == Scan::Damaged);
            }
        }
        assert!(damaged * 2 >= total, "{damaged} of {total}");
    }

    #[test]
    fn a_scan_cut_short_is_damaged() {
        let b = jpeg(64, 48, 95, false);
        let start = scan_start(&b);
        let mut cut = b[..start + (b.len() - start) / 2].to_vec();
        cut.extend_from_slice(&[0xFF, 0xD9]);
        assert_eq!(walk(&cut), Scan::Damaged);
    }
}
