// Look up two neighboring positions before deciding the first one. When the
// indices collide, the second lookup must see the just-inserted first slot.
// A successful first match discards the speculative second lookup.
use super::{Streams, probe, word};
#[inline(always)]
fn keys<const PREFIX: usize, const BITS: usize, const PLAIN: bool>(w: u64) -> (usize, u32) {
    (
        ((w << (64 - 8 * PREFIX)).wrapping_mul(0x9e3779b97f4a7c15) >> (64 - BITS)) as usize,
        if PLAIN {
            (w as u32) << 21
        } else {
            (w as u32).wrapping_mul(0x9e3779b1) << 21
        },
    )
}
pub(super) fn encode(s: &[u8], archive: bool) -> Vec<u8> {
    let mut st = Streams::sized(s.len());
    let end = walk(s, |a, i, n, o| st.emit(s, a, i, n, o));
    st.finish(s, end, archive)
}
#[inline(always)]
pub(super) fn walk(s: &[u8], emit: impl FnMut(usize, usize, usize, usize)) -> usize {
    walk_key::<5, 15, 2, false>(s, emit)
}
#[cfg(target_arch = "aarch64")]
#[inline(always)]
pub(super) fn walk_grouped(
    s: &[u8],
    key: usize,
    emit: impl FnMut(usize, usize, usize, usize),
) -> usize {
    if key == 5 || key == 10 {
        return super::pairhead::walk(s, key, emit);
    }
    match key {
        5 => walk_key::<6, 15, 2, true>(s, emit),
        10 => walk_key::<7, 14, 1, true>(s, emit),
        _ => walk(s, emit),
    }
}
#[inline(always)]
pub(super) fn walk_key<
    const PREFIX: usize,
    const BITS: usize,
    const TAIL: usize,
    const PLAIN: bool,
>(
    s: &[u8],
    mut emit: impl FnMut(usize, usize, usize, usize),
) -> usize {
    let mut table = vec![0u32; 1 << BITS];
    let (mut i, mut anchor) = (0, 0);
    while i + 8 <= s.len() {
        let w = word(s, i);
        let (k, sig) = keys::<PREFIX, BITS, PLAIN>(w);
        let here = sig | i as u32;
        let (mut len, mut q) = if i + 9 <= s.len() {
            let w1 = word(s, i + 1);
            let (k1, sig1) = keys::<PREFIX, BITS, PLAIN>(w1);
            let old = table[k];
            let old1 = if k == k1 { here } else { table[k1] };
            table[k] = here;
            let (a, b) = probe(s, i, w, old, sig);
            if a > 4 || (a == 4 && i - b <= 12000) {
                (a, b)
            } else {
                i += 1;
                table[k1] = sig1 | i as u32;
                probe(s, i, w1, old1, sig1)
            }
        } else {
            let old = std::mem::replace(&mut table[k], here);
            probe(s, i, w, old, sig)
        };
        if len < 4 || (len == 4 && i - q > 12000) {
            i += 1;
            continue;
        }
        let off = i - q;
        while i > anchor && q > 0 && s[i - 1] == s[q - 1] {
            i -= 1;
            q -= 1;
            len += 1;
        }
        emit(anchor, i, len, off);
        i += len;
        anchor = i;
        for j in i - TAIL..i {
            if j + 8 <= s.len() {
                let (k, sig) = keys::<PREFIX, BITS, PLAIN>(word(s, j));
                table[k] = sig | j as u32;
            }
        }
    }
    anchor
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn speculative_second_slot_preserves_the_single_probe_parse() {
        let mut input = Vec::from(&b"aaaaabaaaaac1234"[..]);
        let mut r = 1234u32;
        for j in 0..8192 {
            r = r.wrapping_mul(1664525).wrapping_add(1013904223);
            input.push(b'a' + ((r >> 28) as u8));
            if j % 13 == 0 {
                input.extend_from_slice(b"abcdefghijklmnopqrst_");
            }
        }
        for n in (0..23).chain([4095, input.len()]) {
            for archive in [false, true] {
                let a = encode(&input[..n], archive);
                let b = super::super::quick_impl::<1, 15, false, 2>(&input[..n], archive);
                assert_eq!(a, b);
            }
        }
    }
}
