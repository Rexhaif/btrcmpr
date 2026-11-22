// A 16-bit signal path. Sixteen parallel lanes predict the next group;
// bounded residuals occupy one byte, with large corrections in a side stream.
use crate::{Result, bad, take, u32in};
fn val(s: &[u8], i: usize) -> u16 {
    u16::from_le_bytes([s[i], s[i + 1]])
}
pub fn detect(s: &[u8]) -> bool {
    if s.len() < 8192 {
        return false;
    }
    let mut near = 0;
    let mut nonzero = 0;
    for k in 0..512 {
        let i = ((s.len() - 64) * k / 512) & !1;
        let d = val(s, i + 2).wrapping_sub(val(s, i)) as i16;
        near += usize::from(d.unsigned_abs() < 160);
        nonzero += usize::from(d != 0)
    }
    near > 420 && nonzero > 400
}
#[cfg(test)]
pub fn encode(s: &[u8]) -> Vec<u8> {
    let mut out = vec![2];
    let mut hi = vec![];
    let mut low = vec![];
    let end = s.len() / 32 * 32;
    out.extend(&s[..32]);
    // Predictor is the corresponding pixel in the preceding group.
    for i in (32..end).step_by(32) {
        let mut mask = 0u16;
        let start = hi.len();
        hi.extend([0, 0]);
        for j in 0..16 {
            let at = i + j * 2;
            let d = val(s, at).wrapping_sub(val(s, at - 32)).wrapping_add(128);
            low.push(d as u8);
            if d > 255 {
                mask |= 1 << j;
                hi.push((d >> 8) as u8)
            }
        }
        hi[start..start + 2].copy_from_slice(&mask.to_le_bytes())
    }
    out.extend((low.len() as u32).to_le_bytes());
    out.extend(low);
    out.extend(hi);
    out.extend(&s[end..]);
    out
}
pub fn decode(mut p: &[u8], d: &mut [u8]) -> Result<()> {
    if d.len() < 32 {
        return Err(bad());
    }
    let end = d.len() / 32 * 32;
    d[..32].copy_from_slice(take(&mut p, 32)?);
    let n = u32in(&mut p)?;
    if n != (end - 32) / 2 {
        return Err(bad());
    }
    let low = take(&mut p, n)?;
    let mut k = 0;
    for i in (32..end).step_by(32) {
        let m = u16::from_le_bytes(take(&mut p, 2)?.try_into().unwrap());
        for j in 0..16 {
            let mut v = low[k] as u16;
            k += 1;
            if (m >> j) & 1 != 0 {
                v |= (take(&mut p, 1)?[0] as u16) << 8
            }
            let at = i + 2 * j;
            let x = val(d, at - 32).wrapping_add(v).wrapping_sub(128);
            d[at..at + 2].copy_from_slice(&x.to_le_bytes())
        }
    }
    if p.len() != d.len() - end {
        return Err(bad());
    }
    d[end..].copy_from_slice(p);
    Ok(())
}

