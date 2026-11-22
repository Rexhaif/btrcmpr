// Store the low, middle and high distance bytes in separate lanes. A selector
// determines how many bytes to read, including for the most distant sources.
use super::{BookTags, from_page, pages};
#[cfg(test)]
use super::{Streams, lenin};
#[cfg(test)]
use crate::ans;
use crate::{Result, bad, take};
pub(super) const SHAPES: [[u8; 3]; 8] = [
    [36, 133, 253],
    [56, 142, 253],
    [63, 180, 253],
    [24, 121, 253],
    [52, 148, 253],
    [36, 115, 253],
    [60, 188, 253],
    [84, 212, 253],
];

#[cfg(test)]
pub(super) fn put<const PACK: u8>(
    st: &Streams,
    shape: usize,
    ranked: bool,
    tight: bool,
    custom: Option<&[u8]>,
) -> Vec<u8> {
    let [a, b, c] = SHAPES[shape].map(usize::from);
    let mut heads = Vec::with_capacity(st.offs.len());
    let (mut lo, mut mi, mut high) = (
        Vec::with_capacity(st.low.len()),
        Vec::with_capacity(st.mid.len()),
        Vec::new(),
    );
    let (mut l, mut m, mut old_far) = (0, 0, st.far.as_slice());
    for &t in &st.offs {
        if t < 20 {
            heads.push(t);
            continue;
        }
        let z = if t < 84 {
            usize::from(t)
        } else {
            let low = usize::from(st.low[l]);
            l += 1;
            if t < 212 {
                low | (usize::from(t - 84) << 8)
            } else {
                let mid = usize::from(st.mid[m]);
                m += 1;
                let top = usize::from(t - 212)
                    + if t == 255 {
                        lenin(&mut old_far).expect("encoder distance")
                    } else {
                        0
                    };
                low | (mid << 8) | (top << 16)
            }
        };
        if z < a {
            heads.push(z as u8)
        } else if z < (b - a) << 8 {
            heads.push((a + (z >> 8)) as u8);
            lo.push(z as u8)
        } else if z < (c - b) << 16 {
            heads.push((b + (z >> 16)) as u8);
            lo.push(z as u8);
            mi.push((z >> 8) as u8)
        } else {
            debug_assert!(c + (z >> 24) <= 255);
            heads.push((c + (z >> 24)) as u8);
            lo.push(z as u8);
            mi.push((z >> 8) as u8);
            high.push((z >> 16) as u8)
        }
    }
    debug_assert!(l == st.low.len() && m == st.mid.len() && old_far.is_empty());
    let mut out = vec![42, shape as u8, st.minimum as u8];
    let plain = |v: &[u8], out: &mut Vec<u8>| {
        if tight {
            ans::put_tight(v, out)
        } else {
            ans::put_policy(v, out, 0.985)
        }
    };
    if PACK != 0 {
        ans::put_alternative(&st.tag, &mut out, 3)
    } else {
        plain(&st.tag, &mut out)
    }
    if ranked {
        ans::put_alternative(&heads, &mut out, 3)
    } else {
        plain(&heads, &mut out)
    }
    if PACK == 2 {
        ans::put_alternative(&st.lens, &mut out, 3)
    } else {
        plain(&st.lens, &mut out)
    }
    if let Some(literals) = custom {
        out.extend(literals)
    } else {
        plain(&st.lits, &mut out)
    }
    for v in [&mut lo, &mut mi, &mut high] {
        v.extend([0; 16]);
        plain(v, &mut out)
    }
    out
}

