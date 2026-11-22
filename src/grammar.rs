// A block-local vocabulary of word sequences. Frequent adjacent tokens can
// become a new token; only spellings used as literals are sent to the decoder.
use crate::{Result, ans, bad, take, u32in};

#[derive(Clone, Copy)]
struct Term {
    pos: u32,
    len: u8,
}
#[derive(Clone, Copy, Default)]
struct Slot {
    key: u64,
    count: u32,
    id: u32,
}
fn hash(v: u64, mask: usize) -> usize {
    let x = (v ^ (v >> 29)).wrapping_mul(0xa24baed4963ee407);
    ((x ^ (x >> 31)) as usize) & mask
}

fn merge(s: &[u8], cap: usize, max: usize) -> (Vec<Term>, Vec<u32>) {
    let (terms, ids, _) = merge_rules::<false>(s, cap, max, 4);
    (terms, ids)
}
fn merge_rules<const RULES: bool>(
    s: &[u8],
    cap: usize,
    max: usize,
    minimum: u32,
) -> (Vec<Term>, Vec<u32>, Vec<(u32, u32)>) {
    let (words, mut ids) = crate::wordbase::phrase_tokens::<RULES>(s);
    let mut rules = Vec::new();
    let mut terms: Vec<_> = words
        .iter()
        .map(|w| Term {
            pos: w.pos,
            len: w.len,
        })
        .collect();
    let original = terms.len();
    if cap == 0 || ids.len() < 64 {
        return (terms, ids, rules);
    }
    let size = (ids.len() / (if RULES { 2 } else { 4 }))
        .next_power_of_two()
        .clamp(2048, if RULES { 1 << 21 } else { 1 << 19 });
    let mut slots = vec![Slot::default(); size];
    for round in 0..if RULES { 24 } else { 12 } {
        slots.fill(Slot::default());
        for w in ids.windows(2) {
            if terms[w[0] as usize].len as usize + terms[w[1] as usize].len as usize > max {
                continue;
            }
            let key = (u64::from(w[0]) << 32) | u64::from(w[1]);
            let e = &mut slots[hash(key, size - 1)];
            if key == e.key {
                e.count += 1
            } else if e.count == 0 {
                *e = Slot {
                    key,
                    count: 1,
                    id: 0,
                }
            } else {
                e.count -= 1
            }
        }
        let mut selected: Vec<_> = (0..size).filter(|&i| slots[i].count >= minimum).collect();
        let add = (1024usize << round.min(if RULES { 6 } else { 4 }))
            .min(cap + original - terms.len())
            .min(selected.len());
        if add == 0 {
            break;
        }
        if selected.len() > add {
            selected.select_nth_unstable_by_key(add, |&i| (std::cmp::Reverse(slots[i].count), i));
            selected.truncate(add);
        }
        for i in selected {
            let e = &mut slots[i];
            let a = (e.key >> 32) as usize;
            let b = e.key as u32 as usize;
            e.id = terms.len() as u32;
            if RULES {
                rules.push((a as u32, b as u32));
            }
            terms.push(Term {
                pos: u32::MAX,
                len: terms[a].len + terms[b].len,
            });
        }
        let (mut from, mut to, mut raw) = (0, 0, 0);
        while from < ids.len() {
            let id = ids[from];
            let mut sym = id;
            from += 1;
            if from < ids.len() {
                let key = (u64::from(id) << 32) | u64::from(ids[from]);
                let e = slots[hash(key, size - 1)];
                if e.id != 0 && e.key == key {
                    sym = e.id;
                    from += 1;
                    terms[sym as usize].pos = raw as u32;
                }
            }
            ids[to] = sym;
            to += 1;
            raw += terms[sym as usize].len as usize;
        }
        debug_assert_eq!(raw, s.len());
        ids.truncate(to);
        if terms.len() == original + cap {
            break;
        }
    }
    (terms, ids, rules)
}

