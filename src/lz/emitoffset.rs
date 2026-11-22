// Each offset code depends on the previous three absolute offsets.
use super::{Streams, lenout, quicktwo};
use std::arch::aarch64::*;

const fn order() -> [[u8; 8]; 256] {
    let mut rows = [[255; 8]; 256];
    let mut mask = 0;
    while mask < 256 {
        let (mut i, mut k) = (0, 0);
        while i < 8 {
            if (mask >> i) & 1 != 0 {
                rows[mask][k] = i as u8;
                k += 1;
            }
            i += 1;
        }
        mask += 1;
    }
    rows
}
static ORDER: [[u8; 8]; 256] = order();
const WEIGHTS: [u8; 8] = [1, 2, 4, 8, 16, 32, 64, 128];

// Eight writable spare bytes allow a full store even for an empty mask.
#[inline(always)]
unsafe fn compact(dst: &mut Vec<u8>, data: uint8x8_t, keep: uint8x8_t) {
    unsafe {
        let mask = vaddv_u8(vand_u8(keep, vld1_u8(WEIGHTS.as_ptr()))) as usize;
        let selected = vtbl1_u8(data, vld1_u8(ORDER[mask].as_ptr()));
        let k = dst.len();
        debug_assert!(dst.capacity() - k >= 8);
        vst1_u8(dst.as_mut_ptr().add(k), selected);
        dst.set_len(k + mask.count_ones() as usize);
    }
}

#[inline(always)]
unsafe fn classify(p: *const u32) -> (uint32x4_t, uint32x4_t) {
    unsafe {
        let a = vld1q_u32(p);
        let b = vld1q_u32(p.add(1));
        let c = vld1q_u32(p.add(2));
        let off = vld1q_u32(p.add(3));
        let v = vaddq_u32(off, vdupq_n_u32(19));
        let delta = vsubq_u32(vaddq_u32(off, vdupq_n_u32(8)), c);
        let mut code = vbslq_u32(
            vcltq_u32(v, vdupq_n_u32(32768)),
            vaddq_u32(vshrq_n_u32::<8>(v), vdupq_n_u32(84)),
            vaddq_u32(vshrq_n_u32::<16>(v), vdupq_n_u32(212)),
        );
        code = vbslq_u32(vcleq_u32(off, vdupq_n_u32(64)), v, code);
        code = vbslq_u32(
            vcleq_u32(delta, vdupq_n_u32(16)),
            vaddq_u32(delta, vdupq_n_u32(3)),
            code,
        );
        code = vbslq_u32(vceqq_u32(off, a), vdupq_n_u32(2), code);
        code = vbslq_u32(vceqq_u32(off, b), vdupq_n_u32(1), code);
        (vbicq_u32(code, vceqq_u32(off, c)), v)
    }
}

// Initial three entries contain the previous offsets, oldest first. Only
// groups of eight reach this function; each four-lane load stays in history.
#[inline(always)]
fn eight(st: &mut Streams, history: &[u32]) {
    debug_assert!(history.len() >= 11);
    unsafe {
        let (ca, va) = classify(history.as_ptr());
        let (cb, vb) = classify(history.as_ptr().add(4));
        let codes = vmovn_u16(vcombine_u16(vmovn_u32(ca), vmovn_u32(cb)));
        let values = vcombine_u16(vmovn_u32(va), vmovn_u32(vb));
        let k = st.offs.len();
        debug_assert!(st.offs.capacity() - k >= 8);
        vst1_u8(st.offs.as_mut_ptr().add(k), codes);
        st.offs.set_len(k + 8);
        compact(
            &mut st.low,
            vmovn_u16(values),
            vcge_u8(codes, vdup_n_u8(84)),
        );
        compact(
            &mut st.mid,
            vshrn_n_u16::<8>(values),
            vcge_u8(codes, vdup_n_u8(212)),
        );
    }
}

