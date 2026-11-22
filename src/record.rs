// Four vertical lanes share a bit width. Each group can predict from a
// constant base or the previous row; exceptional high bits are sent separately.
use crate::{Result, bad, take};
const N: usize = 128;
const WIDTHS: &[usize] = &[
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 18, 20, 22, 24, 26, 28, 30, 32,
];
#[inline]
fn read(s: &[u8], p: usize) -> u32 {
    u32::from_le_bytes(s[p..p + 4].try_into().unwrap())
}
fn width(v: &[u32; N], n: usize) -> (usize, usize, usize) {
    let mut hist = [0usize; 33];
    for &x in &v[..n] {
        hist[(32 - x.leading_zeros()) as usize] += 1
    }
    let mut best = (usize::MAX, 0, 0);
    for &w in WIDTHS {
        let mut bytes = w * 16;
        let mut exc = 0;
        for (b, &c) in hist.iter().enumerate().skip(w + 1) {
            bytes += c * (1 + (b - w).div_ceil(7));
            exc += c
        }
        if bytes < best.0 {
            best = (bytes, w, exc)
        }
    }
    best
}

mod recordfast;
fn field<const MODE: u8>(values: &[u32; N], n: usize, prev: u32, out: &mut Vec<u8>) {
    if MODE >= 2 && values[..n].iter().all(|&v| v == values[0]) {
        recordfast::constant(values[0], prev, out);
        return;
    }
    let mut raw = [0u32; N];
    let mut delta = [0u32; N];
    let base = *values[..n].iter().min().unwrap();
    let mut last = prev;
    for i in 0..n {
        let x = values[i];
        raw[i] = x - base;
        let d = x.wrapping_sub(last);
        delta[i] = (d << 1) ^ ((d as i32 >> 31) as u32);
        last = x
    }
    let price = if MODE == 0 { width } else { recordfast::width };
    let both = || (price(&raw, n), price(&delta, n));

    let ((a, w0, e0), (b, w1, e1)) = both();
    let diff = b < a + 4 * (base != 0) as usize;
    let (v, w, ex) = if diff {
        (&delta, w1, e1)
    } else {
        (&raw, w0, e0)
    };
    let has_base = !diff && base != 0;
    out.push(w as u8 | ((diff as u8) << 6) | ((has_base as u8) << 7));
    out.push(ex as u8);
    if has_base {
        out.extend(base.to_le_bytes())
    }
    let mask = (1u64 << w) - 1;
    let mut bits = [0u64; 4];
    let mut used = 0;

    if w != 0 {
        for x in v.chunks_exact(4) {
            for l in 0..4 {
                bits[l] |= ((x[l] as u64) & mask) << used
            }
            used += w;
            if used >= 32 {
                for x in &mut bits {
                    out.extend((*x as u32).to_le_bytes());
                    *x >>= 32
                }
                used -= 32
            }
        }
    }
    if ex != 0 {
        for (i, &x) in v[..n].iter().enumerate() {
            let mut hi = (x as u64) >> w;
            if hi != 0 {
                out.push(i as u8);
                while hi >= 128 {
                    out.push(hi as u8 | 128);
                    hi >>= 7
                }
                out.push(hi as u8)
            }
        }
    }
}
pub fn encode(src: &[u8], w: usize, skip: usize) -> Vec<u8> {
    encode_impl::<0>(src, w, skip)
}
pub(crate) fn quick(src: &[u8], w: usize, skip: usize) -> Vec<u8> {
    encode_impl::<2>(src, w, skip)
}
#[cfg(test)]
pub(crate) fn fast(src: &[u8], w: usize, skip: usize, mode: usize) -> Vec<u8> {
    match mode {
        1 => encode_impl::<1>(src, w, skip),
        2 => encode_impl::<2>(src, w, skip),
        3 => encode_impl::<3>(src, w, skip),

        _ => unreachable!(),
    }
}
fn encode_impl<const MODE: u8>(src: &[u8], w: usize, skip: usize) -> Vec<u8> {
    let skip = skip.min(src.len());
    let n = (src.len() - skip) / (4 * w);
    let s = &src[skip..];
    let mut out = vec![4, w as u8, skip as u8];
    out.extend(&src[..skip]);
    out.extend(&s[n * 4 * w..]);
    let mut prev = [0u32; 8];
    let mut v = [0; N];
    for j in (0..n).step_by(N) {
        let rows = N.min(n - j);
        for (f, last) in prev[..w].iter_mut().enumerate() {
            for (i, x) in v[..rows].iter_mut().enumerate() {
                *x = read(s, 4 * ((j + i) * w + f))
            }
            field::<MODE>(&v, rows, *last, &mut out);
            *last = v[rows - 1]
        }
    }
    out
}
#[cfg(target_arch = "aarch64")]
#[inline(never)]
unsafe fn unpack<const W: usize>(src: *const u8, dst: *mut u32) {
    use std::arch::aarch64::*;
    unsafe {
        let mask = vdupq_n_u32(((1u64 << W) - 1) as u32);
        macro_rules! one {
            ($i:expr) => {{
                let bit = $i * W;
                let sh = bit % 32;
                let p = src.add((bit / 32) * 16).cast::<u32>();
                let mut v = vld1q_u32(p);
                if sh != 0 {
                    v = vshlq_u32(v, vdupq_n_s32(-(sh as i32)))
                }
                if sh + W > 32 {
                    v = vorrq_u32(
                        v,
                        vshlq_u32(vld1q_u32(p.add(4)), vdupq_n_s32(32 - sh as i32)),
                    )
                }
                vst1q_u32(dst.add($i * 4), vandq_u32(v, mask));
            }};
        }
        one!(0);
        one!(1);
        one!(2);
        one!(3);
        one!(4);
        one!(5);
        one!(6);
        one!(7);
        one!(8);
        one!(9);
        one!(10);
        one!(11);
        one!(12);
        one!(13);
        one!(14);
        one!(15);
        one!(16);
        one!(17);
        one!(18);
        one!(19);
        one!(20);
        one!(21);
        one!(22);
        one!(23);
        one!(24);
        one!(25);
        one!(26);
        one!(27);
        one!(28);
        one!(29);
        one!(30);
        one!(31);
    }
}
fn unfield(p: &mut &[u8], v: &mut [u32; N], n: usize, last: &mut u32) -> Result<()> {
    let h = take(p, 2)?;
    let w = (h[0] & 63) as usize;
    let ex = h[1] as usize;
    if w > 32 || ex > n {
        return Err(bad());
    }
    let diff = h[0] & 64 != 0;
    let base = if h[0] & 128 != 0 {
        read(take(p, 4)?, 0)
    } else {
        0
    };
    if w == 0 && ex == 0 {
        let x = if diff { *last } else { base };
        v.fill(x);
        *last = x;
        return Ok(());
    }
    let a = take(p, w * 16)?;
    if w == 0 {
        v.fill(0)
    } else {
        #[cfg(target_arch = "aarch64")]
        unsafe {
            macro_rules! choose{($($ww:literal),*)=>{match w{$($ww=>unpack::<$ww>(a.as_ptr(),v.as_mut_ptr()),)*_=>return Err(bad())}}}
            choose!(
                1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 18, 20, 22, 24, 26, 28, 30,
                32
            )
        }
        #[cfg(not(target_arch = "aarch64"))]
        {
            let mask = (1u64 << w) - 1;
            for j in 0..32 {
                let bit = j * w;
                for l in 0..4 {
                    let pos = (bit / 32) * 16 + l * 4;
                    let mut x = read(a, pos) as u64;
                    if bit % 32 + w > 32 {
                        x |= (read(a, pos + 16) as u64) << 32
                    }
                    v[4 * j + l] = ((x >> (bit % 32)) & mask) as u32
                }
            }
        }
    }
    for _ in 0..ex {
        let idx = take(p, 1)?[0] as usize;
        if idx >= n || w == 32 {
            return Err(bad());
        }
        let mut hi = 0u64;
        let mut sh = 0;
        loop {
            let b = take(p, 1)?[0];
            hi |= ((b & 127) as u64) << sh;
            if b < 128 {
                break;
            }
            sh += 7;
            if sh > 28 {
                return Err(bad());
            }
        }
        if hi > (u32::MAX >> w) as u64 {
            return Err(bad());
        }
        v[idx] |= (hi as u32) << w
    }
    if diff {
        #[cfg(target_arch = "aarch64")]
        unsafe {
            use std::arch::aarch64::*;
            let z = vdupq_n_u32(0);
            let mut prev = vdupq_n_u32(*last);
            for a in v.chunks_exact_mut(4) {
                let t = vld1q_u32(a.as_ptr());
                let t = veorq_u32(
                    vshrq_n_u32::<1>(t),
                    vsubq_u32(z, vandq_u32(t, vdupq_n_u32(1))),
                );
                let t = vaddq_u32(t, vextq_u32::<3>(z, t));
                let t = vaddq_u32(prev, vaddq_u32(t, vextq_u32::<2>(z, t)));
                vst1q_u32(a.as_mut_ptr(), t);
                prev = vdupq_laneq_u32::<3>(t)
            }
        }
        #[cfg(not(target_arch = "aarch64"))]
        {
            let mut p = *last;
            for x in v.iter_mut() {
                p = p.wrapping_add((*x >> 1) ^ (0u32.wrapping_sub(*x & 1)));
                *x = p
            }
        }
    } else if base != 0 {
        for x in v.iter_mut() {
            *x = x.wrapping_add(base)
        }
    }
    *last = v[n - 1];
    Ok(())
}
#[inline(always)]
fn emit(tile: &[[u32; N]; 8], w: usize, rows: usize, d: &mut [u8]) {
    let mut k = 0;
    #[cfg(target_arch = "aarch64")]
    unsafe {
        use std::arch::aarch64::*;
        while k + 4 <= rows {
            let out = d.as_mut_ptr().add(k * w * 4).cast::<u32>();
            let t = |f: usize| vld1q_u32(tile[f].as_ptr().add(k));
            match w {
                1 => vst1q_u32(out, t(0)),
                2 => vst2q_u32(out, uint32x4x2_t(t(0), t(1))),
                4 => vst4q_u32(out, uint32x4x4_t(t(0), t(1), t(2), t(3))),
                6 => {
                    let (a, b, c, e, g, h) = (t(0), t(1), t(2), t(3), t(4), t(5));
                    let lo = |x, y| vreinterpretq_u64_u32(vzip1q_u32(x, y));
                    let hi = |x, y| vreinterpretq_u64_u32(vzip2q_u32(x, y));
                    vst3q_u64(out.cast(), uint64x2x3_t(lo(a, b), lo(c, e), lo(g, h)));
                    vst3q_u64(
                        out.add(12).cast(),
                        uint64x2x3_t(hi(a, b), hi(c, e), hi(g, h)),
                    )
                }
                _ => break,
            }
            k += 4
        }
    }
    for (i, row) in d.chunks_exact_mut(4 * w).enumerate().take(rows).skip(k) {
        for (out, field) in row.chunks_exact_mut(4).zip(&tile[..w]) {
            out.copy_from_slice(&field[i].to_le_bytes())
        }
    }
}
pub(crate) struct Sink<'a, 'b> {
    s: &'a mut [(usize, &'b mut [u8])],
    i: usize,
    off: usize,
}
impl<'a, 'b> Sink<'a, 'b> {
    pub(crate) fn new(s: &'a mut [(usize, &'b mut [u8])]) -> Self {
        Self { s, i: 0, off: 0 }
    }
    pub(crate) fn tile(&mut self, tile: &[[u32; N]; 8], w: usize, rows: usize, tmp: &mut [u8]) {
        let need = rows * w * 4;
        let d = &mut self.s[self.i].1;
        if d.len() - self.off >= need {
            emit(tile, w, rows, &mut d[self.off..self.off + need]);
            self.off += need;
            if self.off == d.len() {
                self.off = 0;
                self.i += 1
            }
        } else {
            emit(tile, w, rows, &mut tmp[..need]);
            self.write(&tmp[..need])
        }
    }
    pub(crate) fn write(&mut self, mut bytes: &[u8]) {
        while !bytes.is_empty() {
            let d = &mut self.s[self.i].1;
            let k = bytes.len().min(d.len() - self.off);
            d[self.off..self.off + k].copy_from_slice(&bytes[..k]);
            bytes = &bytes[k..];
            self.off += k;
            if self.off == d.len() {
                self.i += 1;
                self.off = 0
            }
        }
    }
}
pub fn decode(p: &[u8], dst: &mut [u8]) -> Result<()> {
    let len = dst.len();
    decode_slices(p, len, &mut [(0, dst)])
}
pub fn decode_slices(mut p: &[u8], len: usize, s: &mut [(usize, &mut [u8])]) -> Result<()> {
    let h = take(&mut p, 2)?;
    let w = h[0] as usize;
    let skip = h[1] as usize;
    if !(1..=8).contains(&w) || skip > len || skip >= 4 * w {
        return Err(bad());
    }
    let mut sink = Sink { s, i: 0, off: 0 };
    sink.write(take(&mut p, skip)?);
    let n = (len - skip) / (4 * w);
    let tail = take(&mut p, len - skip - n * 4 * w)?;
    let mut tile = [[0u32; N]; 8];
    let mut prev = [0u32; 8];
    let mut tmp = [0u8; N * 8 * 4];
    for j in (0..n).step_by(N) {
        let rows = (n - j).min(N);
        for f in 0..w {
            unfield(&mut p, &mut tile[f], rows, &mut prev[f])?
        }
        sink.tile(&tile, w, rows, &mut tmp);
    }
    sink.write(tail);
    if !p.is_empty() {
        return Err(bad());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn record_widths_tails_wraparound_and_scatter() {
        for w in [1, 2, 4, 6, 8] {
            for rows in [1, 127, 128, 129, 401] {
                let mut src = vec![13u8; 3];
                let mut x = 0xfffff000u32;
                for i in 0..rows {
                    for f in 0..w {
                        x = x.wrapping_mul(1664525).wrapping_add(1013904223);
                        let value = match f % 5 {
                            0 => 0xffff_fff0u32.wrapping_add(i as u32 * 61),
                            1 => {
                                if i % 53 == 0 {
                                    x
                                } else {
                                    33
                                }
                            }
                            2 => x,
                            3 => i as u32 / 9,
                            _ => x >> 16,
                        };
                        src.extend(value.to_le_bytes())
                    }
                }
                src.extend([8, 9]);
                let b = encode(&src, w, 3);
                let mut dst = vec![0xee; src.len()];
                let mut parts: Vec<_> = dst
                    .chunks_mut(173)
                    .enumerate()
                    .map(|(i, d)| (i * 173, d))
                    .collect();
                decode_slices(&b[1..], src.len(), &mut parts).unwrap();
                assert_eq!(dst, src);
                assert!(decode(&b[1..b.len() - 1], &mut dst).is_err())
            }
        }
    }
}
