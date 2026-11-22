// One eight-byte input word contains both overlapping hash prefixes (up to
// seven bytes each). The second full input word is only needed on a real
// candidate. Position zero is initialized before starting at byte one.
use super::{MASK, word};
#[inline(always)]
fn index<const P: usize, const B: usize>(w: u64) -> usize {
    ((w << (64 - 8 * P)).wrapping_mul(0x9e3779b97f4a7c15) >> (64 - B)) as usize
}
#[inline(always)]
fn check(s: &[u8], i: usize, prefix: u64, slot: u32) -> Option<(usize, u64)> {
    if (slot ^ (prefix as u32) << 21) > MASK as u32 {
        return None;
    }
    let q = slot as usize & MASK;
    // The table is private to this encoder block. Old entries were read
    // before this pair's writes, or refer to the definitely earlier first
    // byte. Even a fresh zero entry refers to a readable word at byte zero.
    debug_assert!(q < i && i + 8 <= s.len());
    let history = unsafe { s.as_ptr().add(q).cast::<u64>().read_unaligned() }.to_le();
    let x = history ^ prefix;
    let fail = (x as u32) != 0 || (x >> 32) as u8 != 0 && i - q > 12000;
    if fail { None } else { Some((q, history)) }
}
fn parse<const P: usize, const B: usize, const TAIL: usize>(
    s: &[u8],
    mut emit: impl FnMut(usize, usize, usize, usize),
) -> usize {
    if s.len() < 9 {
        return 0;
    }
    debug_assert!(s.len() <= MASK + 1 && P <= 7);
    let mut t = vec![0u32; 1 << B];
    let first = word(s, 0);
    t[index::<P, B>(first)] = (first as u32) << 21;
    let mut i = 1;
    let mut anchor = 0;
    while i + 8 <= s.len() {
        let w = word(s, i);
        let h = index::<P, B>(w);
        let slot = (w as u32) << 21 | i as u32;
        let old = t[h];
        let found = if i + 9 <= s.len() {
            let prefix = w >> 8;
            let h1 = index::<P, B>(prefix);
            let mut old1 = t[h1];
            if h == h1 {
                old1 = slot
            }
            t[h] = slot;
            if let Some((q, v)) = check(s, i, w, old) {
                Some((i, q, v ^ w))
            } else {
                t[h1] = (prefix as u32) << 21 | (i + 1) as u32;
                let hit = check(s, i + 1, prefix, old1);
                if hit.is_none() {
                    i += 2;
                    continue;
                }
                hit.map(|(q, v)| (i + 1, q, v ^ word(s, i + 1)))
            }
        } else {
            t[h] = slot;
            check(s, i, w, old).map(|(q, v)| (i, q, v ^ w))
        };
        let Some((at, q, d)) = found else { break };
        let n = super::lengthword::forward(s, q, at, d);
        let back = super::lengthword::backward(s, anchor, q, at);
        emit(anchor, at - back, n + back, at - q);
        i = at + n;
        anchor = i;
        for j in i - TAIL..i {
            if j + 8 <= s.len() {
                let v = word(s, j);
                t[index::<P, B>(v)] = (v as u32) << 21 | j as u32
            }
        }
    }
    anchor
}
pub(super) fn walk(s: &[u8], key: usize, emit: impl FnMut(usize, usize, usize, usize)) -> usize {
    if key == 10 {
        parse::<7, 14, 1>(s, emit)
    } else {
        parse::<6, 15, 2>(s, emit)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn one_word_supplies_two_keys_and_deferred_second_load_preserves_native_parse() {
        let mut s = vec![0; 28];
        let mut r = 223191u32;
        for i in 0..145_057 {
            r = r.wrapping_mul(1664525).wrapping_add(1013904223);
            let b = if i > 88 && (i % 79) < 58 {
                s[s.len() - 71]
            } else if i % 19 < 7 {
                0
            } else {
                (r >> 24) as u8
            };
            s.push(b);
        }
        for key in [5, 10] {
            for n in (0..41).chain([4095, 13290, s.len()]) {
                let mut gold = Vec::new();
                let record = |a, i, l, d| gold.push((a, i, l, d));
                let end = if key == 10 {
                    super::super::quicktwo::walk_key::<7, 14, 1, true>(&s[..n], record)
                } else {
                    super::super::quicktwo::walk_key::<6, 15, 2, true>(&s[..n], record)
                };
                let mut actual = Vec::new();
                let pos = walk(&s[..n], key, |a, i, l, d| actual.push((a, i, l, d)));
                assert_eq!((actual, pos), (gold, end));
            }
            {
                let mut v = Vec::new();
                let end = walk(&s, key, |a, i, l, d| {
                    assert_eq!(v.len(), a);
                    v.extend_from_slice(&s[a..i]);
                    for _ in 0..l {
                        v.push(v[v.len() - d])
                    }
                });
                v.extend_from_slice(&s[end..]);
                assert_eq!(v, s);
            }
        }
    }
}