#[derive(Default)]
struct Numbers {
    head: Vec<u8>,
    tail: Vec<u8>,
    acc: u64,
    bits: u32,
}
impl Numbers {
    fn push(&mut self, v: u32) {
        if v < 16 {
            self.head.push(v as u8);
            return;
        }
        let n = v.ilog2() - 3;
        self.head.push((8 * n + (v >> n)) as u8);
        self.acc |= u64::from(v & ((1 << n) - 1)) << self.bits;
        self.bits += n;
        while self.bits >= 8 {
            self.tail.push(self.acc as u8);
            self.acc >>= 8;
            self.bits -= 8;
        }
    }
    fn finish(mut self, out: &mut Vec<u8>) {
        if self.bits != 0 {
            self.tail.push(self.acc as u8);
        }
        ans::put_tight(&self.head, out);
        out.extend((self.tail.len() as u32).to_le_bytes());
        out.extend(self.tail);
    }
}
fn numbers(p: &mut &[u8], limit: usize) -> Result<Vec<u32>> {
    let h = crate::wordbase::plain(p, limit)?;
    let n = u32in(p)?;
    let stream = take(p, n)?;
    let mut bytes = Vec::from(stream);
    bytes.extend([0; 4]);
    let mut bit = 0usize;
    let mut out = Vec::with_capacity(h.len());
    for &v in h.iter() {
        if v < 16 {
            out.push(u32::from(v));
            continue;
        }
        let width = u32::from(v / 8) - 1;
        if width > 25 || bit + width as usize > 8 * n {
            return Err(bad());
        }
        // At most 25 bits plus a seven-bit shift. The stream has padding.
        let w = unsafe { bytes.as_ptr().add(bit / 8).cast::<u32>().read_unaligned() }.to_le();
        let low = (w >> (bit & 7)) & ((1 << width) - 1);
        out.push(((8 + u32::from(v & 7)) << width) | low);
        bit += width as usize;
    }
    if n != bit.div_ceil(8) || (!bit.is_multiple_of(8) && stream[bit / 8] >> (bit & 7) != 0) {
        return Err(bad());
    }
    Ok(out)
}

fn pack_words(src: &[u8], terms: &[Term], lits: &[u32]) -> Vec<u8> {
    let mut ct = vec![0u32; terms.len()];
    for &v in lits {
        ct[v as usize] += 1;
    }
    let spell = |id: usize| {
        let t = terms[id];
        &src[t.pos as usize..t.pos as usize + t.len as usize]
    };
    let mut used: Vec<_> = (0..terms.len()).filter(|&i| ct[i] != 0).collect();
    used.sort_unstable_by(|&a, &b| {
        ct[b]
            .ilog2()
            .cmp(&ct[a].ilog2())
            .then_with(|| spell(a).cmp(spell(b)))
    });
    let mut map = vec![0; terms.len()];
    let (mut same, mut lens, mut bytes) = (Vec::new(), Vec::new(), Vec::new());
    let mut prev = &[][..];
    for (rank, &id) in used.iter().enumerate() {
        let s = spell(id);
        let keep = s.iter().zip(prev).take_while(|(a, b)| a == b).count();
        same.push(keep as u8);
        lens.push((s.len() - keep) as u8);
        bytes.extend(&s[keep..]);
        map[id] = rank as u32;
        prev = s;
    }
    let mut out = Vec::new();
    out.extend((used.len() as u32).to_le_bytes());
    for v in [&same, &lens, &bytes] {
        ans::put_tight(v, &mut out);
    }
    let mut ids = Numbers::default();
    for &v in lits {
        ids.push(map[v as usize]);
    }
    ids.finish(&mut out);
    out
}
struct Vocabulary {
    spell: Vec<u8>,
    tokens: Vec<u32>,
}
fn unpack(p: &mut &[u8], n: usize) -> Result<Vocabulary> {
    let nd = u32in(p)?;
    if nd > n {
        return Err(bad());
    }
    let same = crate::wordbase::plain(p, nd)?;
    let len = crate::wordbase::plain(p, nd)?;
    let suffix = crate::wordbase::plain(p, n)?;
    let mut raw = &suffix[..];
    if same.len() != nd || len.len() != nd {
        return Err(bad());
    }
    let mut spell = Vec::new();
    let mut row = [0; 64];
    let mut prev = 0usize;
    let mut desc = Vec::with_capacity(nd);
    for (&k, &l) in same.iter().zip(len.iter()) {
        let k = k as usize;
        let l = l as usize;
        let w = k + l;
        if k > prev || !(1..=64).contains(&w) || spell.len() + w > n {
            return Err(bad());
        }
        row[k..w].copy_from_slice(take(&mut raw, l)?);
        desc.push(((spell.len() as u32) << 6) | (w as u32 - 1));
        spell.extend(&row[..w]);
        prev = w;
    }
    if !raw.is_empty() {
        return Err(bad());
    }
    spell.extend([0; 64]);
    let mut tokens = numbers(p, n)?;
    for v in &mut tokens {
        *v = *desc.get(*v as usize).ok_or_else(bad)?;
    }
    Ok(Vocabulary { spell, tokens })
}
#[cfg(target_arch = "aarch64")]
use wordpipe::expand;
#[cfg(not(target_arch = "aarch64"))]
fn expand(ids: &[u32], spell: &[u8], dst: &mut [u8]) -> Result<()> {
    let mut at = 0;
    for tile in ids.chunks(32) {
        let mut pos = [0u32; 32];
        let mut end = at;
        for (i, &v) in tile.iter().enumerate() {
            pos[i] = end as u32;
            end += (v as usize & 63) + 1;
            #[cfg(target_arch = "aarch64")]
            unsafe {
                core::arch::asm!("prfm pldl1keep, [{p}]",p=in(reg)spell.as_ptr().add((v>>6)as usize),options(readonly,nostack,preserves_flags));
            }
        }
        if end > dst.len() {
            return Err(bad());
        }
        if end + 64 <= dst.len() {
            for (i, &v) in tile.iter().enumerate() {
                let w = (v & 63) + 1;
                // Valid descriptors point to spellings in an arena padded by
                // 64 bytes. Later tokens overwrite an overstore in this tile.
                unsafe {
                    let s = spell.as_ptr().add((v >> 6) as usize);
                    let d = dst.as_mut_ptr().add(pos[i] as usize);
                    crate::lz::copy16(s, d);
                    if w > 16 {
                        crate::lz::copy16(s.add(16), d.add(16));
                        if w > 32 {
                            crate::lz::copy16(s.add(32), d.add(32));
                            if w > 48 {
                                crate::lz::copy16(s.add(48), d.add(48));
                            }
                        }
                    }
                }
            }
        } else {
            for (i, &v) in tile.iter().enumerate() {
                let len = (v as usize & 63) + 1;
                let q = (v >> 6) as usize;
                let d = pos[i] as usize;
                dst[d..d + len].copy_from_slice(&spell[q..q + len]);
            }
        }
        at = end;
    }
    if at != dst.len() { Err(bad()) } else { Ok(()) }
}

