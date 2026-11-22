// A fixed-width selector chooses a power-of-two subalphabet. The selectors
// and payload bits live in separate streams, so 16 lookups can run in parallel.
use crate::{Result, bad, take, u32in};
// Try a width for four starting ranks in parallel. Padded suffixes have
// constant cumulative counts and zero remaining cost, including a partial
// last group. Strict comparison preserves the scalar tie break.
#[cfg(target_arch = "aarch64")]
fn plan_row(
    dp: &[u32; 512],
    next: &mut [u32; 512],
    ps: &[u32; 512],
    choices: &mut [u8; 256],
    n: usize,
) {
    use std::arch::aarch64::*;
    unsafe {
        for i in (0..n).step_by(4) {
            let p = ps.as_ptr().add(i);
            let d = dp.as_ptr().add(i);
            let start = vld1q_u32(p);
            let mut best = vld1q_u32(d.add(1));
            let mut width = vdupq_n_u32(0);
            macro_rules! attempt {
                ($w:literal) => {{
                    let end = 1 << $w;
                    let count = vsubq_u32(vld1q_u32(p.add(end)), start);
                    let cost = vmlaq_n_u32(vld1q_u32(d.add(end)), count, $w);
                    width = vbslq_u32(vcltq_u32(cost, best), vdupq_n_u32($w), width);
                    best = vminq_u32(best, cost);
                }};
            }
            attempt!(1);
            attempt!(2);
            attempt!(3);
            attempt!(4);
            attempt!(5);
            attempt!(6);
            attempt!(7);
            attempt!(8);
            vst1q_u32(next.as_mut_ptr().add(i), best);
            let w = vmovn_u16(vcombine_u16(vmovn_u32(width), vdup_n_u16(0)));
            choices[i..i + 4]
                .copy_from_slice(&vget_lane_u32::<0>(vreinterpret_u32_u8(w)).to_le_bytes());
        }
    }
}
#[cfg(not(target_arch = "aarch64"))]
fn plan_row(
    dp: &[u32; 512],
    next: &mut [u32; 512],
    ps: &[u32; 512],
    choices: &mut [u8; 256],
    n: usize,
) {
    for i in 0..n {
        let mut best = dp[i + 1];
        let mut width = 0;
        for w in 1..=8 {
            let end = i + (1 << w);
            let cost = dp[end] + (ps[end] - ps[i]) * w;
            if cost < best {
                best = cost;
                width = w;
            }
        }
        next[i] = best;
        choices[i] = width as u8;
    }
}
#[derive(Clone, Copy, Default)]
#[repr(C, align(4))]
struct Code {
    head: u8,
    rank: u8,
    width: u8,
}

// Four variable-width ranks fit in a 32-bit word. Only the carry at the
// partial byte crosses groups. The fixed-width selectors have independent
// bit positions and eight of them always end on a byte boundary.
fn pack<const K: usize>(data: &[u8], map: &[Code; 256], heads: &mut [u8], tail: &mut [u8]) {
    pack_scalar::<K>(data, map, heads, tail)
}
fn pack_scalar<const K: usize>(data: &[u8], map: &[Code; 256], heads: &mut [u8], tail: &mut [u8]) {
    let mut bit = 0usize;
    let mut carry = 0u64;
    let mut head_at = 0;
    let mut chunks = data.chunks_exact(8);
    for group in &mut chunks {
        let mut heads8 = 0u64;
        for j in 0..2 {
            let i = 4 * j;
            let a = map[group[i] as usize];
            let b = map[group[i + 1] as usize];
            let c = map[group[i + 2] as usize];
            let d = map[group[i + 3] as usize];
            let h = a.head as u64
                | ((b.head as u64) << K)
                | ((c.head as u64) << (2 * K))
                | ((d.head as u64) << (3 * K));
            heads8 |= h << (j * 4 * K);
            let ab = a.rank as u64 | ((b.rank as u64) << a.width);
            let cd = c.rank as u64 | ((d.rank as u64) << c.width);
            let aw = a.width + b.width;
            let new_bits = aw + c.width + d.width;
            let v = carry | ((ab | (cd << aw)) << (bit & 7));
            // The format includes 17 zero padding bytes after the tail.
            tail[bit / 8..bit / 8 + 8].copy_from_slice(&v.to_le_bytes());
            carry = v >> (((bit & 7) + new_bits as usize) & !7);
            bit += new_bits as usize;
        }
        heads[head_at..head_at + 8].copy_from_slice(&heads8.to_le_bytes());
        head_at += K;
    }
    let mut h = 0u64;
    for (j, &s) in chunks.remainder().iter().enumerate() {
        let a = map[s as usize];
        h |= (a.head as u64) << (j * K);
        let v = carry | ((a.rank as u64) << (bit & 7));
        tail[bit / 8..bit / 8 + 2].copy_from_slice(&(v as u16).to_le_bytes());
        carry = v >> (((bit & 7) + a.width as usize) & !7);
        bit += a.width as usize;
    }
    heads[head_at..head_at + 8].copy_from_slice(&h.to_le_bytes());
    debug_assert_eq!(tail.len(), bit.div_ceil(8) + 17);
}