pub(crate) fn read(mut p: &[u8], dst: &mut [u8]) -> Result<()> {
    let h = take(&mut p, 2)?;
    let cuts = *SHAPES.get(h[0] as usize).ok_or_else(bad)?;
    macro_rules! variant {
        ($min:literal) => {
            from_page::<true, $min, false, false, false, false, false, true>(
                p,
                dst,
                &[],
                &[],
                0,
                BookTags::default(),
                &mut pages::Cursor::default(),
                cuts,
            )
        };
    }
    match h[1] {
        3 => variant!(3),
        4 => variant!(4),
        _ => Err(bad()),
    }
}
#[inline]
pub(super) fn scalar(
    c: u8,
    lo: &[u8],
    mi: &[u8],
    l: &mut usize,
    m: &mut usize,
    far: &mut &[u8],
    cuts: [u8; 3],
) -> Result<u32> {
    if c < cuts[0] {
        return Ok(u32::from(c));
    }
    let x = u32::from(*lo.get(*l).ok_or_else(bad)?);
    *l += 1;
    if c < cuts[1] {
        return Ok(x | (u32::from(c - cuts[0]) << 8));
    }
    let y = u32::from(*mi.get(*m).ok_or_else(bad)?);
    *m += 1;
    if c < cuts[2] {
        return Ok(x | (y << 8) | (u32::from(c - cuts[1]) << 16));
    }
    let z = u32::from(take(far, 1)?[0]);
    Ok(x | (y << 8) | (z << 16) | (u32::from(c - cuts[2]) << 24))
}
#[cfg(target_arch = "aarch64")]
#[inline(always)]
pub(super) unsafe fn sixteen(
    c: *const u8,
    l: *const u8,
    m: *const u8,
    far: &[u8],
    d: *mut u32,
    cuts: [u8; 3],
) -> Result<(usize, usize, usize)> {
    use std::arch::aarch64::*;
    unsafe {
        let z = vdupq_n_u8(0);
        let c = vld1q_u8(c);
        let m1 = vcgeq_u8(c, vdupq_n_u8(cuts[0]));
        let m2 = vcgeq_u8(c, vdupq_n_u8(cuts[1]));
        let m3 = vcgeq_u8(c, vdupq_n_u8(cuts[2]));
        macro_rules! scan {
            ($m:expr) => {{
                let mut v = vshrq_n_u8::<7>($m);
                v = vaddq_u8(v, vextq_u8::<15>(z, v));
                v = vaddq_u8(v, vextq_u8::<14>(z, v));
                v = vaddq_u8(v, vextq_u8::<12>(z, v));
                vaddq_u8(v, vextq_u8::<8>(z, v))
            }};
        }
        let a = scan!(m1);
        let b = scan!(m2);
        let one = vdupq_n_u8(1);
        let lo = vbslq_u8(m1, vqtbl1q_u8(vld1q_u8(l), vsubq_u8(a, one)), c);
        let mi = vbslq_u8(
            m2,
            vqtbl1q_u8(vld1q_u8(m), vsubq_u8(b, one)),
            vandq_u8(m1, vsubq_u8(c, vdupq_n_u8(cuts[0]))),
        );
        let hi = vandq_u8(m2, vsubq_u8(c, vdupq_n_u8(cuts[1])));
        let mut extra = 0;
        if vmaxvq_u8(m3) == 0 {
            vst4q_u8(d.cast(), uint8x16x4_t(lo, mi, hi, z));
        } else {
            if far.len() < 16 {
                return Err(bad());
            }
            let ix = scan!(m3);
            extra = vgetq_lane_u8::<15>(ix) as usize;
            let hi = vbslq_u8(
                m3,
                vqtbl1q_u8(vld1q_u8(far.as_ptr()), vsubq_u8(ix, one)),
                hi,
            );
            let top = vandq_u8(m3, vsubq_u8(c, vdupq_n_u8(cuts[2])));
            vst4q_u8(d.cast(), uint8x16x4_t(lo, mi, hi, top));
        }
        Ok((
            vgetq_lane_u8::<15>(a) as usize,
            vgetq_lane_u8::<15>(b) as usize,
            extra,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn checked_lanes_restore_short_repeated_and_four_byte_offsets() {
        const BASE: usize = 10 << 20;
        let mut raw: Vec<_> = (0..BASE + 1024)
            .map(|v| ((v.wrapping_mul(7919) ^ (v >> 7)) % 251) as u8)
            .collect();
        let mut st = Streams::new();
        let mut anchor = 0;
        for j in 0..53 {
            let at = BASE + 16 * j;
            let d = [
                1usize, 4, 7, 43, 65535, 219_019, 6_400_019, 9_500_019, 4, 291,
            ][j % 10];
            for k in 0..9 {
                raw[at + k] = raw[at - d + k];
            }
            st.emit(&raw, anchor, at, 9, d);
            anchor = at + 9;
        }
        st.lits.extend_from_slice(&raw[anchor..]);
        let mut dst = vec![0xa5; raw.len() + 32];
        for shape in [0, 2, 6, 7] {
            let wire = put::<1>(&st, shape, true, false, None);
            crate::decode(&wire, &mut dst[16..16 + raw.len()]).unwrap();
            assert_eq!(dst[16..16 + raw.len()], raw);
            assert!(
                dst[..16]
                    .iter()
                    .chain(&dst[16 + raw.len()..])
                    .all(|&v| v == 0xa5)
            );
            assert!(crate::decode(&wire[..wire.len() - 1], &mut dst[16..16 + raw.len()]).is_err());
            let mut bad = wire;
            bad[1] = 8;
            assert!(crate::decode(&bad, &mut dst[16..16 + raw.len()]).is_err());
        }
    }
}