#[derive(Clone, Copy)]
struct Edge {
    len: u32,
    off: u32,
}
fn sequences(ids: &[u32], terms: usize) -> Vec<(usize, usize, usize)> {
    let n = ids.len();
    if n < 3 {
        return vec![];
    }
    let mut ct = vec![0u32; terms];
    for &v in ids {
        ct[v as usize] += 1;
    }
    let fee: Vec<_> = ct
        .iter()
        .map(|&c| (((n as f64 / c.max(1) as f64).log2() * 16.).round() as u32).max(8))
        .collect();
    let size = (n / 3).next_power_of_two().clamp(4096, 1 << 20);
    let mut pair = vec![[u32::MAX; 4]; size];
    let mut longer = vec![u32::MAX; size];
    let mut cost = vec![u64::MAX; n + 1];
    let mut back = vec![Edge { len: 0, off: 0 }; n + 1];
    cost[0] = 0;
    for i in 0..n {
        let base = cost[i];
        let literal = base + u64::from(fee[ids[i] as usize]);
        if literal < cost[i + 1] {
            cost[i + 1] = literal;
            back[i + 1] = Edge { len: 0, off: 0 };
        }
        if i + 2 >= n {
            continue;
        }
        let key = (u64::from(ids[i]) << 32) | u64::from(ids[i + 1]);
        let a = hash(key, size - 1);
        let b = hash(
            key.rotate_left(21) ^ u64::from(ids[i + 2]).wrapping_mul(0x9e3779b97f4a7c15),
            size - 1,
        );
        let old = pair[a];
        let q3 = std::mem::replace(&mut longer[b], i as u32);
        pair[a] = [i as u32, old[0], old[1], old[2]];
        let mut last = 1;
        // Earlier slots tend to be nearer and cheaper. A farther slot only
        // competes on lengths the nearer candidates could not reach.
        for q in [old[0], old[1], old[2], old[3], q3] {
            let q = q as usize;
            if q >= i || ids[q] != ids[i] || ids[q + 1] != ids[i + 1] {
                continue;
            }
            let mut len = 2;
            while len < 256 && i + len < n && ids[q + len] == ids[i + len] {
                len += 1;
            }
            if len <= last {
                continue;
            }
            let off = i - q;
            let paid = base + u64::from((off as u32).ilog2() * 16 + 118);
            let mut k = (last + 1).max(2);
            loop {
                let price = paid + u64::from(((k - 1) as u32).ilog2() * 15);
                if price < cost[i + k] {
                    cost[i + k] = price;
                    back[i + k] = Edge {
                        len: k as u32,
                        off: off as u32,
                    };
                }
                if k == len {
                    break;
                }
                k = if k < 18 {
                    k + 1
                } else if k < 64 {
                    (k + 4).min(len)
                } else {
                    len
                };
            }
            last = len;
            if len >= 128 {
                break;
            }
        }
    }
    let mut res = vec![];
    let mut at = n;
    while at != 0 {
        let e = back[at];
        if e.len == 0 {
            at -= 1;
        } else {
            at -= e.len as usize;
            res.push((at, e.len as usize, e.off as usize));
        }
    }
    res.reverse();
    res
}
fn excess(out: &mut Vec<u8>, mut v: usize) {
    while v >= 128 {
        out.push(v as u8 | 128);
        v >>= 7
    }
    out.push(v as u8);
}
pub(crate) fn native(s: &[u8]) -> Vec<u8> {
    if s.is_empty() {
        return vec![0];
    }
    let (terms, ids) = merge(s, 4096, 32);
    let seq = sequences(&ids, terms.len());
    let mut out = vec![43, 1];
    let literals = pack_runs(&ids, seq, &mut out);
    out.extend(pack_words(s, &terms, &literals));
    out
}