pub fn put(data: &[u8], counts: &[u32; 256], out: &mut Vec<u8>, ideal: f64, plan: usize) -> bool {
    let mut syms: Vec<u8> = (0..=255).filter(|&i| counts[i as usize] > 0).collect();
    syms.sort_unstable_by_key(|&i| std::cmp::Reverse(counts[i as usize]));
    let n = syms.len();
    let mut ps = [data.len() as u32; 512];
    ps[0] = 0;
    for (i, &sym) in syms.iter().enumerate() {
        ps[i + 1] = ps[i] + counts[sym as usize]
    }
    {
        let policy = plan;
        if policy != 0 {
            let (best, widths) = rankgroups::plan(&ps, n, data.len(), policy);
            if best < data.len() * 8 && (best as f64) <= ideal * 1.11 + 1024. {
                return write_groups(data, out, &syms, &ps, widths);
            }
            if policy != 3 {
                return false;
            }
        }
    }
    let mut dp = [0; 512];
    dp[..n].fill(0x1fff_ffff);
    let mut next = [0; 512];
    let mut choice = [[0u8; 256]; 33];
    let (mut best, mut k) = (u32::MAX, 0usize);
    for (g, decisions) in choice.iter_mut().enumerate().skip(1) {
        plan_row(&dp, &mut next, &ps, decisions, n);
        std::mem::swap(&mut dp, &mut next);
        if g >= 2 && g.is_power_of_two() {
            let log = g.ilog2();
            let bits = dp[0] + log * data.len() as u32 + 8 * (g + n + 40) as u32;
            if bits < best {
                best = bits;
                k = log as usize
            }
            // Larger selector widths alone would cost at least this much.
            if best <= (log + 1) * data.len() as u32 + 8 * (2 * g + n + 40) as u32 || g >= n {
                break;
            }
        }
    }
    if best as usize >= data.len() * 8 || best as f64 > ideal * 1.11 + 1024. {
        return false;
    }
    let groups = 1 << k;
    let mut widths = vec![0; groups];
    let mut at = 0;
    for (h, w) in widths.iter_mut().enumerate() {
        if at >= n {
            break;
        }
        *w = choice[groups - h][at];
        at = (at + (1 << *w)).min(n);
    }
    write_groups(data, out, &syms, &ps, widths)
}
#[cfg(target_arch = "aarch64")]
pub(crate) mod fixedpalette;

