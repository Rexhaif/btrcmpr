// Pack dictionary fields separately. Same-width child ranks often have small deltas.
use super::{Fields, Numbers, legacy_fields, numbers};
use crate::{Result, ans, bad, take, u32in, wordbase::plain};
use std::borrow::Cow;

const MARK: &[u8] = &[0, 0, 0, 0, 55];
fn byte_put(data: &[u8], methods: usize) -> Vec<u8> {
    let mut best = Vec::new();
    ans::put_tight(data, &mut best);
    if data.len() < 512 {
        return best;
    }
    if methods & 1 != 0 {
        let mut v = (data.len() as u32).to_le_bytes().to_vec();
        v.push(56);
        for part in data.chunks(1 << 18) {
            let wire = crate::lz::encode_level::<false>(part, 5);
            if !matches!(wire.first(), Some(0 | 1 | 13 | 16 | 17)) {
                return best;
            }
            v.extend((wire.len() as u32).to_le_bytes());
            v.extend(wire);
        }
        if v.len() < best.len() {
            best = v
        }
    }
    if methods & 4 != 0 {
        let v = crate::context::candidate(data, None);
        if v.len() < best.len() {
            best = v
        }
    }
    best
}
fn byte_get<'a>(p: &mut &'a [u8], limit: usize) -> Result<Cow<'a, [u8]>> {
    match p.get(4) {
        Some(22) => ans::get(p, limit),
        Some(56) => {
            let n = u32in(p)?;
            take(p, 1)?;
            if n > limit {
                return Err(bad());
            }
            let mut raw = vec![0; n];
            for part in raw.chunks_mut(1 << 18) {
                let m = u32in(p)?;
                let v = take(p, m)?;
                if !matches!(v.first(), Some(0 | 1 | 13 | 16 | 17)) {
                    return Err(bad());
                }
                crate::decode(v, part)?;
            }
            Ok(Cow::Owned(raw))
        }
        _ => plain(p, limit),
    }
}
fn numbers_wire(v: &[u32]) -> Vec<u8> {
    let mut n = Numbers::default();
    for &x in v {
        n.push(x)
    }
    let mut out = vec![];
    n.finish(&mut out);
    out
}
// mode 1 predicts with the previous child of this width throughout the book;
// mode 2 keeps independent history for each parent width and child width.
fn predict<const DECODE: bool>(
    source: &[u32],
    div: &[u8],
    sizes: &[usize; 65],
    right: bool,
    mode: u8,
) -> Result<Vec<u32>> {
    if !(1..=2).contains(&mode) {
        return Err(bad());
    }
    let mut state = vec![0u32; if mode == 1 { 65 } else { 65 * 65 }];
    let mut out = Vec::with_capacity(source.len());
    let mut index = 0;
    for (w, &count) in sizes.iter().enumerate().skip(1) {
        let mut old = u32::MAX;
        let row = div.get(index..index + count).ok_or_else(bad)?;
        index += count;
        for &a in row {
            if a == 0 {
                continue;
            }
            if a as usize >= w {
                return Err(bad());
            }
            let width = if right {
                w - usize::from(a)
            } else {
                usize::from(a)
            };
            let p = &mut state[if mode == 1 { width } else { w * 65 + width }];
            let v = *source.get(out.len()).ok_or_else(bad)?;
            let rank = if DECODE {
                i64::from(*p) + (i64::from(v >> 1) ^ -i64::from(v & 1))
            } else if v == 0 {
                i64::from(old)
            } else {
                i64::from(v - 1)
            };
            if rank < 0 || rank as usize >= sizes[width] {
                return Err(bad());
            }
            let x = rank as u32;
            let value = if DECODE {
                if x == old { 0 } else { x + 1 }
            } else {
                let d = rank - i64::from(*p);
                ((d << 1) ^ (d >> 63)) as u32
            };
            old = x;
            *p = x;
            out.push(value);
        }
    }
    if index != div.len() || out.len() != source.len() {
        return Err(bad());
    }
    Ok(out)
}
pub(super) fn fields<'a>(
    p: &mut &'a [u8],
    sizes: &[usize; 65],
    total: usize,
    bytes: usize,
) -> Result<Fields<'a>> {
    if take(p, 5)? != MARK {
        return Err(bad());
    }
    let modes = take(p, 2)?;
    let div = byte_get(p, total)?;
    let keeps = byte_get(p, total)?;
    let dna = byte_get(p, bytes)?;
    if div.len() != total {
        return Err(bad());
    }
    let mut l = numbers(p, total)?;
    let mut r = numbers(p, total)?;
    if modes[0] != 0 {
        l = predict::<true>(&l, &div, sizes, false, modes[0])?
    }
    if modes[1] != 0 {
        r = predict::<true>(&r, &div, sizes, true, modes[1])?
    }
    Ok((div, keeps, dna, l, r))
}