fn pack_runs(ids: &[u32], seq: Vec<(usize, usize, usize)>, out: &mut Vec<u8>) -> Vec<u32> {
    let mut anchor = 0;
    let mut literals = Vec::new();
    let mut tags = vec![];
    let mut ex = vec![];
    let mut distance = Numbers::default();
    for (at, len, off) in seq {
        let ll = at - anchor;
        let ml = len - 2;
        tags.push((ll.min(15) * 16 + ml.min(15)) as u8);
        if ll >= 15 {
            excess(&mut ex, ll - 15)
        }
        if ml >= 15 {
            excess(&mut ex, ml - 15)
        }
        distance.push((off - 1) as u32);
        literals.extend(&ids[anchor..at]);
        anchor = at + len;
    }
    literals.extend(&ids[anchor..]);
    out.extend((ids.len() as u32).to_le_bytes());
    ans::put_tight(&tags, out);
    ans::put_tight(&ex, out);
    distance.finish(out);
    literals
}

pub(crate) fn decode(mut p: &[u8], dst: &mut [u8]) -> Result<()> {
    if take(&mut p, 1)? != [1] {
        return Err(bad());
    }
    let cmds = Commands::read(&mut p, dst.len())?;
    let vocab = unpack(&mut p, dst.len())?;
    if !p.is_empty() {
        return Err(bad());
    }
    cmds.restore(vocab, dst)
}
struct Commands<'a> {
    nt: usize,
    tags: std::borrow::Cow<'a, [u8]>,
    ex: std::borrow::Cow<'a, [u8]>,
    distances: Vec<u32>,
}
impl<'a> Commands<'a> {
    fn read(p: &mut &'a [u8], limit: usize) -> Result<Self> {
        let nt = u32in(p)?;
        if nt > limit {
            return Err(bad());
        }
        let tags = crate::wordbase::plain(p, nt / 2)?;
        let ex = crate::wordbase::plain(p, nt)?;
        let distances = numbers(p, tags.len())?;
        if distances.len() != tags.len() {
            return Err(bad());
        }
        Ok(Self {
            nt,
            tags,
            ex,
            distances,
        })
    }
    fn restore(self, mut v: Vocabulary, dst: &mut [u8]) -> Result<()> {
        let Self {
            nt,
            tags,
            ex,
            distances,
        } = self;
        let mut ex = &ex[..];
        let lit = v.tokens.len();
        v.tokens.extend([0; 8]);
        let mut ids = vec![0u32; nt + 8];
        let (mut at, mut li) = (0, 0);
        for (ts, ds) in tags.chunks(32).zip(distances.chunks(32)) {
            let mut prepared = [(0usize, 0usize, 0usize, 0usize, 0usize); 32];
            for (j, (&t, &dist)) in ts.iter().zip(ds).enumerate() {
                let mut ll = usize::from(t >> 4);
                let mut ml = usize::from(t & 15) + 2;
                if ll == 15 {
                    ll += crate::lz::lenin(&mut ex)?;
                }
                if ml == 17 {
                    ml += crate::lz::lenin(&mut ex)?;
                }
                let d = dist as usize + 1;
                if ll > lit - li || ll + ml > nt - at || d > at + ll {
                    return Err(bad());
                }
                prepared[j] = (at, li, ll, ml, d);
                #[cfg(target_arch = "aarch64")]
                unsafe {
                    core::arch::asm!("prfm pldl1keep, [{p}]",p=in(reg)ids.as_ptr().add(at+ll-d),options(readonly,nostack,preserves_flags));
                }
                at += ll + ml;
                li += ll;
            }
            for &(beg, l, ll, ml, d) in &prepared[..ts.len()] {
                if ll != 0 {
                    if ll <= 8 {
                        // Both descriptor buffers have eight padding entries.
                        unsafe {
                            std::ptr::copy_nonoverlapping(
                                v.tokens.as_ptr().add(l),
                                ids.as_mut_ptr().add(beg),
                                8,
                            );
                        }
                    } else {
                        ids[beg..beg + ll].copy_from_slice(&v.tokens[l..l + ll]);
                    }
                }
                let m = beg + ll;
                if ml <= 8 && d >= ml {
                    unsafe {
                        std::ptr::copy(ids.as_ptr().add(m - d), ids.as_mut_ptr().add(m), 8);
                    }
                } else {
                    let k = ml.min(d);
                    ids.copy_within(m - d..m - d + k, m);
                    let mut have = k;
                    while have < ml {
                        let k = have.min(ml - have);
                        ids.copy_within(m..m + k, m + have);
                        have += k;
                    }
                }
            }
        }
        if !ex.is_empty() || nt - at != lit - li {
            return Err(bad());
        }
        ids[at..nt].copy_from_slice(&v.tokens[li..lit]);
        expand(&ids[..nt], &v.spell, dst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tag 43, mode 1 fixture from the archived encoder.
    #[test]
    fn historical_phrase_block_matches_the_supported_wire_mode() {
        let plain =
            b"the river flows under the bridge; over the bridge the road follows the river.\n"
                .repeat(48);
        let golden = include_bytes!("../tests/fixtures/legacy-phrases.block");
        assert_eq!(native(&plain), golden.as_slice());
        let mut restored = vec![0; plain.len()];
        crate::decode(golden, &mut restored).unwrap();
        assert_eq!(restored, plain);
        let mut invalid = *golden;
        for mode in [0, 2, 3, 4, 5, 255] {
            invalid[1] = mode;
            assert!(crate::decode(&invalid, &mut restored).is_err());
        }
    }

    #[test]
    fn numeric_price_classes_preserve_boundaries() {
        let vals = [
            0,
            1,
            15,
            16,
            17,
            31,
            32,
            63,
            255,
            256,
            1024,
            (1 << 24) - 1,
            1 << 24,
            (1 << 25) - 1,
        ];
        let mut ns = Numbers::default();
        for v in vals {
            ns.push(v);
        }
        let mut encoded = vec![];
        ns.finish(&mut encoded);
        let mut p = encoded.as_slice();
        assert_eq!(numbers(&mut p, vals.len()).unwrap(), vals);
        let mut z = Numbers::default();
        let mut rnd = 123u32;
        let mut all = vec![];
        for j in 0..997 {
            rnd = rnd.wrapping_mul(1664525).wrapping_add(1013904223);
            let v = rnd >> ((j % 29) + 3);
            all.push(v);
            z.push(v);
        }
        let mut data = vec![];
        z.finish(&mut data);
        let mut r = data.as_slice();
        assert_eq!(numbers(&mut r, all.len()).unwrap(), all);
        assert!(r.is_empty());
        assert!(numbers(&mut &data[..data.len() - 1], all.len()).is_err());
        assert!(p.is_empty());
    }
}

mod wordpipe;

pub(crate) mod worddag;