mod rankgroups;
fn write_groups(
    data: &[u8],
    out: &mut Vec<u8>,
    syms: &[u8],
    ps: &[u32; 512],
    widths: Vec<u8>,
) -> bool {
    let n = syms.len();
    let k = widths.len().ilog2() as usize;
    let mut at = 0;
    let mut map = [Code::default(); 256];
    let mut nb = 0u32;
    for (h, w) in widths.iter().enumerate() {
        if at >= n {
            break;
        }
        let end = (at + (1 << *w)).min(n);
        for (rank, &s) in syms[at..end].iter().enumerate() {
            map[s as usize] = Code {
                head: h as u8,
                rank: rank as u8,
                width: *w,
            }
        }
        nb += (ps[end] - ps[at]) * *w as u32;
        at = end;
    }

    out.push(5);
    out.push(k as u8);
    out.push((n - 1) as u8);
    out.extend(&widths);
    out.extend_from_slice(syms);
    let start = out.len();
    let head_len = (data.len() * k).div_ceil(8) + 16;
    out.resize(start + head_len + 4 + (nb as usize).div_ceil(8) + 17, 0);
    let (heads, rest) = out[start..].split_at_mut(head_len);
    rest[..4].copy_from_slice(&nb.to_le_bytes());
    let tail = &mut rest[4..];
    match k {
        1 => pack::<1>(data, &map, heads, tail),
        2 => pack::<2>(data, &map, heads, tail),
        3 => pack::<3>(data, &map, heads, tail),
        4 => pack::<4>(data, &map, heads, tail),
        _ => pack::<5>(data, &map, heads, tail),
    }
    true
}
#[cfg(target_arch = "aarch64")]
unsafe fn vector<const K: usize>(
    heads: &[u8],
    tail: &[u8],
    nb: usize,
    ww: &[u8; 32],
    bb: &[u8; 32],
    sy: &[u8; 256],
    out: &mut [u8],
) -> (usize, usize) {
    use std::arch::aarch64::*;
    unsafe {
        let z = vdupq_n_u8(0);
        let mask = vdupq_n_u8((1 << K) - 1);
        let hi: [u8; 16] = std::array::from_fn(|j| (j * K / 8) as u8);
        let hs: [i8; 16] = std::array::from_fn(|j| -((j * K % 8) as i8));
        let hi = vld1q_u8(hi.as_ptr());
        let hs = vld1q_s8(hs.as_ptr());
        let wtab = uint8x16x2_t(vld1q_u8(ww.as_ptr()), vld1q_u8(ww.as_ptr().add(16)));
        let btab = uint8x16x2_t(vld1q_u8(bb.as_ptr()), vld1q_u8(bb.as_ptr().add(16)));
        let t0 = vld1q_u8_x4(sy.as_ptr());
        let t1 = vld1q_u8_x4(sy.as_ptr().add(64));
        let t2 = vld1q_u8_x4(sy.as_ptr().add(128));
        let t3 = vld1q_u8_x4(sy.as_ptr().add(192));
        let mut bit = 0;
        let mut at = 0;
        while at + 16 <= out.len() && bit <= nb {
            let hraw = vld1q_u8(heads.as_ptr().add(at * K / 8));
            let mut h = vshlq_u8(vqtbl1q_u8(hraw, hi), hs);
            if K == 3 || K == 5 {
                h = vorrq_u8(
                    h,
                    vshlq_u8(
                        vqtbl1q_u8(hraw, vaddq_u8(hi, vdupq_n_u8(1))),
                        vaddq_s8(hs, vdupq_n_s8(8)),
                    ),
                )
            }
            let h = vandq_u8(h, mask);
            let w = if K == 5 {
                vqtbl2q_u8(wtab, h)
            } else {
                vqtbl1q_u8(wtab.0, h)
            };
            let base = if K == 5 {
                vqtbl2q_u8(btab, h)
            } else {
                vqtbl1q_u8(btab.0, h)
            };
            let mut scan = w;
            scan = vaddq_u8(scan, vextq_u8::<15>(z, scan));
            scan = vaddq_u8(scan, vextq_u8::<14>(z, scan));
            scan = vaddq_u8(scan, vextq_u8::<12>(z, scan));
            scan = vaddq_u8(scan, vextq_u8::<8>(z, scan));
            let index = vaddq_u8(vsubq_u8(scan, w), vdupq_n_u8((bit & 7) as u8));
            let p = tail.as_ptr().add(bit / 8);
            bit += vgetq_lane_u8::<15>(scan) as usize;
            let off = vshrq_n_u8::<3>(index);
            let shift = vreinterpretq_s8_u8(vandq_u8(index, vdupq_n_u8(7)));
            let low = vshlq_u8(vqtbl1q_u8(vld1q_u8(p), off), vnegq_s8(shift));
            let high = vshlq_u8(
                vqtbl1q_u8(vld1q_u8(p.add(1)), off),
                vsubq_s8(vdupq_n_s8(8), shift),
            );
            let field = vmvnq_u8(vshlq_u8(vdupq_n_u8(255), vreinterpretq_s8_u8(w)));
            let rank = vaddq_u8(base, vandq_u8(field, vorrq_u8(low, high)));
            let mut sym = vqtbl4q_u8(t0, rank);
            let r = vsubq_u8(rank, vdupq_n_u8(64));
            sym = vqtbx4q_u8(sym, t1, r);
            let r = vsubq_u8(r, vdupq_n_u8(64));
            sym = vqtbx4q_u8(sym, t2, r);
            let r = vsubq_u8(r, vdupq_n_u8(64));
            sym = vqtbx4q_u8(sym, t3, r);
            vst1q_u8(out.as_mut_ptr().add(at), sym);
            at += 16;
        }
        (at, bit)
    }
}
pub fn get(p: &mut &[u8], n: usize) -> Result<Vec<u8>> {
    let k = take(p, 1)?[0] as usize;
    if !(1..=5).contains(&k) {
        return Err(bad());
    }
    let ns = take(p, 1)?[0] as usize + 1;
    let mut ww = [0; 32];
    let mut bb = [0; 32];
    let mut base = 0usize;
    for (i, &w) in take(p, 1 << k)?.iter().enumerate() {
        if w > 8 {
            return Err(bad());
        }
        ww[i] = w;
        bb[i] = base.min(255) as u8;
        base += 1 << w
    }
    if base < ns {
        return Err(bad());
    }
    let mut sy = [0u8; 256];
    sy[..ns].copy_from_slice(take(p, ns)?);
    let heads = take(p, (n * k).div_ceil(8) + 16)?;
    let nb = u32in(p)?;
    if nb > n * 8 {
        return Err(bad());
    }
    let tail = take(p, nb.div_ceil(8) + 17)?;
    let mut out = vec![0; n];
    #[cfg(target_arch = "aarch64")]
    let (at, mut bit) = unsafe {
        match k {
            1 => vector::<1>(heads, tail, nb, &ww, &bb, &sy, &mut out),
            2 => vector::<2>(heads, tail, nb, &ww, &bb, &sy, &mut out),
            3 => vector::<3>(heads, tail, nb, &ww, &bb, &sy, &mut out),
            4 => vector::<4>(heads, tail, nb, &ww, &bb, &sy, &mut out),
            _ => vector::<5>(heads, tail, nb, &ww, &bb, &sy, &mut out),
        }
    };
    #[cfg(not(target_arch = "aarch64"))]
    let (at, mut bit) = (0, 0);
    for (j, dest) in out.iter_mut().enumerate().skip(at) {
        if bit > nb {
            return Err(bad());
        }
        let pos = j * k;
        let h = ((u16::from_le_bytes(heads[pos / 8..pos / 8 + 2].try_into().unwrap()) >> (pos & 7))
            & ((1 << k) - 1)) as usize;
        let w = ww[h];
        let u = u16::from_le_bytes(tail[bit / 8..bit / 8 + 2].try_into().unwrap());
        let rank = bb[h].wrapping_add(((u >> (bit & 7)) & ((1 << w) - 1)) as u8);
        bit += w as usize;
        *dest = sy[rank as usize]
    }
    if bit != nb {
        return Err(bad());
    }
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn varied_distributions_and_tails() {
        for &n in &[1536, 1543, 7231, 65537] {
            let mut exercised = 0;
            for skew in [2, 5, 17, 75] {
                let mut seed = 55u32;
                let mut ct = [0; 256];
                let v: Vec<u8> = (0..n)
                    .map(|_| {
                        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                        let u = (seed >> 24) as u8;
                        let b = if !seed.is_multiple_of(5) { u % skew } else { u };
                        ct[b as usize] += 1;
                        b
                    })
                    .collect();
                let mut stream = vec![];
                if !put(&v, &ct, &mut stream, n as f64 * 8., 0) {
                    continue;
                }
                exercised += 1;
                let mut p = &stream[1..];
                assert_eq!(get(&mut p, n).unwrap(), v);
                assert!(p.is_empty());
            }
            assert!(exercised >= 2);
        }
    }
}