pub(super) fn repack(src: &[u8], n: usize, methods: usize) -> Result<Vec<u8>> {
    let mut p = src;
    let groups = usize::from(take(&mut p, 1)?[0]) + 1;
    let nt = u32in(&mut p)?;
    if nt == 0 || nt > n {
        return Err(bad());
    }
    let mut sizes = [0usize; 65];
    for count in &mut sizes[1..] {
        *count = u32in(&mut p)?
    }
    for _ in 0..groups {
        let w = usize::from(take(&mut p, 1)?[0]);
        let count = u32in(&mut p)?;
        if w == 0 || w > 64 || count == 0 {
            return Err(bad());
        }
        sizes[w] = sizes[w].checked_add(count).ok_or_else(bad)?;
    }
    let bytes: usize = sizes.iter().enumerate().map(|(w, &k)| w * k).sum();
    let total = sizes.iter().sum();
    if total == 0 || bytes > n.saturating_mul(4).min((1 << 26) - 64) {
        return Err(bad());
    }
    let header = src.len() - p.len();
    let (div, keeps, dna, l, r) = legacy_fields(&mut p, total, bytes)?;
    if !p.is_empty() || div.len() != total || l.len() != r.len() || keeps.len() + l.len() != total {
        return Err(bad());
    }
    let mut v = src[..header].to_vec();
    v.extend(MARK);
    v.extend([0, 0]);
    for raw in [&div, &keeps, &dna] {
        v.extend(byte_put(raw, methods))
    }
    for (side, old) in [&l, &r].iter().enumerate() {
        let mut best = numbers_wire(old);
        if methods & 2 != 0 {
            for mode in 1..=2 {
                let coded = numbers_wire(&predict::<false>(old, &div, &sizes, side == 1, mode)?);
                if coded.len() < best.len() {
                    v[header + 5 + side] = mode;
                    best = coded
                }
            }
        }
        v.extend(best)
    }
    Ok(if v.len() < src.len() { v } else { src.to_vec() })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parent_predictions_reset_per_width_and_reject_out_of_range_children() {
        let mut sizes = [0; 65];
        sizes[1] = 10;
        sizes[2] = 6;
        sizes[3] = 2;
        let mut div = vec![0; 10];
        div.extend([1, 1, 1, 1, 1, 1, 1, 2]);
        for right in [true, false] {
            let old = [1, 0, 2, 0, 1, 10, 2, 5];
            for mode in [1, 2] {
                let v = predict::<false>(&old, &div, &sizes, right, mode).unwrap();
                assert_eq!(predict::<true>(&v, &div, &sizes, right, mode).unwrap(), old);
                let mut invalid = v;
                invalid[0] = u32::MAX;
                assert!(predict::<true>(&invalid, &div, &sizes, right, mode).is_err());
            }
        }
    }
    #[test]
    fn a_compressed_dictionary_byte_field_cannot_contain_a_nested_frame() {
        let mut data = vec![];
        data.extend(1024u32.to_le_bytes());
        data.push(56);
        data.extend(5u32.to_le_bytes());
        data.extend([50, 1, 0, 0, 0]);
        assert!(byte_get(&mut data.as_slice(), 2048).is_err());
        let stream = vec![b'x'; 1007];
        let wire = byte_put(&stream, 7);
        let mut p = wire.as_slice();
        assert_eq!(byte_get(&mut p, 1007).unwrap().as_ref(), stream);
        assert!(p.is_empty());
    }
}
