// Estimate a spatial stride from the input; the decoder receives it in full.
// For an available reference u, x = floor((last + u)/2) + signed_error.
// Eight such steps compose exactly by weighting the errors by powers of two.
// Wrapped errors use the scalar inverse if the provisional lanes leave u16.
use crate::{Result, ans, bad, take};
pub(crate) struct Plan {
    pub(crate) kib: usize,
    mode: usize,
    effort: usize,
    reach: usize,
}
pub(crate) fn plan(level: u8) -> Option<Plan> {
    // Short jobs parallelize the row recurrence. Larger jobs amortize row seeds.
    let (kib, effort, reach) = match level {
        2 => (512, 1, 2048),
        3..=4 => (1024, 2, 4096),
        5..=11 => (2048, 1, 2048),
        12..=22 => (2048, 3, 8192),
        _ => return None,
    };
    Some(Plan {
        kib,
        mode: 2,
        effort,
        reach,
    })
}
#[inline(always)]
fn at(s: &[u8], j: usize) -> u16 {
    u16::from_le_bytes([s[2 * j], s[2 * j + 1]])
}
#[inline(always)]
fn avg(a: u16, b: u16) -> u16 {
    ((u32::from(a) + u32::from(b)) >> 1) as u16
}
#[inline(always)]
fn above(s: &[u8], j: usize, smooth: bool) -> u16 {
    let a = at(s, j);
    if smooth {
        avg(a, avg(at(s, j - 1), at(s, j + 1)))
    } else {
        a
    }
}
fn anchors<const N: usize>(n: usize, maxd: usize, mut rng: u32) -> [usize; N] {
    std::array::from_fn(|_| {
        rng = rng.wrapping_mul(1664525).wrapping_add(1013904223);
        maxd + 2 + ((u64::from(rng) * (n - maxd - 3) as u64) >> 32) as usize
    })
}
fn discover(s: &[u8], maxd: usize, effort: usize) -> Vec<usize> {
    let n = s.len() / 2;
    let maxd = maxd.min(n / 8);
    if maxd < 32 {
        return vec![];
    }
    let probe = anchors::<48>(n, maxd, 19088743);
    let hold = anchors::<192>(n, maxd, 21749813);
    let coarse = if effort == 1 {
        12
    } else if effort == 2 {
        24
    } else {
        48
    };
    let mut best: Vec<(u32, usize)> = (32..=maxd)
        .map(|d| {
            let mut price = 0;
            for &i in &probe[..coarse] {
                price += u32::from(at(s, i).abs_diff(at(s, i - d))).min(256)
            }
            (price, d)
        })
        .collect();
    let count = if effort == 1 {
        4
    } else if effort == 2 {
        8
    } else {
        16
    };
    if best.len() > count {
        best.select_nth_unstable(count);
        best.truncate(count)
    }
    for h in &mut best {
        let mut score = 0;
        for &i in &hold {
            score +=
                u32::from(at(s, i).abs_diff(avg(at(s, i - 1), above(s, i - h.1, true)))).min(512)
        }
        h.0 = score;
    }
    best.sort_unstable();
    best.into_iter()
        .map(|v| v.1)
        .take(if effort < 3 { 1 } else { 2 })
        .collect()
}
#[cfg(target_arch = "aarch64")]
#[inline(always)]
unsafe fn above8(ptr: *const u16, smooth: bool) -> std::arch::aarch64::uint16x8_t {
    use std::arch::aarch64::*;
    unsafe {
        let u = vld1q_u16(ptr);
        if smooth {
            vhaddq_u16(u, vhaddq_u16(vld1q_u16(ptr.sub(1)), vld1q_u16(ptr.add(1))))
        } else {
            u
        }
    }
}
#[inline(always)]
fn residual16(s: &[u8], i: usize, d: usize, smooth: bool, lo: &mut [u8], hi: &mut [u8; 16]) {
    debug_assert!(i > d && d >= 32 && 2 * (i + 16) <= s.len() && lo.len() >= 16);
    #[cfg(target_arch = "aarch64")]
    unsafe {
        use std::arch::aarch64::*;
        let ptr = s.as_ptr().cast::<u16>().add(i);
        let one = |p: *const u16| {
            let u = above8(p.sub(d), smooth);
            let guess = vhaddq_u16(vld1q_u16(p.sub(1)), u);
            vreinterpretq_u8_u16(vaddq_u16(vsubq_u16(vld1q_u16(p), guess), vdupq_n_u16(128)))
        };
        let a = one(ptr);
        let b = one(ptr.add(8));
        vst1q_u8(lo.as_mut_ptr(), vuzp1q_u8(a, b));
        vst1q_u8(hi.as_mut_ptr(), vuzp2q_u8(a, b));
    }
    #[cfg(not(target_arch = "aarch64"))]
    for t in 0..16 {
        let j = i + t;
        let x = at(s, j)
            .wrapping_sub(avg(at(s, j - 1), above(s, j - d, smooth)))
            .wrapping_add(128);
        lo[t] = x as u8;
        hi[t] = (x >> 8) as u8;
    }
}
fn encode_d(s: &[u8], d: usize, smooth: bool, coded: bool) -> Vec<u8> {
    let first = d + 1;
    let g = (s.len() / 2 - first) / 16;
    let mut lo = vec![0; g * 16];
    let mut flags = vec![0u8; g.div_ceil(8)];
    let mut extra = Vec::new();
    for j in 0..g {
        let mut hi = [0u8; 16];
        residual16(
            s,
            first + 16 * j,
            d,
            smooth,
            &mut lo[16 * j..16 * j + 16],
            &mut hi,
        );
        if hi != [0; 16] {
            flags[j / 8] |= 1 << (j % 8);
            let k = extra.len();
            extra.extend([0; 2]);
            let mut m = 0u16;
            for (t, v) in hi.into_iter().enumerate() {
                if v != 0 {
                    m |= 1 << t;
                    extra.push(v)
                }
            }
            extra[k..k + 2].copy_from_slice(&m.to_le_bytes());
        }
    }
    let mut v = vec![63, u8::from(smooth) | (u8::from(coded) << 1)];
    v.extend((d as u16).to_le_bytes());
    v.extend_from_slice(&s[..2 * first]);
    if coded {
        ans::put_config(&lo, &mut v, 0.985, 0, (2, 1))
    } else {
        v.extend(lo)
    }
    v.extend(flags);
    v.extend(extra);
    v.extend_from_slice(&s[2 * (first + 16 * g)..]);
    v
}

