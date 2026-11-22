// Decode a large job in smaller metadata pages, retaining the original match
// history and sharing one block-wide literal model across page boundaries.
use super::{BookTags, Streams, from_page, lenin};
use crate::{Result, ans, bad, take, u32in};

pub(super) const COMMANDS: usize = 131_072;
pub(super) struct Cursor {
    pub(super) at: usize,
    pub(super) rep: [usize; 3],
}
impl Default for Cursor {
    fn default() -> Self {
        Self {
            at: 0,
            rep: [1, 4, 8],
        }
    }
}

pub(super) fn encode<const N: usize>(st: &Streams, global: &[u8]) -> Vec<u8> {
    encode_variant(st, global, N, 1)
}

fn put_field(stream: &[u8], out: &mut Vec<u8>, exact: bool) {
    if exact {
        ans::put_alternative(stream, out, 3)
    } else {
        ans::put_policy(stream, out, 0.985)
    }
}
pub(super) fn encode_variant(st: &Streams, global: &[u8], per_page: usize, policy: u8) -> Vec<u8> {
    let mut out = Vec::with_capacity(global.len() + 2 * st.tag.len());
    out.extend([41, 1]);
    out.extend(global);
    let mut lens = st.lens.as_slice();
    let mut far = st.far.as_slice();
    let (mut lo, mut mi, mut at, mut lit) = (0, 0, 0, 0);
    for (i, (tags, offs)) in st
        .tag
        .chunks(per_page)
        .zip(st.offs.chunks(per_page))
        .enumerate()
    {
        let (lo0, mi0, at0, lit0) = (lo, mi, at, lit);
        let (x, y) = (lens, far);
        for (&t, &o) in tags.iter().zip(offs) {
            let mut ll = usize::from(t >> 4);
            let m = usize::from(t & 15);
            if ll == 15 {
                ll += lenin(&mut lens).expect("encoder literal length");
            }
            let ml = m
                + st.minimum
                + if m == 15 {
                    lenin(&mut lens).expect("encoder match length")
                } else {
                    0
                };
            lo += usize::from(o >= 84);
            mi += usize::from(o >= 212);
            if o == 255 {
                lenin(&mut far).expect("encoder distance");
            }
            at += ll + ml;
            lit += ll;
        }
        if (i + 1) * per_page >= st.tag.len() {
            at += st.lits.len() - lit;
            lit = st.lits.len();
        }
        let far_page = &y[..y.len() - far.len()];
        out.extend(((at - at0) as u32).to_le_bytes());
        out.extend(((lit - lit0) as u32).to_le_bytes());
        let field = out.len();
        out.extend([0; 4]);
        let start = out.len();
        out.push(match (st.minimum, far_page.is_empty()) {
            (3, true) => 16,
            (3, false) => 17,
            (_, true) => 1,
            _ => 13,
        });
        put_field(tags, &mut out, policy == 1 || policy == 2);
        put_field(offs, &mut out, policy == 1 || policy == 3);
        ans::put_policy(&x[..x.len() - lens.len()], &mut out, 0.985);
        out.extend([0; 5]); // literal bytes are in the block-wide stream
        for v in [&st.low[lo0..lo], &st.mid[mi0..mi]] {
            let mut padded = Vec::with_capacity(v.len() + 16);
            padded.extend(v);
            padded.extend([0; 16]);
            ans::put_policy(&padded, &mut out, 0.985);
        }
        if !far_page.is_empty() {
            ans::put_policy(far_page, &mut out, 0.985);
        }
        let bytes = (out.len() - start) as u32;
        out[field..field + 4].copy_from_slice(&bytes.to_le_bytes());
    }
    debug_assert!(lens.is_empty() && far.is_empty() && lo == st.low.len() && mi == st.mid.len());
    out
}

pub(crate) fn decode(mut p: &[u8], dst: &mut [u8]) -> Result<()> {
    if take(&mut p, 1)?[0] != 1 || !matches!(p.get(4), Some(0 | 1 | 2 | 4 | 5 | 22)) {
        return Err(bad());
    }
    let literals = ans::get(&mut p, dst.len())?;
    let mut lits = &*literals;
    let mut cursor = Cursor::default();
    while !p.is_empty() {
        let bytes = u32in(&mut p)?;
        let ll = u32in(&mut p)?;
        let packed = u32in(&mut p)?;
        if bytes == 0 || bytes > dst.len() - cursor.at || ll > bytes {
            return Err(bad());
        }
        let end = cursor.at + bytes;
        let ext = take(&mut lits, ll)?;
        let (&mode, inner) = take(&mut p, packed)?.split_first().ok_or_else(bad)?;
        macro_rules! page {
            ($e:literal,$m:literal) => {
                from_page::<$e, $m, true, true, false, false, true, false>(
                    inner,
                    &mut dst[..end],
                    ext,
                    &[],
                    0,
                    BookTags::default(),
                    &mut cursor,
                    [0; 3],
                )
            };
        }
        match mode {
            1 => page!(false, 4),
            13 => page!(true, 4),
            16 => page!(false, 3),
            17 => page!(true, 3),
            _ => Err(bad()),
        }?;
    }
    if cursor.at != dst.len() || !lits.is_empty() {
        return Err(bad());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn carries_offsets_and_history_and_rejects_damaged_page() {
        let s = b"abcdabcdabcdabcdabcdabcdabcdabcd tail";
        let mut st = Streams::new();
        for j in 1..8 {
            st.emit(s, if j == 1 { 0 } else { 4 * j }, 4 * j, 4, 4);
        }
        st.lits.extend_from_slice(&s[32..]);
        let mut global = vec![];
        ans::put_tight(&st.lits, &mut global);
        let wire = encode::<2>(&st, &global);
        let mut buf = vec![0; s.len()];
        crate::decode(&wire, &mut buf).unwrap();
        assert_eq!(&buf, s);
        assert!(crate::decode(&wire[..wire.len() - 1], &mut buf).is_err());
        let mut bad = wire.clone();
        let n = 2 + global.len();
        bad[n..n + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(crate::decode(&bad, &mut buf).is_err());
        let mut bad = wire;
        bad[n + 12] = 41;
        assert!(crate::decode(&bad, &mut buf).is_err());
    }
}
