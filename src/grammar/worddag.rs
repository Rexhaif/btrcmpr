// Store a phrase either as literal bytes or as two shorter dictionary entries.
// All phrases of one width are adjacent. A small head plus a few index bits
// locates both the spelling and the output position without a large rank table.
use super::{Numbers, Term, merge_rules, numbers};
use crate::{Result, ans, bad, take, u32in, wordbase::plain};

#[derive(Clone, Copy)]
struct Group {
    w: usize,
    lo: usize,
    hi: usize,
    freq: u64,
}
fn width(n: usize) -> usize {
    usize::BITS as usize - (n - 1).leading_zeros() as usize
}
fn cost(n: usize, f: u64) -> f64 {
    if f == 0 {
        0.
    } else {
        let f = f as f64;
        f * (width(n) as f64 - f.log2())
    }
}
pub(crate) fn preset(s: &[u8], level: u8) -> Vec<u8> {
    let (cap, width) = match level {
        13 => (16384, 48),
        14 => (32768, 64),
        _ => (65536, 64),
    };
    encode_with(s, cap, width, 3)
}

fn encode_with(s: &[u8], cap: usize, max: usize, minimum: u32) -> Vec<u8> {
    if s.is_empty() {
        return vec![0];
    }
    let (terms, ids, rules) = merge_rules::<true>(s, cap, max, minimum);

    let mut out = vec![48];
    let mut uses = vec![0u32; terms.len()];
    for &v in &ids {
        uses[v as usize] += 1;
    }
    let Some(book) = build_book(s, &terms, &rules, uses, ids.len(), &mut out) else {
        let mut raw = vec![0];
        raw.extend(s);
        return raw;
    };

    put_root(&ids, &book, &mut out, false);

    out
}
struct EncodeBook {
    groups: Vec<Group>,
    heads: Vec<u8>,
    global: Vec<u32>,
    rows: [(usize, usize, usize); 65],
}
struct DecodeBook {
    meta: [Meta; 256],
    spell: Vec<u8>,
}
fn build_book(
    s: &[u8],
    terms: &[Term],
    rules: &[(u32, u32)],
    uses: Vec<u32>,
    nt: usize,
    out: &mut Vec<u8>,
) -> Option<EncodeBook> {
    let leaf = terms.len() - rules.len();
    let mut alive: Vec<_> = uses.iter().map(|&f| f != 0).collect();
    let mut refs = vec![0u32; terms.len()];
    for (j, &(a, b)) in rules.iter().enumerate().rev() {
        if alive[leaf + j] {
            for v in [a as usize, b as usize] {
                alive[v] = true;
                refs[v] += 1;
            }
        }
    }
    let raw = |id: usize| {
        let t = terms[id];
        &s[t.pos as usize..t.pos as usize + usize::from(t.len)]
    };
    let mut groups: Vec<Group> = vec![];
    let mut ord = Vec::new();
    let mut rows = [(0usize, 0usize, 0usize); 65];
    for (w, row) in rows.iter_mut().enumerate().skip(1) {
        let start = ord.len();
        ord.extend((0..terms.len()).filter(|&j| usize::from(terms[j].len) == w && uses[j] != 0));
        ord[start..].sort_unstable_by_key(|&j| (std::cmp::Reverse(uses[j]), j));
        let hi = ord.len();
        let mut block = 1;
        let mut at = start;
        while at < hi {
            let end = (at + block).min(hi);
            groups.push(Group {
                w,
                lo: at,
                hi: end,
                freq: ord[at..end].iter().map(|&j| u64::from(uses[j])).sum(),
            });
            at = end;
            block *= 2;
        }
        ord.extend(
            (0..terms.len())
                .filter(|&j| usize::from(terms[j].len) == w && uses[j] == 0 && alive[j]),
        );
        ord[hi..].sort_unstable_by_key(|&j| (std::cmp::Reverse(refs[j]), j));
        *row = (start, hi, ord.len());
    }
    while groups.len() > 256 {
        let j = (0..groups.len() - 1)
            .filter(|&j| groups[j].w == groups[j + 1].w)
            .min_by(|&i, &j| {
                let loss = |i: usize| {
                    let a = groups[i];
                    let b = groups[i + 1];
                    cost(b.hi - a.lo, a.freq + b.freq)
                        - cost(a.hi - a.lo, a.freq)
                        - cost(b.hi - b.lo, b.freq)
                };
                loss(i).total_cmp(&loss(j))
            })
            .unwrap();
        let b = groups.remove(j + 1);
        groups[j].hi = b.hi;
        groups[j].freq += b.freq;
    }
    // Sort within a frequency band for front coding and repeated pair parents.
    for g in &groups {
        ord[g.lo..g.hi].sort_unstable_by(|&a, &b| raw(a).cmp(raw(b)).then(a.cmp(&b)));
    }
    for &(_, a, b) in &rows {
        ord[a..b].sort_unstable_by(|&a, &b| raw(a).cmp(raw(b)).then(a.cmp(&b)));
    }
    let spellbytes: usize = ord.iter().map(|&j| usize::from(terms[j].len)).sum();
    if spellbytes > (s.len().saturating_mul(4)).min((1 << 26) - 64) {
        return None;
    }
    let mut global = vec![0u32; terms.len()];
    let mut heads = vec![0u8; terms.len()];
    for (w, &(lo, _, hi)) in rows.iter().enumerate().skip(1) {
        for (k, &j) in ord[lo..hi].iter().enumerate() {
            global[j] = k as u32;
            debug_assert_eq!(usize::from(terms[j].len), w);
        }
    }
    out.push((groups.len() - 1) as u8);
    out.extend((nt as u32).to_le_bytes());
    for &(_, mid, hi) in rows.iter().skip(1) {
        out.extend(((hi - mid) as u32).to_le_bytes());
    }
    for (i, g) in groups.iter().enumerate() {
        out.push(g.w as u8);
        out.extend(((g.hi - g.lo) as u32).to_le_bytes());
        for &j in &ord[g.lo..g.hi] {
            heads[j] = i as u8;
        }
    }
    let (mut split, mut same, mut dna) = (vec![], vec![], vec![]);
    let (mut left, mut right) = (Numbers::default(), Numbers::default());
    for &(lo, _, hi) in rows.iter().skip(1) {
        let mut prev = &[][..];
        let (mut pl, mut pr) = (u32::MAX, u32::MAX);
        for &j in &ord[lo..hi] {
            if j < leaf {
                let word = raw(j);
                let k = prev.iter().zip(word).take_while(|(a, b)| a == b).count();
                split.push(0);
                same.push(k as u8);
                dna.extend(&word[k..]);
                prev = word;
            } else {
                let (a, b) = rules[j - leaf];
                let l = global[a as usize];
                let r = global[b as usize];
                split.push(terms[a as usize].len);
                left.push(if l == pl { 0 } else { l + 1 });
                right.push(if r == pr { 0 } else { r + 1 });
                pl = l;
                pr = r;
            }
        }
    }
    for bytes in [&split, &same, &dna] {
        ans::put_tight(bytes, out)
    }
    left.finish(out);
    right.finish(out);
    Some(EncodeBook {
        groups,
        heads,
        global,
        rows,
    })
}
fn put_root(
    ids: &[u32],
    book: &EncodeBook,
    out: &mut Vec<u8>,
    conditional: bool,
) -> (usize, usize) {
    let EncodeBook {
        groups,
        heads,
        global,
        rows,
        ..
    } = book;
    let mut top = Vec::with_capacity(ids.len());
    let mut bits = Vec::new();
    let (mut acc, mut used) = (0u64, 0usize);
    for &id in ids {
        let j = id as usize;
        let head = heads[j];
        let g = groups[usize::from(head)];
        let index = global[j] as usize - (g.lo - rows[g.w].0);
        top.push(head);
        acc |= (index as u64) << used;
        used += width(g.hi - g.lo);
        while used >= 8 {
            bits.push(acc as u8);
            used -= 8;
            acc >>= 8
        }
    }
    if used != 0 {
        bits.push(acc as u8)
    }
    let mut top_wire = Vec::new();
    ans::put_tight(&top, &mut top_wire);
    if conditional && top.len() > 2048 {
        let conditioned = crate::context::candidate(&top, None);
        if conditioned.len() < top_wire.len() {
            top_wire = conditioned;
        }
    }
    let sizes = (top_wire.len(), bits.len());
    out.extend(top_wire);
    out.extend((bits.len() as u32).to_le_bytes());
    out.extend(bits);
    sizes
}
#[derive(Default, Clone, Copy)]
struct Meta {
    w: u8,
    k: u8,
    start: u32,
    n: u32,
}
type Fields<'a> = (
    std::borrow::Cow<'a, [u8]>,
    std::borrow::Cow<'a, [u8]>,
    std::borrow::Cow<'a, [u8]>,
    Vec<u32>,
    Vec<u32>,
);
fn legacy_fields<'a>(p: &mut &'a [u8], total: usize, bytes: usize) -> Result<Fields<'a>> {
    Ok((
        plain(p, total)?,
        plain(p, total)?,
        plain(p, bytes)?,
        numbers(p, total)?,
        numbers(p, total)?,
    ))
}
fn read_book(p: &mut &[u8], n: usize) -> Result<(usize, DecodeBook)> {
    let ng = usize::from(take(p, 1)?[0]) + 1;
    let nt = u32in(p)?;
    if nt == 0 || nt > n {
        return Err(bad());
    }
    let mut sizes = [0usize; 65];
    for count in &mut sizes[1..] {
        *count = u32in(p)?;
    }
    let mut meta = [Meta::default(); 256];
    let mut hot = [0usize; 65];
    let mut prev = 0;
    for m in &mut meta[..ng] {
        let w = usize::from(take(p, 1)?[0]);
        let count = u32in(p)?;
        if !(1..=64).contains(&w) || w < prev || count == 0 || count > nt {
            return Err(bad());
        }
        prev = w;
        *m = Meta {
            w: w as u8,
            k: width(count) as u8,
            start: hot[w] as u32,
            n: count as u32,
        };
        hot[w] = hot[w].checked_add(count).ok_or_else(bad)?;
        if hot[w] > nt {
            return Err(bad());
        }
    }
    let mut base = [0usize; 65];
    let (mut total, mut bytes) = (0usize, 0usize);
    for w in 1..=64 {
        sizes[w] = sizes[w].checked_add(hot[w]).ok_or_else(bad)?;
        base[w] = bytes;
        total = total.checked_add(sizes[w]).ok_or_else(bad)?;
        bytes = bytes
            .checked_add(w.checked_mul(sizes[w]).ok_or_else(bad)?)
            .ok_or_else(bad)?;
    }
    if bytes > n.saturating_mul(4).min((1 << 26) - 64) || total == 0 {
        return Err(bad());
    }
    for m in &mut meta[..ng] {
        let w = usize::from(m.w);
        m.start = base[w] as u32 + m.start * u32::from(m.w);
    }
    let (div, keeps, raw, lv, rv) = if p.starts_with(&[0, 0, 0, 0, 55]) {
        bookpack::fields(p, &sizes, total, bytes)?
    } else {
        legacy_fields(p, total, bytes)?
    };
    if div.len() != total || rv.len() != lv.len() || keeps.len() + lv.len() != total {
        return Err(bad());
    }
    let mut spell = vec![0; bytes + 64];
    let (mut dx, mut lx, mut ix, mut from) = (0, 0, 0, 0);
    for w in 1..=64 {
        let mut old = [0u8; 64];
        let mut prior = false;
        let (mut pl, mut pr) = (u32::MAX, u32::MAX);
        for j in 0..sizes[w] {
            let q = base[w] + j * w;
            let a = usize::from(div[dx]);
            dx += 1;
            if a == 0 {
                let k = usize::from(*keeps.get(lx).ok_or_else(bad)?);
                lx += 1;
                if k > w || (!prior && k != 0) {
                    return Err(bad());
                }
                let count = w - k;
                old[k..w].copy_from_slice(raw.get(from..from + count).ok_or_else(bad)?);
                from += count;
                spell[q..q + w].copy_from_slice(&old[..w]);
                prior = true;
            } else {
                if a >= w || ix >= lv.len() {
                    return Err(bad());
                }
                if lv[ix] != 0 {
                    pl = lv[ix] - 1
                }
                if rv[ix] != 0 {
                    pr = rv[ix] - 1
                }
                ix += 1;
                let b = w - a;
                if pl as usize >= sizes[a] || pr as usize >= sizes[b] {
                    return Err(bad());
                }
                let l = base[a] + pl as usize * a;
                let r = base[b] + pr as usize * b;
                // A child has strictly smaller width and is already complete.
                spell.copy_within(l..l + a, q);
                spell.copy_within(r..r + b, q + a);
            }
        }
    }
    if from != raw.len() || lx != keeps.len() || ix != lv.len() {
        return Err(bad());
    }
    Ok((nt, DecodeBook { meta, spell }))
}
fn read_root(mut p: &[u8], nt: usize, book: &DecodeBook, dst: &mut [u8]) -> Result<()> {
    let n = dst.len();
    if nt == 0 || nt > n {
        return Err(bad());
    }

    let DecodeBook { meta, spell } = book;
    // The optional rank alphabet has exactly one layer of conditioning;
    // context::get validates that its own payload is a plain entropy stream.
    let h = if matches!(p.get(4), Some(22)) {
        ans::get(&mut p, nt)?
    } else {
        plain(&mut p, nt)?
    };
    let nb = u32in(&mut p)?;
    if h.len() != nt || p.len() != nb {
        return Err(bad());
    }
    let mut stream = Vec::from(p);
    stream.extend([0; 8]);
    let (mut at, mut bit) = (0usize, 0usize);
    let mut block = [Meta::default(); 32];
    let mut pos = [0usize; 32];
    let mut fields = [0usize; 32];
    let mut source = [0usize; 32];
    for tile in h.chunks(32) {
        for (j, &head) in tile.iter().enumerate() {
            let m = meta[usize::from(head)];
            block[j] = m;
            pos[j] = at;
            fields[j] = bit;
            at += usize::from(m.w);
            bit += usize::from(m.k);
        }
        if at > n || bit > nb * 8 {
            return Err(bad());
        }
        let mut invalid = 0;
        for (j, d) in source[..tile.len()].iter_mut().enumerate() {
            let m = block[j];
            let b = fields[j];
            let v = unsafe { stream.as_ptr().add(b / 8).cast::<u32>().read_unaligned() }.to_le();
            let x = (v >> (b & 7)) & ((1 << m.k) - 1);
            invalid |= u32::from(x >= m.n);
            *d = (m.start + x * u32::from(m.w)) as usize;
            #[cfg(target_arch = "aarch64")]
            unsafe {
                core::arch::asm!("prfm pldl1keep, [{p}]",p=in(reg)spell.as_ptr().wrapping_add(*d), options(nostack,readonly,preserves_flags));
            }
        }
        if invalid != 0 {
            return Err(bad());
        }
        for j in 0..tile.len() {
            let w = usize::from(block[j].w);
            let q = source[j];
            let d = pos[j];
            if at + 64 <= n {
                unsafe {
                    let s = spell.as_ptr().add(q);
                    let o = dst.as_mut_ptr().add(d);
                    crate::lz::copy16(s, o);
                    if w > 16 {
                        crate::lz::copy16(s.add(16), o.add(16));
                        if w > 32 {
                            crate::lz::copy16(s.add(32), o.add(32));
                            if w > 48 {
                                crate::lz::copy16(s.add(48), o.add(48));
                            }
                        }
                    }
                }
            } else {
                dst[d..d + w].copy_from_slice(&spell[q..q + w]);
            }
        }
    }
    if at != n || nb != bit.div_ceil(8) || (!bit.is_multiple_of(8) && p[bit / 8] >> (bit & 7) != 0)
    {
        return Err(bad());
    }
    Ok(())
}
pub(crate) fn decode(mut p: &[u8], dst: &mut [u8]) -> Result<()> {
    let (nt, book) = read_book(&mut p, dst.len())?;
    read_root(p, nt, &book, dst)
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_phrases_restore_and_reject_bad_tables() {
        let mut s = vec![];
        for i in 0..1200 {
            s.extend(format!("After the river crossing {} riders followed the riders and crossed the river. {}\n",i%43,i%127).bytes());
            if i % 11 == 0 {
                s.extend([0xff, 0x97]);
            }
        }
        let wire = encode_with(&s, 32768, 64, 2);
        assert_eq!(wire[0], 48);
        let mut out = vec![0xaf; s.len() + 32];
        decode(&wire[1..], &mut out[16..16 + s.len()]).unwrap();
        assert_eq!(&out[16..16 + s.len()], s);
        assert!(
            out[..16]
                .iter()
                .chain(&out[16 + s.len()..])
                .all(|&x| x == 0xaf)
        );
        for i in [1, 4, 48, 192, wire.len() - 1] {
            assert!(decode(&wire[1..i], &mut out[16..16 + s.len()]).is_err());
        }
        let mut bad = wire.clone();
        bad[6..10].fill(255);
        assert!(decode(&bad[1..], &mut out[16..16 + s.len()]).is_err());
    }
}

mod bookpack;

pub(crate) mod shared;