// All grouped parsers probe an eight-byte source word before a match.
// A backward extension only lowers i, so even a zero literal run has eight
// readable bytes at its anchor. Only initialized literal bytes are exposed.
#[inline(always)]
fn short_command<const M: usize>(st: &mut Streams, s: &[u8], a: usize, i: usize, ml: usize) {
    let ll = i - a;
    let at = st.lits.len();
    debug_assert!(a + 8 <= s.len() && at + ll.max(8) <= st.lits.capacity());
    unsafe {
        let src = s.as_ptr().add(a);
        let dst = st.lits.as_mut_ptr().add(at);
        std::ptr::copy_nonoverlapping(src, dst, 8);
        if M == 2 {
            let shift = ll.saturating_sub(8);
            std::ptr::copy_nonoverlapping(src.add(shift), dst.add(shift), 8);
            if ll > 16 {
                std::ptr::copy_nonoverlapping(src.add(8), dst.add(8), ll - 16);
            }
        } else if ll > 8 {
            std::ptr::copy_nonoverlapping(src.add(8), dst.add(8), ll - 8);
        }
        st.lits.set_len(at + ll);
    }
    if M == 3 || ll.max(ml) >= 143 {
        if ll >= 15 {
            lenout(ll - 15, &mut st.lens);
        }
        if ml >= 15 {
            lenout(ml - 15, &mut st.lens);
        }
    } else {
        let l = usize::from(ll >= 15);
        let m = usize::from(ml >= 15);
        let at = st.lens.len();
        debug_assert!(st.lens.capacity() - at >= 2);
        // Only the first l+m bytes become visible. If neither is needed both
        // speculative stores land in a spare byte past the initialized data.
        unsafe {
            let p = st.lens.as_mut_ptr().add(at);
            p.write(ll.wrapping_sub(15) as u8);
            // When only the first length escapes, preserve that byte.
            p.add(l).write(ml.wrapping_sub(15) as u8);
            st.lens.set_len(at + l + m);
        }
    }
}
fn grouped<const N: usize, const M: usize>(
    s: &[u8],
    archive: bool,
    key: usize,
    entropy: (usize, usize),
) -> Vec<u8> {
    let (st, end) = parsed::<N, M>(s, key, entropy);
    st.finish(s, end, archive)
}

pub(super) fn parsed<const N: usize, const M: usize>(
    s: &[u8],
    key: usize,
    entropy: (usize, usize),
) -> (Streams, usize) {
    let mut st = Streams::new();
    let end = parsed_into::<N, M>(s, key, entropy, &mut st);
    (st, end)
}

#[inline(always)]
pub(super) fn parsed_into<const N: usize, const M: usize>(
    s: &[u8],
    key: usize,
    entropy: (usize, usize),
    st: &mut Streams,
) -> usize {
    assert!(s.len() <= crate::MAX_BLOCK && N >= 11 && (N - 3).is_multiple_of(8));
    st.entropy = entropy;
    // The native parser consumes >=4 distinct bytes per match. This bound
    // also covers the final speculative eight-byte store to each payload.
    let cap = s.len() / 4 + 8;
    st.offs.reserve_exact(cap);
    st.low.reserve_exact(cap);
    st.mid.reserve_exact(cap);
    st.tag.reserve_exact(cap);
    st.lits.reserve_exact(s.len() + 8);
    if M != 0 {
        st.lens.reserve_exact(s.len() / 8 + 8);
    }
    let mut queue = [0u32; N];
    queue[..3].copy_from_slice(&[8, 4, 1]);
    let mut used = 3;
    let end = quicktwo::walk_grouped(s, key, |a, i, n, off| {
        let ll = i - a;
        let ml = n - 4;
        st.tag.push(((ll.min(15) << 4) | ml.min(15)) as u8);
        if M != 0 {
            short_command::<M>(st, s, a, i, ml);
        } else {
            if ll >= 15 {
                lenout(ll - 15, &mut st.lens);
            }
            if ml >= 15 {
                lenout(ml - 15, &mut st.lens);
            }
            if ll != 0 {
                if ll <= 8 && s.len() - a >= 8 {
                    let k = st.lits.len();
                    // All literal runs together contain at most s.len() bytes.
                    unsafe {
                        std::ptr::copy_nonoverlapping(
                            s.as_ptr().add(a),
                            st.lits.as_mut_ptr().add(k),
                            8,
                        );
                        st.lits.set_len(k + ll);
                    }
                } else {
                    st.lits.extend_from_slice(&s[a..i]);
                }
            }
        }
        queue[used] = off as u32;
        used += 1;
        if used == N {
            for j in (0..N - 3).step_by(8) {
                eight(st, &queue[j..]);
            }
            queue.copy_within(N - 3..N, 0);
            used = 3;
        }
    });
    st.rep = [queue[2] as usize, queue[1] as usize, queue[0] as usize];
    for &off in &queue[3..used] {
        st.offset(off as usize);
    }
    end
}

pub(super) fn archive(s: &[u8]) -> Vec<u8> {
    grouped::<35, 0>(s, true, 10, (2, 0))
}
pub(super) fn text(s: &[u8]) -> Vec<u8> {
    grouped::<35, 0>(s, false, 5, (2, 1))
}