// Store low delta bytes together; only groups with exceptions carry a
// 16-bit mask. Each NEON register holds eight deltas.
pub fn adjacent(s: &[u8]) -> Vec<u8> {
    if s.len() < 2 {
        let mut r = vec![0];
        r.extend(s);
        return r;
    }
    let groups = (s.len() - 2) / 32;
    let bitmap = 3 + groups * 16;
    let mut b = Vec::with_capacity(bitmap + groups.div_ceil(8) + groups);
    b.resize(bitmap + groups.div_ceil(8), 0);
    b[0] = 6;
    b[1..3].copy_from_slice(&s[..2]);
    for g in 0..groups {
        let i = 2 + 32 * g;
        let hi = split16(&s[i - 2..i + 32], &mut b[3 + 16 * g..3 + 16 * (g + 1)]);
        if hi != [0; 16] {
            b[bitmap + g / 8] |= 1 << (g % 8);
            let m = b.len();
            b.extend([0, 0]);
            let mut mask = 0u16;
            for (j, v) in hi.into_iter().enumerate() {
                if v != 0 {
                    mask |= 1 << j;
                    b.push(v);
                }
            }
            b[m..m + 2].copy_from_slice(&mask.to_le_bytes());
        }
    }
    b.extend(&s[2 + groups * 32..]);
    b
}
#[inline(always)]
fn split16(s: &[u8], lo: &mut [u8]) -> [u8; 16] {
    #[cfg(target_arch = "aarch64")]
    unsafe {
        use std::arch::aarch64::*;
        let p = s.as_ptr().cast::<u16>();
        let bias = vdupq_n_u16(128);
        let a = vreinterpretq_u8_u16(vaddq_u16(
            bias,
            vsubq_u16(vld1q_u16(p.add(1)), vld1q_u16(p)),
        ));
        let c = vreinterpretq_u8_u16(vaddq_u16(
            bias,
            vsubq_u16(vld1q_u16(p.add(9)), vld1q_u16(p.add(8))),
        ));
        vst1q_u8(lo.as_mut_ptr(), vuzp1q_u8(a, c));
        let high = vuzp2q_u8(a, c);
        if vmaxvq_u8(high) == 0 {
            return [0; 16];
        }
        let mut ret = [0u8; 16];
        vst1q_u8(ret.as_mut_ptr(), high);
        ret
    }
    #[cfg(not(target_arch = "aarch64"))]
    {
        let mut hi = [0; 16];
        for j in 0..16 {
            let x = val(s, j * 2 + 2)
                .wrapping_sub(val(s, j * 2))
                .wrapping_add(128);
            lo[j] = x as u8;
            hi[j] = (x >> 8) as u8;
        }
        hi
    }
}
#[inline(always)]
fn join16(lo: &[u8], hi: &[u8; 16], dst: &mut [u8], carry: u16) -> u16 {
    #[cfg(target_arch = "aarch64")]
    unsafe {
        use std::arch::aarch64::*;
        let low = vld1q_u8(lo.as_ptr());
        let high = vld1q_u8(hi.as_ptr());
        let zero = vdupq_n_u16(0);
        macro_rules! scan {
            ($v:expr) => {{
                let v = vsubq_u16(vreinterpretq_u16_u8($v), vdupq_n_u16(128));
                let v = vaddq_u16(v, vextq_u16::<7>(zero, v));
                let v = vaddq_u16(v, vextq_u16::<6>(zero, v));
                vaddq_u16(v, vextq_u16::<4>(zero, v))
            }};
        }
        let a = vaddq_u16(scan!(vzip1q_u8(low, high)), vdupq_n_u16(carry));
        let c = vaddq_u16(scan!(vzip2q_u8(low, high)), vdupq_laneq_u16::<7>(a));
        let d = dst.as_mut_ptr().cast::<u16>();
        vst1q_u16(d, a);
        vst1q_u16(d.add(8), c);
        vgetq_lane_u16::<7>(c)
    }
    #[cfg(not(target_arch = "aarch64"))]
    {
        let mut last = carry;
        for j in 0..16 {
            last = last
                .wrapping_add(u16::from_le_bytes([lo[j], hi[j]]))
                .wrapping_sub(128);
            dst[j * 2..j * 2 + 2].copy_from_slice(&last.to_le_bytes());
        }
        last
    }
}
pub fn encode_level(s: &[u8], level: u8) -> Vec<u8> {
    let normal = adjacent(s);
    if level < 5 || s.len() < 8192 {
        return normal;
    }
    let stop = 3 + (s.len() - 2) / 32 * 16;
    let mut b = vec![10, normal[1], normal[2]];
    crate::ans::put_policy(&normal[3..stop], &mut b, 0.985);
    b.extend(&normal[stop..]);
    if b.len() < normal.len() { b } else { normal }
}
pub fn unadjacent(mut p: &[u8], d: &mut [u8], coded: bool) -> Result<()> {
    if d.len() < 2 {
        return Err(bad());
    }
    let g = (d.len() - 2) / 32;
    let tail = (d.len() - 2) % 32;
    let seed = take(&mut p, 2)?;
    let mut carry = val(seed, 0);
    d[..2].copy_from_slice(seed);
    let storage;
    let low = if coded {
        storage = crate::ans::get(&mut p, 16 * g)?;
        if storage.len() != 16 * g {
            return Err(bad());
        }
        &storage
    } else {
        take(&mut p, 16 * g)?
    };
    let flags = take(&mut p, g.div_ceil(8))?;
    for i in 0..g {
        let mut hi = [0; 16];
        if (flags[i / 8] >> (i % 8)) & 1 != 0 {
            let mut m = u16::from_le_bytes(take(&mut p, 2)?.try_into().unwrap());
            let ex = take(&mut p, m.count_ones() as usize)?;
            for &x in ex {
                hi[m.trailing_zeros() as usize] = x;
                m &= m - 1;
            }
        }
        carry = join16(
            &low[i * 16..i * 16 + 16],
            &hi,
            &mut d[2 + i * 32..2 + (i + 1) * 32],
            carry,
        );
    }
    if p.len() != tail {
        return Err(bad());
    }
    d[2 + 32 * g..].copy_from_slice(p);
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn old_archives_and_all_adjacent_tails() {
        let mut seed = 17u32;
        let src: Vec<u8> = (0..12023)
            .map(|_| {
                seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                (seed >> 24) as u8
            })
            .collect();
        for n in 8192..8256 {
            let s = &src[..n];
            let b = encode(s);
            let mut dst = vec![0; n];
            decode(&b[1..], &mut dst).unwrap();
            assert_eq!(s, dst);
            for l in [1, 8] {
                let b = encode_level(s, l);
                unadjacent(&b[1..], &mut dst, b[0] == 10).unwrap();
                assert_eq!(s, dst);
                assert!(unadjacent(&b[1..b.len() - 1], &mut dst, b[0] == 10).is_err());
            }
        }
    }
}