pub(crate) fn encode_plan(s: &[u8], level: u8, plan: Plan) -> Vec<u8> {
    let mut best = crate::pixel::encode_level(s, level);
    if s.len() < 64 << 10 {
        return best;
    }
    for d in discover(s, plan.reach, plan.effort) {
        for smooth in [false, true] {
            if plan.mode == 1 && smooth || plan.mode == 2 && !smooth {
                continue;
            }
            let got = encode_d(s, d, smooth, plan.mode != 4);
            if got.len() < best.len() {
                best = got
            }
        }
    }
    best
}
#[inline(always)]
fn scalar(s: &mut [u8], i: usize, d: usize, smooth: bool, errors: &[i16], mut last: u16) -> u16 {
    for (j, &e) in errors.iter().enumerate() {
        last = avg(last, above(s, i + j - d, smooth)).wrapping_add_signed(e);
        s[2 * (i + j)..2 * (i + j) + 2].copy_from_slice(&last.to_le_bytes())
    }
    last
}
#[cfg(target_arch = "aarch64")]
#[inline(always)]
unsafe fn compose8(s: &mut [u8], i: usize, d: usize, smooth: bool, e: &[i16], last: u16) -> u16 {
    use std::arch::aarch64::*;
    // d>=32 leaves at least sixteen samples between this destination and
    // any smoothed source. Both eight-lane loads and the final store fit.
    unsafe {
        let dst = s.as_mut_ptr().cast::<u16>().add(i);
        let u = above8(dst.sub(d), smooth);
        let errs = vld1q_s16(e.as_ptr());
        let zero = vdupq_n_s32(0);
        let pow = vld1q_s32([0, 1, 2, 3].as_ptr());
        let fold = |a: uint16x4_t, b: int16x4_t| {
            let v = vaddq_s32(
                vreinterpretq_s32_u32(vmovl_u16(a)),
                vshlq_n_s32::<1>(vmovl_s16(b)),
            );
            let x = vshlq_s32(v, pow);
            let x = vaddq_s32(x, vextq_s32::<3>(zero, x));
            vaddq_s32(x, vextq_s32::<2>(zero, x))
        };
        let a = fold(vget_low_u16(u), vget_low_s16(errs));
        let b = fold(vget_high_u16(u), vget_high_s16(errs));
        let a = vaddq_s32(a, vdupq_n_s32(i32::from(last)));
        let b = vaddq_s32(vshlq_n_s32::<4>(b), vdupq_laneq_s32::<3>(a));
        let x = vshlq_s32(a, vld1q_s32([-1, -2, -3, -4].as_ptr()));
        let y = vshlq_s32(b, vld1q_s32([-5, -6, -7, -8].as_ptr()));
        let invalid = vmaxvq_u32(vorrq_u32(
            vreinterpretq_u32_s32(x),
            vreinterpretq_u32_s32(y),
        )) > 65535;
        if invalid {
            return scalar(s, i, d, smooth, &e[..8], last);
        }
        let samples = vcombine_u16(
            vmovn_u32(vreinterpretq_u32_s32(x)),
            vmovn_u32(vreinterpretq_u32_s32(y)),
        );
        vst1q_u16(dst, samples);
        vgetq_lane_u16::<7>(samples)
    }
}
pub(crate) fn get(mut p: &[u8], s: &mut [u8]) -> Result<()> {
    let mode = take(&mut p, 1)?[0];
    let d = u16::from_le_bytes(take(&mut p, 2)?.try_into().unwrap()) as usize;
    let first = d + 1;
    if mode > 3 || !(32..=8192).contains(&d) || first > s.len() / 2 {
        return Err(bad());
    }
    let seed = take(&mut p, first * 2)?;
    s[..2 * first].copy_from_slice(seed);
    let g = (s.len() / 2 - first) / 16;
    let n = g * 16;
    let store;
    let low = if mode & 2 != 0 {
        store = ans::get(&mut p, n)?;
        if store.len() != n {
            return Err(bad());
        }
        &*store
    } else {
        take(&mut p, n)?
    };
    let flags = take(&mut p, g.div_ceil(8))?;
    let mut last = at(seed, d);
    let smooth = mode & 1 != 0;
    for j in 0..g {
        let i = first + j * 16;
        let mut hi = [0; 16];
        if flags[j / 8] & (1 << (j % 8)) != 0 {
            let mut m = u16::from_le_bytes(take(&mut p, 2)?.try_into().unwrap());
            for &v in take(&mut p, m.count_ones() as usize)? {
                hi[m.trailing_zeros() as usize] = v;
                m &= m - 1
            }
        }
        let err: [i16; 16] = std::array::from_fn(|t| {
            u16::from_le_bytes([low[j * 16 + t], hi[t]]).wrapping_sub(128) as i16
        });
        #[cfg(target_arch = "aarch64")]
        unsafe {
            last = compose8(s, i, d, smooth, &err, last);
            last = compose8(s, i + 8, d, smooth, &err[8..], last)
        }
        #[cfg(not(target_arch = "aarch64"))]
        {
            last = scalar(s, i, d, smooth, &err, last);
        }
    }
    let end = 2 * (first + 16 * g);
    if p.len() != s.len() - end {
        return Err(bad());
    }
    s[end..].copy_from_slice(p);
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn floor_composition_and_wrapped_residuals_restore_edges_and_tails() {
        let mut src = vec![];
        let mut r = 6972u32;
        for i in 0..33000 {
            r = r.wrapping_mul(1664525).wrapping_add(1013904223);
            let v = match i % 503 {
                0 => 65535,
                1 => 0,
                _ => {
                    if i % 113 < 85 {
                        (21000 + (i / 63) % 300 + (r >> 28)) as u16
                    } else {
                        (r >> 16) as u16
                    }
                }
            };
            src.extend(v.to_le_bytes())
        }
        for d in [32, 61, 322] {
            for end in [2 * d + 2, 2 * d + 33, 2 * d + 69, src.len() - 1, src.len()] {
                for (smooth, coded) in [(true, true), (false, false)] {
                    let got = encode_d(&src[..end], d, smooth, coded);
                    let mut out = vec![0; end];
                    get(&got[1..], &mut out).unwrap();
                    assert_eq!(out, src[..end]);
                    assert!(get(&got[1..got.len() - 1], &mut out).is_err())
                }
            }
        }
    }
}
