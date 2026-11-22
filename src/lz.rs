use crate::{Result, ans, bad, take};
#[inline(always)]
pub(crate) fn word(s: &[u8], i: usize) -> u64 {
    u64::from_le_bytes(s[i..i + 8].try_into().unwrap())
}
#[inline(always)]
pub(crate) fn common(s: &[u8], mut a: usize, mut b: usize) -> usize {
    let start = b;
    while b + 8 <= s.len() {
        let d = word(s, a) ^ word(s, b);
        if d != 0 {
            return b - start + d.trailing_zeros() as usize / 8;
        }
        a += 8;
        b += 8
    }
    while b < s.len() && s[a] == s[b] {
        a += 1;
        b += 1
    }
    b - start
}
const MASK: usize = (1 << 21) - 1;
#[derive(Clone, Copy)]
struct Key {
    a: usize,
    b: usize,
    sig: u32,
}
#[inline(always)]
fn hashes(w: u64) -> Key {
    let h = (w as u32).wrapping_mul(0x9e3779b1);
    Key {
        a: (h >> 16) as usize,
        b: ((w << 8).wrapping_mul(0xcf1bbcdcb7a56463) >> 48) as usize,
        sig: h << 21,
    }
}
#[inline(always)]
fn probe(s: &[u8], i: usize, w: u64, slot: u32, sig: u32) -> (usize, usize) {
    let pos = slot as usize & MASK;
    if (slot ^ sig) > MASK as u32 || pos >= i {
        return (0, 0);
    }
    // Stored positions and i have eight readable bytes; pos is strictly earlier.
    let d = unsafe { std::ptr::read_unaligned(s.as_ptr().add(pos).cast::<u64>()) }.to_le() ^ w;
    if d != 0 {
        return ((d.trailing_zeros() / 8) as usize, pos);
    }
    (8 + common(s, pos + 8, i + 8), pos)
}
fn lenout(mut n: usize, v: &mut Vec<u8>) {
    while n >= 128 {
        v.push(n as u8 | 128);
        n >>= 7
    }
    v.push(n as u8)
}
#[inline(always)]
pub(crate) fn lenin(v: &mut &[u8]) -> Result<usize> {
    let mut n = 0;
    for shift in (0..=21).step_by(7) {
        let b = take(v, 1)?[0];
        n |= ((b & 127) as usize) << shift;
        if b < 128 {
            return Ok(n);
        }
    }
    Err(bad())
}
pub(crate) struct Streams {
    tag: Vec<u8>,
    offs: Vec<u8>,
    lits: Vec<u8>,
    lens: Vec<u8>,
    low: Vec<u8>,
    mid: Vec<u8>,
    far: Vec<u8>,
    pub(crate) rep: [usize; 3],
    minimum: usize,
    // Histogram and rank planner for these streams only.
    entropy: (usize, usize),
}
impl Streams {
    fn new() -> Self {
        Self {
            tag: vec![],
            offs: vec![],
            lits: vec![],
            lens: vec![],
            low: vec![],
            mid: vec![],
            far: vec![],
            rep: [1, 4, 8],
            minimum: 4,
            entropy: (0, 0),
        }
    }
    pub(crate) fn sized(n: usize) -> Self {
        let mut s = Self::new();
        s.tag.reserve(n / 8);
        s.offs.reserve(n / 8);
        s.lits.reserve(n / 5 + 16);
        s.low.reserve(n / 8 + 16);
        s.mid.reserve(n / 32 + 16);
        s
    }
    #[inline(always)]
    pub(crate) fn emit(&mut self, s: &[u8], anchor: usize, i: usize, best: usize, off: usize) {
        let ll = i - anchor;
        let ml = best - self.minimum;
        self.tag.push(((ll.min(15) << 4) | ml.min(15)) as u8);
        if ll >= 15 {
            lenout(ll - 15, &mut self.lens)
        }
        if ml >= 15 {
            lenout(ml - 15, &mut self.lens)
        }
        if ll != 0 {
            if ll <= 8 && anchor + 8 <= s.len() && self.lits.capacity() - self.lits.len() >= 8 {
                let n = self.lits.len();
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        s.as_ptr().add(anchor),
                        self.lits.as_mut_ptr().add(n),
                        8,
                    );
                    self.lits.set_len(n + ll);
                }
            } else {
                self.lits.extend_from_slice(&s[anchor..i]);
            }
        }
        self.offset(off);
    }
    #[inline(always)]
    fn offset(&mut self, off: usize) {
        if off == self.rep[0] {
            self.offs.push(0)
        } else if off == self.rep[1] {
            self.offs.push(1)
        } else if off == self.rep[2] {
            self.offs.push(2)
        } else {
            let delta = off.wrapping_sub(self.rep[0]).wrapping_add(8);
            if delta <= 16 {
                self.offs.push(3 + delta as u8)
            } else if off <= 64 {
                self.offs.push(19 + off as u8)
            } else if off + 19 < 32768 {
                let v = off + 19;
                self.offs.push(84 + (v >> 8) as u8);
                self.low.push(v as u8)
            } else {
                let v = off + 19;
                let high = v >> 16;
                self.offs.push(212 + high.min(43) as u8);
                if high >= 43 {
                    lenout(high - 43, &mut self.far);
                }
                self.low.push(v as u8);
                self.mid.push((v >> 8) as u8)
            }
        }
        self.rep = [off, self.rep[0], self.rep[1]];
    }
    pub(crate) fn finish(self, s: &[u8], anchor: usize, archive: bool) -> Vec<u8> {
        self.finish_policy(s, anchor, archive, false)
    }
    fn finish_policy(self, s: &[u8], anchor: usize, archive: bool, tight: bool) -> Vec<u8> {
        self.finish_literals(s, anchor, archive, tight, None)
    }
    pub(crate) fn finish_literals(
        self,
        s: &[u8],
        anchor: usize,
        archive: bool,
        tight: bool,
        custom: Option<&[u8]>,
    ) -> Vec<u8> {
        self.finish_coded::<0>(s, anchor, archive, tight, custom)
    }
    fn finish_coded<const PACK: u8>(
        mut self,
        s: &[u8],
        anchor: usize,
        archive: bool,
        tight: bool,
        custom: Option<&[u8]>,
    ) -> Vec<u8> {
        self.lits.extend(&s[anchor..]);

        if PACK == 3
            && self.tag.len() >= 2 * pages::COMMANDS
            && let Some(literals) = custom
        {
            return pages::encode::<{ pages::COMMANDS }>(&self, literals);
        }
        self.low.extend([0; 16]);
        self.mid.extend([0; 16]);
        let extended = !self.far.is_empty();
        let mut out = vec![match (self.minimum == 3, extended) {
            (true, true) => 17,
            (true, false) => 16,
            (false, true) => 13,
            (false, false) => 1,
        }];
        for (stream, v) in [
            &self.tag, &self.offs, &self.lens, &self.lits, &self.low, &self.mid,
        ]
        .into_iter()
        .enumerate()
        {
            if stream == 3
                && let Some(payload) = custom
            {
                out.extend(payload);
            } else if PACK != 0 && (stream <= 1 || (PACK == 2 && stream == 2)) {
                ans::put_alternative(v, &mut out, 3);
            } else if tight {
                ans::put_tight(v, &mut out)
            } else {
                ans::put_config(
                    v,
                    &mut out,
                    if archive { 0.90 } else { 0.985 },
                    0,
                    self.entropy,
                )
            }
        }
        if extended {
            ans::put_config(&self.far, &mut out, 0.985, 0, self.entropy);
        }
        out
    }
}
pub fn encode<const ARCHIVE: bool>(s: &[u8]) -> Vec<u8> {
    legacy::<ARCHIVE, 16, 16, false, 0>(s)
}
pub(crate) fn legacy<
    const ARCHIVE: bool,
    const SHORT: u32,
    const LONG: u32,
    const TWICE: bool,
    const FILL: usize,
>(
    s: &[u8],
) -> Vec<u8> {
    legacy_ext::<ARCHIVE, SHORT, LONG, TWICE, FILL, 0>(s)
}
pub(crate) fn legacy_ext<
    const ARCHIVE: bool,
    const SHORT: u32,
    const LONG: u32,
    const TWICE: bool,
    const FILL: usize,
    const EXTRA: u32,
>(
    s: &[u8],
) -> Vec<u8> {
    legacy_width::<ARCHIVE, SHORT, LONG, TWICE, FILL, EXTRA, 21>(s)
}
pub(crate) fn legacy_width<
    const ARCHIVE: bool,
    const SHORT: u32,
    const LONG: u32,
    const TWICE: bool,
    const FILL: usize,
    const EXTRA: u32,
    const PB: u32,
>(
    s: &[u8],
) -> Vec<u8> {
    legacy_policy::<ARCHIVE, SHORT, LONG, TWICE, FILL, EXTRA, PB, false, 0>(s)
}

// Distant matches need more offset bytes and can miss the decoder cache.
#[inline(always)]
fn history_value(len: usize, off: usize, repeat: usize) -> isize {
    if len < 4 {
        return -200;
    }
    let cost = if off == repeat {
        0
    } else {
        3 + 5 * (off >= 65) as isize
            + 3 * (off >= 1024) as isize
            + 6 * (off >= 32749) as isize
            + 3 * (off >= 262144) as isize
            + 4 * (off >= 2818029) as isize
    };
    5 * len as isize - cost
}
pub(crate) fn legacy_policy<
    const ARCHIVE: bool,
    const SHORT: u32,
    const LONG: u32,
    const TWICE: bool,
    const FILL: usize,
    const EXTRA: u32,
    const PB: u32,
    const COST: bool,
    const GATE: u32,
>(
    s: &[u8],
) -> Vec<u8> {
    history_model::<
        ARCHIVE,
        SHORT,
        LONG,
        TWICE,
        FILL,
        EXTRA,
        PB,
        COST,
        GATE,
        0,
        false,
        false,
        0,
        false,
    >(s)
}

// Maximum distance by match length (4 through 12). Repeat distances are exempt.
#[inline(always)]
fn remote_short<const LIMIT: u8>(len: usize, off: usize, rep: usize) -> bool {
    if LIMIT == 0 || !(4..13).contains(&len) || off == rep {
        return false;
    }
    let powers = if LIMIT == 1 {
        [14, 18, 21, 23, 24, 25, 25, 25, 25]
    } else {
        [13, 17, 19, 21, 22, 23, 24, 25, 25]
    };
    off > 1usize << powers[len - 4]
}
#[inline(always)]
fn local_price<const LIMIT: u8>(len: usize, off: usize, rep: usize) -> isize {
    if remote_short::<LIMIT>(len, off, rep) {
        return -180;
    }
    let price = history_value(len, off, rep);
    if LIMIT != 0 && off != rep {
        price
            - 3 * ((off >= 262_144) as isize
                + (off >= 1_048_576) as isize
                + (off >= 4_194_304) as isize)
    } else {
        price
    }
}
pub(crate) fn history_model<
    const ARCHIVE: bool,
    const SHORT: u32,
    const LONG: u32,
    const TWICE: bool,
    const FILL: usize,
    const EXTRA: u32,
    const PB: u32,
    const COST: bool,
    const GATE: u32,
    const LIMIT: u8,
    const CONTEXT: bool,
    const DISTINCT: bool,
    const PACK: u8,
    const NINTH: bool,
>(
    s: &[u8],
) -> Vec<u8> {
    #[inline(always)]
    fn wide_probe<const PB: u32>(
        s: &[u8],
        i: usize,
        w: u64,
        slot: u32,
        sig: u32,
    ) -> (usize, usize) {
        if PB == 21 {
            return probe(s, i, w, slot, sig);
        }
        let mask = (1u32 << PB) - 1;
        let pos = (slot & mask) as usize;
        if (slot ^ sig) > mask || pos >= i {
            return (0, 0);
        }
        let d = word(s, pos) ^ w;
        if d != 0 {
            return (d.trailing_zeros() as usize / 8, pos);
        }
        (8 + common(s, pos + 8, i + 8), pos)
    }
    #[inline(always)]
    fn superkey<const B: u32>(s: &[u8], i: usize, w: u64) -> usize {
        let x = u32::from_le_bytes(s[i + 8..i + 12].try_into().unwrap()) as u64;
        ((w.wrapping_mul(0x9e3779b97f4a7c15) ^ x.wrapping_mul(0xa24baed4963ee407)) >> (64 - B))
            as usize
    }
    // The nine-byte index separates seven-byte collisions that differ before byte twelve.
    #[inline(always)]
    fn nine(w: u64, ninth: u8) -> usize {
        ((w ^ (ninth as u64).wrapping_mul(0xa24baed4963ee407)).wrapping_mul(0x9e3779b97f4a7c15)
            >> (64 - 21)) as usize
    }
    let mut h9 = if NINTH {
        vec![0u32; 1 << 21]
    } else {
        Vec::new()
    };
    let mut ext = if EXTRA != 0 {
        vec![0u32; 1 << EXTRA]
    } else {
        vec![]
    };
    #[inline(always)]
    fn hash<const SHORT: u32, const LONG: u32, const PB: u32>(w: u64) -> Key {
        let h = (w as u32).wrapping_mul(0x9e3779b1);
        Key {
            a: (h >> (32 - SHORT)) as usize,
            b: ((w << 8).wrapping_mul(0xcf1bbcdcb7a56463) >> (64 - LONG)) as usize,
            sig: h << PB,
        }
    }
    let mut older = if TWICE {
        vec![0u32; 1 << SHORT]
    } else {
        vec![]
    };
    let mut h4 = vec![0u32; 1 << SHORT];
    let mut h7 = vec![0u64; 1 << LONG];
    let mut streams = Streams::sized(s.len());
    let mut i = 0;
    let mut anchor = 0;
    while i + 8 <= s.len() {
        let w = word(s, i);
        let k = hash::<SHORT, LONG, PB>(w);
        let here = i as u32 | k.sig;
        let p4 = std::mem::replace(&mut h4[k.a], here);
        let old = h7[k.b];
        h7[k.b] = (old << 32) | here as u64;
        let p7 = old as u32;
        let (m7, q7) = wide_probe::<PB>(s, i, w, p7, k.sig);
        let (m4, q4) = wide_probe::<PB>(s, i, w, p4, k.sig);
        let rep = streams.rep[0];
        let better = |len, q, prev, p, nearest_tie| {
            if COST {
                if len < 4 {
                    return false;
                }
                if prev < 4 {
                    return true;
                }
                !remote_short::<LIMIT>(len, i - q, rep)
                    && ((len == prev && q > p)
                        || local_price::<LIMIT>(len, i - q, rep)
                            > local_price::<LIMIT>(prev, i - p, rep))
            } else {
                len > prev || (nearest_tie && len == prev && q > p)
            }
        };
        let (mut best, mut pos) = if better(m4, q4, m7, q7, true) {
            (m4, q4)
        } else {
            (m7, q7)
        };
        let (m, q) = wide_probe::<PB>(s, i, w, (old >> 32) as u32, k.sig);
        if better(m, q, best, pos, false) {
            best = m;
            pos = q
        }
        // History and lookahead must index the same short-prefix buckets.
        if EXTRA != 0 && k.a & ((1 << GATE) - 1) == 0 && i + 12 <= s.len() {
            let e = std::mem::replace(&mut ext[superkey::<EXTRA>(s, i, w)], here);
            let (l, q) = wide_probe::<PB>(s, i, w, e, k.sig);
            if l > best {
                best = l;
                pos = q;
            }
        }
        if TWICE {
            let candidate = std::mem::replace(&mut older[k.a], p4);
            let (l, q) = wide_probe::<PB>(s, i, w, candidate, k.sig);
            if l > best {
                best = l;
                pos = q;
            }
        }
        if NINTH && i + 9 <= s.len() {
            let e = std::mem::replace(&mut h9[nine(w, s[i + 8])], here);
            let (l, q) = wide_probe::<PB>(s, i, w, e, k.sig);
            if better(l, q, best, pos, true) {
                best = l;
                pos = q;
            }
        }
        if i >= streams.rep[0] {
            let (l, p) = wide_probe::<PB>(s, i, w, (i - streams.rep[0]) as u32, 0);
            if l >= 4 && l + 1 >= best {
                best = l;
                pos = p
            }
        }
        if best < 4 {
            i += 1;
            continue;
        }
        if best < 40 && i + 9 <= s.len() {
            let w1 = word(s, i + 1);
            let k = hash::<SHORT, LONG, PB>(w1);
            let (l7, p7) = wide_probe::<PB>(s, i + 1, w1, h7[k.b] as u32, k.sig);
            let (x, q) = wide_probe::<PB>(s, i + 1, w1, (h7[k.b] >> 32) as u32, k.sig);
            let (l7, p7) = if x > l7 { (x, q) } else { (l7, p7) };
            let (l7, p7) = if EXTRA != 0 && k.a & ((1 << GATE) - 1) == 0 && i + 13 <= s.len() {
                let (l, q) =
                    wide_probe::<PB>(s, i + 1, w1, ext[superkey::<EXTRA>(s, i + 1, w1)], k.sig);
                if l > l7 { (l, q) } else { (l7, p7) }
            } else {
                (l7, p7)
            };
            let wait = |len, q| {
                if COST {
                    len >= 4
                        && local_price::<LIMIT>(len, i + 1 - q, rep)
                            > local_price::<LIMIT>(best, i - pos, rep) + 2
                } else {
                    len > best
                }
            };
            if wait(l7, p7) {
                i += 1;
                best = l7;
                pos = p7
            } else {
                let (l4, p4) = wide_probe::<PB>(s, i + 1, w1, h4[k.a], k.sig);
                if wait(l4, p4) {
                    i += 1;
                    best = l4;
                    pos = p4
                }
            }
        }
        if !ARCHIVE && best < 10 && i + 10 <= s.len() {
            let w2 = word(s, i + 2);
            let k = hash::<SHORT, LONG, PB>(w2);
            let (l, q) = wide_probe::<PB>(s, i + 2, w2, h7[k.b] as u32, k.sig);
            if l > best + 2 {
                i += 2;
                best = l;
                pos = q
            }
        }
        let off = i - pos;
        if remote_short::<LIMIT>(best, off, rep) {
            i += 1;
            continue;
        }
        if ((ARCHIVE && best < 6 && off > 256) || (!ARCHIVE && best == 4 && off > 16384))
            && off != streams.rep[0]
        {
            i += 1;
            continue;
        }
        while i > anchor && pos > 0 && s[i - 1] == s[pos - 1] {
            i -= 1;
            pos -= 1;
            best += 1
        }
        streams.emit(s, anchor, i, best, off);
        let end = i + best;
        if FILL > 0 && end >= i + 10 {
            let mut j = i + 4;
            while j + 3 < end && j + 8 <= s.len() {
                let k = hash::<SHORT, LONG, PB>(word(s, j));
                let slot = j as u32 | k.sig;
                if TWICE {
                    older[k.a] = h4[k.a];
                }
                if EXTRA != 0 && k.a & ((1 << GATE) - 1) == 0 && j + 12 <= s.len() {
                    ext[superkey::<EXTRA>(s, j, word(s, j))] = slot;
                }
                if NINTH && j + 9 <= s.len() {
                    h9[nine(word(s, j), s[j + 8])] = slot;
                }
                h4[k.a] = slot;
                h7[k.b] = (h7[k.b] << 32) | slot as u64;
                j += FILL;
            }
        }
        for (which, j) in [i + 1, i + 2, i + 3, end - 3, end - 2, end - 1]
            .into_iter()
            .enumerate()
        {
            if DISTINCT && which >= 3 && j <= i + 3 {
                continue;
            }
            if j + 8 <= s.len() {
                let k = hash::<SHORT, LONG, PB>(word(s, j));
                let slot = j as u32 | k.sig;
                if TWICE {
                    older[k.a] = h4[k.a];
                }
                if EXTRA != 0 && k.a & ((1 << GATE) - 1) == 0 && j + 12 <= s.len() {
                    ext[superkey::<EXTRA>(s, j, word(s, j))] = slot;
                }
                if NINTH && j + 9 <= s.len() {
                    h9[nine(word(s, j), s[j + 8])] = slot;
                }
                h4[k.a] = slot;
                h7[k.b] = (h7[k.b] << 32) | slot as u64
            }
        }
        i = end;
        anchor = end;
    }
    if CONTEXT && !ARCHIVE && streams.lits.len() >= 16_384 {
        streams.lits.extend_from_slice(&s[anchor..]);
        let payload = crate::context::candidate(&streams.lits, None);
        streams.finish_coded::<PACK>(s, s.len(), false, false, Some(&payload))
    } else {
        streams.finish_coded::<PACK>(s, anchor, ARCHIVE, false, None)
    }
}
pub(crate) fn eighth_text(s: &[u8]) -> Vec<u8> {
    history_model::<false, 18, 21, false, 1, 18, 25, true, 2, 1, false, true, 0, false>(s)
}
pub(crate) fn middle_text(s: &[u8], level: u8) -> Vec<u8> {
    // Level 10 indexes all 12-byte prefixes. Level 11 retains a second short-prefix
    // candidate and entropy-codes excess lengths.
    match level {
        9 => history_model::<false, 18, 21, false, 1, 19, 25, true, 1, 0, true, true, 3, false>(s),
        10 => history_model::<false, 19, 21, false, 1, 22, 25, true, 0, 0, true, true, 1, false>(s),
        11 => history_model::<false, 20, 22, true, 1, 22, 25, true, 0, 0, true, true, 2, false>(s),
        _ => history_model::<false, 19, 21, true, 1, 22, 25, true, 0, 0, true, true, 2, true>(s),
    }
}
pub fn encode_level<const ARCHIVE: bool>(s: &[u8], level: u8) -> Vec<u8> {
    match level {
        1 => quick::<1>(s, ARCHIVE),
        2 => quick::<2>(s, ARCHIVE),
        3 => encode::<ARCHIVE>(s),
        4 if !ARCHIVE => legacy::<false, 17, 17, false, 0>(s),
        _ => dense::<false>(s, level, ARCHIVE),
    }
}
fn quick<const Q: u8>(s: &[u8], archive: bool) -> Vec<u8> {
    quick_core::<Q>(s, archive)
}
fn quick_core<const Q: u8>(s: &[u8], archive: bool) -> Vec<u8> {
    if Q == 1 {
        #[cfg(target_arch = "aarch64")]
        if s.len() <= crate::MAX_BLOCK {
            return if archive {
                emitoffset::archive(s)
            } else {
                emitoffset::text(s)
            };
        }
        return quicktwo::encode(s, archive);
    }
    quick_impl::<Q, 15, false, 2>(s, archive)
}

pub(crate) fn quick_impl<const Q: u8, const B: u32, const LAZY: bool, const TAIL: usize>(
    s: &[u8],
    archive: bool,
) -> Vec<u8> {
    #[inline(always)]
    fn key<const Q: u8, const B: u32>(w: u64) -> usize {
        let (shift, mul) = if Q == 1 {
            (24, 0x9e3779b97f4a7c15)
        } else {
            (8, 0xcf1bbcdcb7a56463)
        };
        ((w << shift).wrapping_mul(mul) >> (64 - B)) as usize
    }
    #[inline(always)]
    fn sk<const B: u32>(w: u64) -> usize {
        ((w as u32).wrapping_mul(0x9e3779b1) >> (32 - B)) as usize
    }
    let mut tab = vec![0u32; 1 << B];
    let mut short = if Q == 2 { vec![0u32; 1 << B] } else { vec![] };
    let mut streams = Streams::sized(s.len());
    let mut i = 0;
    let mut anchor = 0;
    while i + 8 <= s.len() {
        let w = word(s, i);
        let k = hashes(w);
        let sig = k.sig;
        let idx = key::<Q, B>(w);
        let prev = std::mem::replace(&mut tab[idx], sig | i as u32);
        let (mut len, mut pos) = probe(s, i, w, prev, sig);
        if Q == 2 {
            let old = std::mem::replace(&mut short[sk::<B>(w)], sig | i as u32);
            let (l, p) = probe(s, i, w, old, sig);
            if l > len {
                (len, pos) = (l, p);
            }
        }
        if len < 4 || (len == 4 && i - pos > 12_000) {
            i += 1;
            continue;
        }
        if LAZY && len < 20 && i + 9 <= s.len() {
            let w1 = word(s, i + 1);
            let sig = hashes(w1).sig;
            let (l, p) = probe(s, i + 1, w1, tab[key::<Q, B>(w1)], sig);
            if l > len {
                i += 1;
                len = l;
                pos = p;
            }
        }
        let off = i - pos;
        while i > anchor && pos > 0 && s[i - 1] == s[pos - 1] {
            i -= 1;
            pos -= 1;
            len += 1;
        }
        streams.emit(s, anchor, i, len, off);
        i += len;
        anchor = i;
        for j in i.saturating_sub(TAIL)..i {
            if j + 8 <= s.len() {
                let w = word(s, j);
                let k = hashes(w);
                let idx = key::<Q, B>(w);
                tab[idx] = k.sig | j as u32;
                if Q == 2 {
                    short[sk::<B>(w)] = k.sig | j as u32;
                }
            }
        }
    }
    streams.finish(s, anchor, archive)
}

// Chained searches use two prefix lengths. Long keys reach older phrases
// without walking all the nearby short-prefix collisions.
struct Search<'a> {
    s: &'a [u8],
    head: Vec<u32>,
    long: Vec<u32>,
    prev: Vec<u32>,
    back: Vec<u32>,
    bits: usize,
    depth: usize,
    nice: usize,
}
impl<'a> Search<'a> {
    fn new(s: &'a [u8], level: u8) -> Self {
        let bits = if level < 7 {
            16
        } else if level < 12 {
            17
        } else {
            18
        };
        let depth = [
            2, 3, 4, 6, 8, 10, 12, 16, 20, 24, 32, 40, 48, 64, 80, 96, 128, 160, 224,
        ][(level - 4) as usize];
        Self {
            s,
            head: vec![0; 1 << bits],
            long: vec![0; 1 << bits],
            prev: vec![0; s.len()],
            back: vec![0; s.len()],
            bits,
            depth,
            nice: 24 + level as usize * 3,
        }
    }
    #[inline(always)]
    fn keys(&self, w: u64) -> (usize, usize) {
        (
            ((w as u32).wrapping_mul(0x9e3779b1) >> (32 - self.bits)) as usize,
            ((w << 8).wrapping_mul(0x9e3779b97f4a7c15) >> (64 - self.bits)) as usize,
        )
    }
    #[inline(always)]
    fn add(&mut self, i: usize) {
        let (a, b) = self.keys(word(self.s, i));
        self.prev[i] = std::mem::replace(&mut self.head[a], i as u32 + 1);
        self.back[i] = std::mem::replace(&mut self.long[b], i as u32 + 1);
    }
    #[inline]
    fn find(&self, i: usize) -> (usize, usize) {
        let s = self.s;
        let w = word(s, i);
        let (a, b) = self.keys(w);
        let mut best = 3usize;
        let mut pos = i;
        let mut score = 0isize;
        for (mut next, chain, limit) in [
            (self.long[b], &self.back, self.depth.div_ceil(2)),
            (self.head[a], &self.prev, self.depth),
        ] {
            for _ in 0..limit {
                if next == 0 {
                    break;
                }
                let j = (next - 1) as usize;
                if i + best < s.len() && s[j + best] == s[i + best] {
                    let x = w ^ word(s, j);
                    let len = if x == 0 {
                        8 + common(s, j + 8, i + 8)
                    } else {
                        x.trailing_zeros() as usize / 8
                    };
                    if len >= 4 {
                        let v = (len as isize) * 6 - (i - j).ilog2() as isize;
                        if v > score {
                            best = len;
                            pos = j;
                            score = v;
                            if best >= self.nice {
                                return (best, pos);
                            }
                        }
                    }
                }
                next = chain[j];
            }
        }
        if pos == i { (0, i) } else { (best, pos) }
    }
}
pub(crate) fn dense<const TIGHT: bool>(s: &[u8], level: u8, archive: bool) -> Vec<u8> {
    let mut search = Search::new(s, level);
    let mut streams = Streams::sized(s.len());
    let mut anchor = 0;
    let mut i = 0;
    let mut indexed = 0;
    while i + 8 <= s.len() {
        while indexed < i {
            search.add(indexed);
            indexed += 1;
        }
        let (mut len, mut pos) = search.find(i);
        if len < 4 {
            i += 1;
            continue;
        }
        let mut score = 6 * len as isize - (i - pos).ilog2() as isize;
        let ahead = if level < 6 {
            1
        } else if level < 12 {
            2
        } else {
            3
        };
        let base = i;
        if len < search.nice {
            for k in 1..=ahead {
                let at = base + k;
                if at + 8 > s.len() {
                    break;
                }
                while indexed < at {
                    search.add(indexed);
                    indexed += 1;
                }
                let (l, p) = search.find(at);
                if l < 4 {
                    continue;
                }
                let value = (l as isize - k as isize) * 6 - (at - p).ilog2() as isize;
                if value > score {
                    score = value;
                    i = at;
                    len = l;
                    pos = p;
                }
            }
        }
        let off = i - pos;
        while i > anchor && pos > 0 && s[i - 1] == s[pos - 1] {
            i -= 1;
            pos -= 1;
            len += 1;
        }
        streams.emit(s, anchor, i, len, off);
        i += len;
        anchor = i;
    }
    streams.finish_policy(s, anchor, archive, TIGHT)
}
const SHUF: [[u8; 16]; 17] = {
    let mut v = [[0; 16]; 17];
    let mut l = 0;
    while l < 17 {
        let mut j = 0;
        while j < 16 {
            v[l][j] = if j < l { j as u8 } else { (16 + j - l) as u8 };
            j += 1
        }
        l += 1
    }
    v
};
const BATCH: usize = 64;

const BOOK_BIT: u32 = 1 << 31;
#[derive(Clone)]
struct Fields {
    src: [u32; BATCH],
    dst: [u32; BATCH],
    lit: [u32; BATCH],
    ll: [u32; BATCH],
    ml: [u32; BATCH],
}
impl Fields {
    fn new() -> Self {
        Self {
            src: [0; BATCH],
            dst: [0; BATCH],
            lit: [0; BATCH],
            ll: [0; BATCH],
            ml: [0; BATCH],
        }
    }
}
#[cfg(target_arch = "aarch64")]
#[inline(always)]
#[allow(clippy::too_many_arguments)] // Hot loop over separate, bounded wire streams.
unsafe fn prep16<const MIN: usize, const BOOK: bool>(
    f: &mut Fields,
    k: usize,
    t: *const u8,
    o: *const u8,
    abs: &mut [u32; BATCH + 3],
    lens: &mut &[u8],
    at: &mut usize,
    lit: &mut usize,
    n: usize,
    book_base: u32,
    entries: u32,
) -> Result<()> {
    use std::arch::aarch64::*;
    unsafe {
        let t0 = vld1q_u8(t);
        let o0 = vld1q_u8(o);
        let lm = vshrq_n_u8::<4>(t0);
        let mm = vandq_u8(t0, vdupq_n_u8(15));
        let big = vceqq_u8(vmaxq_u8(lm, mm), vdupq_n_u8(15));
        let special = vcltq_u8(o0, vdupq_n_u8(20));
        let mask = |v| {
            vget_lane_u64::<0>(vreinterpret_u64_u8(vshrn_n_u16::<4>(vreinterpretq_u16_u8(
                v,
            )))) & 0x1111111111111111
        };
        let mut m = mask(special);
        while m != 0 {
            let j = k + 3 + (m.trailing_zeros() / 4) as usize;
            m &= m - 1;
            let c = abs[j];
            abs[j] = if c < 3 {
                abs[j - 1 - c as usize]
            } else {
                abs[j - 1].wrapping_add(c).wrapping_sub(11)
            };
        }
        macro_rules! widen {
            ($v:expr,$dst:expr) => {{
                let lo = vmovl_u8(vget_low_u8($v));
                let hi = vmovl_high_u8($v);
                let ptr = $dst;
                vst1q_u32(ptr, vmovl_u16(vget_low_u16(lo)));
                vst1q_u32(ptr.add(4), vmovl_high_u16(lo));
                vst1q_u32(ptr.add(8), vmovl_u16(vget_low_u16(hi)));
                vst1q_u32(ptr.add(12), vmovl_high_u16(hi));
            }};
        }
        widen!(lm, f.ll.as_mut_ptr().add(k));
        widen!(
            vaddq_u8(mm, vdupq_n_u8(MIN as u8)),
            f.ml.as_mut_ptr().add(k)
        );
        let mut m = mask(big);
        while m != 0 {
            let j = k + (m.trailing_zeros() / 4) as usize;
            m &= m - 1;
            if f.ll[j] == 15 {
                let e = lenin(lens)?;
                if e > n {
                    return Err(bad());
                }
                f.ll[j] += e as u32
            }
            if f.ml[j] == (15 + MIN) as u32 {
                let e = lenin(lens)?;
                if e > n {
                    return Err(bad());
                }
                f.ml[j] += e as u32
            }
        }
        let z = vdupq_n_u32(0);
        let mut dest = vdupq_n_u32(*at as u32);
        let mut lp = vdupq_n_u32(*lit as u32);
        let mut invalid = z;
        macro_rules! prefix {
            ($v:expr) => {{
                let v = $v;
                let v = vaddq_u32(v, vextq_u32::<3>(z, v));
                vaddq_u32(v, vextq_u32::<2>(z, v))
            }};
        }
        for h in (k..k + 16).step_by(4) {
            let ll = vld1q_u32(f.ll.as_ptr().add(h));
            let ml = vld1q_u32(f.ml.as_ptr().add(h));
            let end = vaddq_u32(dest, prefix!(vaddq_u32(ll, ml)));
            dest = vdupq_laneq_u32::<3>(end);
            let d = vsubq_u32(end, ml);
            let le = vaddq_u32(lp, prefix!(ll));
            lp = vdupq_laneq_u32::<3>(le);
            let off = vsubq_u32(vld1q_u32(abs.as_ptr().add(h + 3)), vdupq_n_u32(19));
            let source = if BOOK {
                let id = vsubq_u32(off, vdupq_n_u32(book_base));
                let in_book = vcltq_u32(id, vdupq_n_u32(entries));
                let shift = vandq_u32(
                    vcgeq_u32(off, vdupq_n_u32(book_base + entries)),
                    vdupq_n_u32(entries),
                );
                let back = vsubq_u32(off, shift);
                let range = vbslq_u32(in_book, vcgtq_u32(ml, vdupq_n_u32(16)), vcgtq_u32(back, d));
                invalid = vorrq_u32(invalid, vorrq_u32(range, vceqq_u32(off, z)));
                vbslq_u32(
                    in_book,
                    vorrq_u32(vshlq_n_u32::<4>(id), vdupq_n_u32(BOOK_BIT)),
                    vsubq_u32(d, back),
                )
            } else {
                invalid = vorrq_u32(invalid, vorrq_u32(vcgtq_u32(off, d), vceqq_u32(off, z)));
                vsubq_u32(d, off)
            };
            vst1q_u32(f.src.as_mut_ptr().add(h), source);
            vst1q_u32(f.dst.as_mut_ptr().add(h), d);
            vst1q_u32(f.lit.as_mut_ptr().add(h), vsubq_u32(le, ll));
        }
        *at = vgetq_lane_u32::<0>(dest) as usize;
        *lit = vgetq_lane_u32::<0>(lp) as usize;
        if vmaxvq_u32(invalid) != 0 || *at > n {
            return Err(bad());
        }
        Ok(())
    }
}
#[inline(always)]
pub(crate) unsafe fn copy16(s: *const u8, d: *mut u8) {
    #[cfg(target_arch = "aarch64")]
    unsafe {
        use std::arch::aarch64::*;
        vst1q_u8(d, vld1q_u8(s))
    }
    #[cfg(not(target_arch = "aarch64"))]
    unsafe {
        std::ptr::copy(s, d, 16)
    }
}
#[inline(never)]
unsafe fn long_literals(s: *const u8, d: *mut u8, n: usize) {
    unsafe { std::ptr::copy_nonoverlapping(s, d, n) }
}
#[inline(never)]
pub(crate) unsafe fn long_match(d: *mut u8, off: usize, ml: usize) {
    unsafe { copy_match(d, off, ml) }
}
#[inline(always)]
unsafe fn copy_match(d: *mut u8, off: usize, ml: usize) {
    use std::ptr::copy_nonoverlapping as cp;
    // Precondition: off <= produced; room for ml+16 bytes beyond d.
    unsafe {
        let s = d.sub(off);
        #[cfg(target_arch = "aarch64")]
        if off <= 16 {
            use std::arch::aarch64::*;
            const IX: [[u8; 16]; 17] = {
                let mut t = [[0; 16]; 17];
                let mut i = 1;
                while i <= 16 {
                    let mut j = 0;
                    while j < 16 {
                        t[i][j] = (j % i) as u8;
                        j += 1
                    }
                    i += 1
                }
                t
            };
            const STEP: [usize; 17] = [
                0, 16, 16, 15, 16, 15, 12, 14, 16, 9, 10, 11, 12, 13, 14, 15, 16,
            ];
            if off == 1 && ml >= 64 {
                std::ptr::write_bytes(d, *s, ml);
                return;
            }
            let v = vqtbl1q_u8(vld1q_u8(s), vld1q_u8(IX[off].as_ptr()));
            let mut a = 0;
            while a < ml {
                vst1q_u8(d.add(a), v);
                a += STEP[off]
            }
            return;
        }
        if off >= 16 {
            let mut k = 0;
            if off >= 64 && ml >= 128 {
                while k + 64 <= ml {
                    cp(s.add(k), d.add(k), 64);
                    k += 64
                }
            }
            while k < ml {
                cp(s.add(k), d.add(k), 16);
                k += 16
            }
        } else if off == 1 {
            std::ptr::write_bytes(d, *s, ml)
        } else {
            cp(s, d, off);
            let mut k = off;
            while k < ml {
                let step = k.min(ml - k);
                cp(d, d.add(k), step);
                k += step
            }
        }
    }
}
// Expand sixteen variable-width offsets without a branch for each width.
// Prefix sums give each lane its index in the two compact byte streams.
#[cfg(target_arch = "aarch64")]
#[inline(always)]
#[cfg(test)]
unsafe fn expand16<const FAR: bool>(
    c: *const u8,
    l: *const u8,
    m: *const u8,
    d: *mut u32,
    far: &mut &[u8],
    n: usize,
) -> Result<(usize, usize)> {
    unsafe { expand_book16::<FAR, false>(c, l, m, d, far, n, BookTags::default()) }
}
#[derive(Clone, Copy, Default)]
pub(crate) struct BookTags {
    direct: u8,
    ext: u8,
}
impl BookTags {
    pub(crate) fn direct_id(self, c: u8) -> bool {
        (self.direct..84).contains(&c)
    }
    pub(crate) fn extra_id(self, c: u8) -> bool {
        (self.ext..212).contains(&c)
    }
    fn mapped(self, c: u8, original: u32) -> u32 {
        if self.direct_id(c) {
            (1 << 28) + ((self.ext as u32 - 85) << 8) + c.wrapping_sub(84) as u32
        } else if self.extra_id(c) {
            (1 << 28) + original
        } else {
            original
        }
    }
}
#[cfg(target_arch = "aarch64")]
#[inline(always)]
#[allow(clippy::too_many_arguments)]
unsafe fn expand_book16<const FAR: bool, const TAGGED: bool>(
    c: *const u8,
    l: *const u8,
    m: *const u8,
    d: *mut u32,
    far: &mut &[u8],
    n: usize,
    codes: BookTags,
) -> Result<(usize, usize)> {
    use std::arch::aarch64::*;
    unsafe {
        let z = vdupq_n_u8(0);
        let c = vld1q_u8(c);
        let med = vcgeq_u8(c, vdupq_n_u8(84));
        let long = vcgeq_u8(c, vdupq_n_u8(212));
        macro_rules! scan {
            ($mask:expr) => {{
                let mut v = vshrq_n_u8::<7>($mask);
                v = vaddq_u8(v, vextq_u8::<15>(z, v));
                v = vaddq_u8(v, vextq_u8::<14>(z, v));
                v = vaddq_u8(v, vextq_u8::<12>(z, v));
                vaddq_u8(v, vextq_u8::<8>(z, v))
            }};
        }
        let a = scan!(med);
        let b = scan!(long);
        let ls = vqtbl1q_u8(vld1q_u8(l), vsubq_u8(a, vdupq_n_u8(1)));
        let ms = vqtbl1q_u8(vld1q_u8(m), vsubq_u8(b, vdupq_n_u8(1)));
        let x = vbslq_u8(med, ls, c);
        let y = vbslq_u8(long, ms, vandq_u8(med, vsubq_u8(c, vdupq_n_u8(84))));
        let mut h = vandq_u8(long, vsubq_u8(c, vdupq_n_u8(212)));
        // Most escape payloads fit one byte. Gather up to sixteen in one
        // instruction; entries that might be multibyte retain the scalar
        // varint path. Zero-mask lanes must not consume an escape byte.
        let mut sparse = 0;
        if FAR {
            let mask = vceqq_u8(c, vdupq_n_u8(255));
            sparse = vget_lane_u64::<0>(vreinterpret_u64_u8(vshrn_n_u16::<4>(
                vreinterpretq_u16_u8(mask),
            ))) & 0x1111111111111111;
            if sparse != 0 && far.len() >= 16 {
                let v = vld1q_u8(far.as_ptr());
                if vmaxvq_u8(v) < 128 {
                    let sum = scan!(mask);
                    let extra = vandq_u8(mask, vqtbl1q_u8(v, vsubq_u8(sum, vdupq_n_u8(1))));
                    if n >> 16 < 128 && vmaxvq_u8(extra) as usize > n >> 16 {
                        return Err(bad());
                    }
                    h = vaddq_u8(h, extra);
                    *far = &far[vgetq_lane_u8::<15>(sum) as usize..];
                    sparse = 0;
                }
            }
        }
        if TAGGED {
            let direct = vandq_u8(
                vcgeq_u8(c, vdupq_n_u8(codes.direct)),
                vcltq_u8(c, vdupq_n_u8(84)),
            );
            let ext = vandq_u8(
                vcgeq_u8(c, vdupq_n_u8(codes.ext)),
                vcltq_u8(c, vdupq_n_u8(212)),
            );
            let x = vbslq_u8(direct, vsubq_u8(c, vdupq_n_u8(84)), x);
            let y = vbslq_u8(direct, vdupq_n_u8(codes.ext - 85), y);
            let flag = vandq_u8(vorrq_u8(direct, ext), vdupq_n_u8(16));
            vst4q_u8(d.cast(), uint8x16x4_t(x, y, h, flag));
        } else {
            vst4q_u8(d.cast(), uint8x16x4_t(x, y, h, z));
        }
        while sparse != 0 {
            let j = (sparse.trailing_zeros() / 4) as usize;
            sparse &= sparse - 1;
            let extra = match far.split_first() {
                Some((&x, rest)) if x < 128 => {
                    *far = rest;
                    x as usize
                }
                _ => lenin(far)?,
            };
            if extra > n >> 16 {
                return Err(bad());
            }
            *d.add(j) += (extra as u32) << 16;
        }
        Ok((
            vgetq_lane_u8::<15>(a) as usize,
            vgetq_lane_u8::<15>(b) as usize,
        ))
    }
}
fn leaf_get<'a, const LEAF: bool>(
    p: &mut &'a [u8],
    n: usize,
) -> Result<std::borrow::Cow<'a, [u8]>> {
    if LEAF && !matches!(p.get(4), Some(0 | 1 | 2 | 4 | 5)) {
        return Err(bad());
    }
    ans::get(p, n)
}
pub fn decode(p: &[u8], dst: &mut [u8]) -> Result<()> {
    decode_impl::<false>(p, dst)
}
pub fn decode_impl<const EXTENDED: bool>(p: &[u8], dst: &mut [u8]) -> Result<()> {
    decode_min::<EXTENDED, 4>(p, dst)
}
pub fn decode_min<const EXTENDED: bool, const MIN: usize>(p: &[u8], dst: &mut [u8]) -> Result<()> {
    decode_core::<EXTENDED, MIN, false, false>(p, dst, &[])
}
pub(crate) fn decode_core<
    const EXTENDED: bool,
    const MIN: usize,
    const EXTERNAL: bool,
    const LEAF: bool,
>(
    p: &[u8],
    dst: &mut [u8],
    external: &[u8],
) -> Result<()> {
    decode_books::<EXTENDED, MIN, EXTERNAL, LEAF, false, false>(
        p,
        dst,
        external,
        &[],
        0,
        BookTags::default(),
    )
}
pub(crate) fn decode_books<
    const EXTENDED: bool,
    const MIN: usize,
    const EXTERNAL: bool,
    const LEAF: bool,
    const BOOK: bool,
    const TAGGED: bool,
>(
    p: &[u8],
    dst: &mut [u8],
    external: &[u8],
    book: &[[u8; 16]],
    book_base: u32,
    codes: BookTags,
) -> Result<()> {
    from_page::<EXTENDED, MIN, EXTERNAL, LEAF, BOOK, TAGGED, false, false>(
        p,
        dst,
        external,
        book,
        book_base,
        codes,
        &mut pages::Cursor::default(),
        [0; 3],
    )
}
#[derive(Clone, Copy, Default)]
struct Ready {
    start: usize,
    lstart: usize,
    at: usize,
    lit: usize,
    count: usize,
}
// Preparation checked every source and output range. Apply matches in output order.
#[inline(always)]
#[allow(clippy::too_many_arguments)]
fn restore_ready<const BOOK: bool>(
    dst: &mut [u8],
    lits: &[u8],
    book: &[[u8; 16]],
    f: &Fields,
    run: Ready,
    early: bool,
    prefetch: bool,
) {
    #[cfg(not(target_arch = "aarch64"))]
    let _ = (early, prefetch);
    let n = dst.len();
    let Ready {
        start,
        lstart,
        at,
        lit,
        ..
    } = run;
    if at + 16 <= n && lit + 16 <= lits.len() {
        unsafe {
            // All match positions for this batch have been validated. A
            // non-binding hint may also target a match produced later in
            // the batch; the actual copies still run in output order.
            #[cfg(target_arch = "aarch64")]
            if prefetch && !early {
                for (&p, &d) in f.src[..run.count].iter().zip(&f.dst) {
                    if (!BOOK || p < BOOK_BIT) && d - p >= 256 {
                        core::arch::asm!("prfm pldl1keep, [{p}]",p=in(reg)dst.as_ptr().add(p as usize),options(readonly,nostack,preserves_flags));
                    }
                }
            }

            // The vector and scalar preparation paths checked the source
            // ranges. Resolve dictionary indices before the copy loop.
            let mut pointers = [std::ptr::null(); BATCH];
            if BOOK {
                for (ptr, &s) in pointers[..run.count].iter_mut().zip(&f.src) {
                    *ptr = if s >= BOOK_BIT {
                        book.as_ptr().cast::<u8>().add((s - BOOK_BIT) as usize)
                    } else {
                        dst.as_ptr().add(s as usize)
                    };
                }
            }
            for (j, &literal_count) in f.ll[..run.count].iter().enumerate() {
                let ll = literal_count as usize;
                let ml = f.ml[j] as usize;
                let d = dst.as_mut_ptr().add(f.dst[j] as usize);
                let l = lits.as_ptr().add(f.lit[j] as usize);
                let off = f.dst[j].wrapping_sub(f.src[j]) as usize;
                let source = if BOOK {
                    pointers[j]
                } else {
                    dst.as_ptr().add(f.src[j] as usize)
                };
                #[cfg(target_arch = "aarch64")]
                if ll + ml <= 16 && off >= ll + ml {
                    use std::arch::aarch64::*;
                    let a = vld1q_u8(l);
                    let b = vld1q_u8(source);
                    let ix = vld1q_u8(SHUF[ll].as_ptr());
                    vst1q_u8(d.sub(ll), vqtbl2q_u8(uint8x16x2_t(a, b), ix));
                    continue;
                }
                if ll != 0 {
                    copy16(l, d.sub(ll));
                    if ll > 16 {
                        long_literals(l.add(16), d.sub(ll).add(16), ll - 16)
                    }
                }
                if ml <= 16 && off >= ml {
                    copy16(source, d)
                } else {
                    long_match(d, off, ml)
                }
            }
        }
    } else {
        let mut j = start;
        let mut l = lstart;
        for h in 0..run.count {
            let ll = f.ll[h] as usize;
            let ml = f.ml[h] as usize;
            let off = f.dst[h].wrapping_sub(f.src[h]) as usize;
            dst[j..j + ll].copy_from_slice(&lits[l..l + ll]);
            j += ll;
            l += ll;
            if BOOK && f.src[h] >= BOOK_BIT {
                dst[j..j + ml].copy_from_slice(&book[((f.src[h] - BOOK_BIT) / 16) as usize][..ml]);
                j += ml;
                continue;
            }
            let k = off.min(ml);
            dst.copy_within(j - off..j - off + k, j);
            let mut k = k;
            while k < ml {
                let m = k.min(ml - k);
                dst.copy_within(j..j + m, j + k);
                k += m
            }
            j += ml;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn from_page<
    const EXTENDED: bool,
    const MIN: usize,
    const EXTERNAL: bool,
    const LEAF: bool,
    const BOOK: bool,
    const TAGGED: bool,
    const PAGED: bool,
    const SPAN: bool,
>(
    mut p: &[u8],
    dst: &mut [u8],
    external: &[u8],
    book: &[[u8; 16]],
    book_base: u32,
    codes: BookTags,
    cursor: &mut pages::Cursor,
    cuts: [u8; 3],
) -> Result<()> {
    let n = dst.len();
    let size = if PAGED {
        n.checked_sub(cursor.at).ok_or_else(bad)?
    } else {
        n
    };
    let tags = leaf_get::<LEAF>(&mut p, size / MIN)?;
    let offs = if LEAF {
        leaf_get::<true>(&mut p, tags.len())?
    } else {
        ans::get_context(&mut p, tags.len(), Some(&tags))?
    };
    if offs.len() != tags.len() {
        return Err(bad());
    }
    let lens = leaf_get::<LEAF>(&mut p, size)?;
    let mut lens = &lens[..];
    let encoded_lits = leaf_get::<LEAF>(&mut p, size)?;
    let lits = if EXTERNAL {
        if !encoded_lits.is_empty() || external.len() > size {
            return Err(bad());
        }
        external
    } else {
        &*encoded_lits
    };
    let lo = leaf_get::<LEAF>(&mut p, tags.len() + 16)?;
    let mi = leaf_get::<LEAF>(&mut p, tags.len() + 16)?;
    let far = if EXTENDED {
        leaf_get::<LEAF>(&mut p, tags.len() * 3 + if SPAN { 16 } else { 0 })?
    } else {
        std::borrow::Cow::Borrowed(&[][..])
    };
    let mut far = &*far;
    if lo.len() < 16 || mi.len() < 16 {
        return Err(bad());
    }
    let (mut li, mut ji) = (0, 0);
    let mut rep = if PAGED { cursor.rep } else { [1, 4, 8] };
    let mut at = if PAGED { cursor.at } else { 0 };
    let mut lit = 0;
    let f = &mut Fields::new();
    let prefetch = n > crate::MAX_BLOCK;
    let early = EXTENDED && n > (4 << 20);
    let batch = if early { 32 } else { BATCH };
    for (ts, os) in tags.chunks(batch).zip(offs.chunks(batch)) {
        #[cfg(target_arch = "aarch64")]
        if PAGED || (!BOOK && !EXTERNAL && n > 4 << 20 && n <= 5 << 20) {
            // These are hints on sequential input. Only the checked slices below are read.
            for ptr in [
                ts.as_ptr().wrapping_add(256),
                os.as_ptr().wrapping_add(256),
                lo.as_ptr().wrapping_add(li + 256),
                mi.as_ptr().wrapping_add(ji + 128),
                lens.as_ptr().wrapping_add(64),
                lits.as_ptr().wrapping_add(lit + 512),
            ] {
                unsafe {
                    core::arch::asm!("prfm pldl1keep, [{p}]", p=in(reg)ptr,options(readonly,nostack,preserves_flags));
                }
            }
        }
        let start = at;
        let lstart = lit;
        let mut abs = [0u32; BATCH + 3];
        abs[..3].copy_from_slice(&[rep[2] as u32 + 19, rep[1] as u32 + 19, rep[0] as u32 + 19]);
        let mut k = 0;
        #[cfg(target_arch = "aarch64")]
        while k + 16 <= os.len() && li + 16 <= lo.len() && ji + 16 <= mi.len() {
            let (a, b) = if SPAN {
                let (a, b, h) = unsafe {
                    dspan::sixteen(
                        os.as_ptr().add(k),
                        lo.as_ptr().add(li),
                        mi.as_ptr().add(ji),
                        far,
                        abs.as_mut_ptr().add(k + 3),
                        cuts,
                    )?
                };
                far = &far[h..];
                (a, b)
            } else {
                unsafe {
                    expand_book16::<EXTENDED, TAGGED>(
                        os.as_ptr().add(k),
                        lo.as_ptr().add(li),
                        mi.as_ptr().add(ji),
                        abs.as_mut_ptr().add(k + 3),
                        &mut far,
                        n,
                        codes,
                    )?
                }
            };
            li += a;
            ji += b;
            k += 16;
        }
        for (&c, d) in os[k..].iter().zip(&mut abs[k + 3..]) {
            *d = if SPAN {
                dspan::scalar(c, &lo, &mi, &mut li, &mut ji, &mut far, cuts)?
            } else if c < 84 {
                c as u32
            } else {
                let l = *lo.get(li).ok_or_else(bad)? as u32;
                li += 1;
                if c < 212 {
                    ((c as u32 - 84) << 8) | l
                } else {
                    let m = *mi.get(ji).ok_or_else(bad)? as u32;
                    ji += 1;
                    ((c as u32 - 212) << 16) | (m << 8) | l
                }
            }
        }
        if TAGGED {
            for (i, &c) in os.iter().enumerate().skip(k) {
                abs[i + 3] = codes.mapped(c, abs[i + 3]);
            }
        }
        if EXTENDED && !SPAN {
            for (j, &c) in os.iter().enumerate().skip(k) {
                if c == 255 {
                    let extra = lenin(&mut far)?;
                    if extra > n >> 16 {
                        return Err(bad());
                    }
                    abs[j + 3] += (extra as u32) << 16;
                }
            }
        }
        let mut j = 0;
        #[cfg(target_arch = "aarch64")]
        while j + 16 <= ts.len() {
            unsafe {
                prep16::<MIN, BOOK>(
                    f,
                    j,
                    ts.as_ptr().add(j),
                    os.as_ptr().add(j),
                    &mut abs,
                    &mut lens,
                    &mut at,
                    &mut lit,
                    n,
                    book_base,
                    book.len() as u32,
                )?
            }
            #[cfg(target_arch = "aarch64")]
            if early {
                for h in j..j + 16 {
                    if (!BOOK || f.src[h] < BOOK_BIT) && f.dst[h] - f.src[h] >= 256 {
                        let p = dst.as_ptr().wrapping_add(f.src[h] as usize);
                        unsafe {
                            core::arch::asm!("prfm pldl1keep, [{p}]",p=in(reg)p,options(readonly,nostack,preserves_flags));
                            if (p as usize & 63) > 48 {
                                core::arch::asm!("prfum pldl1keep, [{p}, #15]",p=in(reg)p,options(readonly,nostack,preserves_flags));
                            }
                        }
                    }
                }
            }
            j += 16;
        }
        for j in j..ts.len() {
            let t = ts[j];
            let c = abs[j + 3];
            let code = if c >= 20 {
                c
            } else if c < 3 {
                abs[j + 2 - c as usize]
            } else {
                abs[j + 2].wrapping_add(c).wrapping_sub(11)
            };
            abs[j + 3] = code;
            let mut ll = (t >> 4) as usize;
            let mut ml = (t & 15) as usize + MIN;
            if ll == 15 {
                ll += lenin(&mut lens)?
            }
            if ml == 15 + MIN {
                ml += lenin(&mut lens)?
            }
            let off = code.wrapping_sub(19) as usize;
            let source = if BOOK && off.wrapping_sub(book_base as usize) < book.len() {
                if ml > 16 {
                    return Err(bad());
                }
                BOOK_BIT | (((off - book_base as usize) * 16) as u32)
            } else {
                let off = if BOOK && off >= book_base as usize + book.len() {
                    off - book.len()
                } else {
                    off
                };
                if off == 0 || off > at + ll {
                    return Err(bad());
                }
                (at + ll - off) as u32
            };
            f.ll[j] = ll as u32;
            f.ml[j] = ml as u32;
            f.src[j] = source;
            f.dst[j] = (at + ll) as u32;
            f.lit[j] = lit as u32;
            lit += ll;
            at += ll + ml;
        }
        let tail = &abs[ts.len()..ts.len() + 3];
        rep = [
            tail[2].wrapping_sub(19) as usize,
            tail[1].wrapping_sub(19) as usize,
            tail[0].wrapping_sub(19) as usize,
        ];
        if at > n || lit > lits.len() {
            return Err(bad());
        }
        let run = Ready {
            start,
            lstart,
            at,
            lit,
            count: ts.len(),
        };
        restore_ready::<BOOK>(dst, lits, book, f, run, early, prefetch);
    }
    if !lens.is_empty()
        || far.len() != if SPAN { 16 } else { 0 }
        || !p.is_empty()
        || li + 16 != lo.len()
        || ji + 16 != mi.len()
        || lits.len() - lit != n - at
    {
        return Err(bad());
    }
    if PAGED {
        cursor.at = n;
        cursor.rep = rep;
    }
    dst[at..].copy_from_slice(&lits[lit..]);
    Ok(())
}

// Two suffix-order neighbors give the longest previous match at each byte.
// Short-distance and repeat candidates compete with those longer references
// in a shortest-path parse, where every edge is priced in sixteenths of a bit.
pub(crate) fn optimal(s: &[u8], level: u8) -> Vec<u8> {
    optimal_parse::<false>(s, level)
}
pub(crate) fn optimal_parse<const PRICED: bool>(s: &[u8], level: u8) -> Vec<u8> {
    optimal_engine::<PRICED, 0>(s, level)
}
pub(crate) fn optimal_engine<const PRICED: bool, const BT: usize>(s: &[u8], level: u8) -> Vec<u8> {
    if s.len() < 16 {
        return quick::<1>(s, false);
    }
    let n = s.len();
    let min = if PRICED || level >= 14 { 3 } else { 4 };
    let mut near = if level >= 16 && BT == 0 {
        vec![[u32::MAX; 2]; n]
    } else {
        vec![]
    };
    if level >= 16 && BT == 0 {
        let mut x = Vec::with_capacity(n + 1);
        x.extend(s.iter().map(|&b| b as u32 + 1));
        x.push(0);
        let sa = crate::sorted::suffixes(&x, 257);
        drop(x);
        let mut stack = Vec::<u32>::with_capacity(4096);
        for &p in &sa {
            if p as usize == n {
                continue;
            }
            while let Some(&q) = stack.last() {
                if q < p {
                    break;
                }
                stack.pop();
                near[q as usize][1] = p;
            }
            if let Some(&q) = stack.last() {
                near[p as usize][0] = q;
            }
            stack.push(p);
        }
        drop(sa);
        drop(stack);
    }
    let mut tree = (BT != 0).then(|| crate::prefixtree::Tree::new(n));
    let mut counts = [0u32; 256];
    for &c in s {
        counts[c as usize] += 1;
    }
    let mut lit = [0u32; 256];
    for (i, c) in counts.into_iter().enumerate() {
        lit[i] = ((n as f64 / c.max(1) as f64).log2().clamp(2.5, 11.) * 16. + 9.) as u32;
    }
    let mut cost = vec![u32::MAX; n + 1];
    cost[0] = 0;
    // A zero-length step is a literal; its offset stores the previous match
    // distance so the repeat candidate is available on the next edge.
    let mut step = vec![(0u32, 1u32); n + 1];
    let mut h3 = vec![u32::MAX; 1 << 16];
    let mut h8 = vec![u32::MAX; 1 << 17];
    let mut h4b = vec![u32::MAX; 1 << 16];
    let mut h4 = vec![u32::MAX; 1 << 16];
    let mut h6 = vec![u32::MAX; 1 << 17];

    let mut gaps = if PRICED { vec![0u8; n + 1] } else { vec![] };
    let mut limit_long = 0;
    let cap = if level >= 20 { 96 } else { 64 };
    for i in 0..n {
        let base = cost[i];
        let prev = step[i].1;
        let gap = if PRICED { gaps[i] } else { 0 };
        let literal_price = lit[s[i] as usize];

        let literal = base + literal_price;
        if literal < cost[i + 1] {
            cost[i + 1] = literal;
            step[i + 1] = (0, prev);
            if PRICED {
                gaps[i + 1] = (gap + 1).min(15);
            }
        }
        if i + 8 > n {
            continue;
        }
        let w = word(s, i);
        let a = ((w as u32).wrapping_mul(0x9e3779b1) >> 16) as usize;
        let b = ((w << 16).wrapping_mul(0x9e3779b97f4a7c15) >> 47) as usize;
        let q = std::mem::replace(&mut h4[a], i as u32);
        let q2 = std::mem::replace(&mut h4b[a], q);
        let p3 = if min == 3 {
            std::mem::replace(
                &mut h3[((w << 40).wrapping_mul(0x9e3779b97f4a7c15) >> 48) as usize],
                i as u32,
            )
        } else {
            u32::MAX
        };
        let p8 = std::mem::replace(
            &mut h8[(w.wrapping_mul(0xa24baed4963ee407) >> 47) as usize],
            i as u32,
        );
        let r = std::mem::replace(&mut h6[b], i as u32);
        let rep = (i as u32).checked_sub(prev).unwrap_or(u32::MAX);
        let mut matches = [(u32::MAX, 0u32, 0usize); 8];
        let mut used = 0;
        let neighbors = if let Some(t) = &mut tree {
            t.insert::<BT>(s, i)
        } else if level >= 16 {
            near[i]
        } else {
            [u32::MAX; 2]
        };
        for cand in [rep, q, p3, r, q2, p8, neighbors[0], neighbors[1]] {
            if cand >= i as u32 {
                continue;
            }
            let p = cand as usize;
            let off = (i - p) as u32;
            if matches[..used].iter().any(|&(_, d, _)| d == off) {
                continue;
            }
            let xor = w ^ word(s, p);
            let mut len = (xor.trailing_zeros() / 8) as usize;
            if len < min || (len == 3 && off > 4096 && off != prev) {
                continue;
            }
            if xor == 0 {
                let bound = (n - i).min(cap);
                while len + 8 <= bound {
                    let z = word(s, i + len) ^ word(s, p + len);
                    len += (z.trailing_zeros() / 8) as usize;
                    if z != 0 {
                        break;
                    }
                }
                if len >= cap && i >= limit_long {
                    len += common(s, p + len, i + len);
                    if len > cap {
                        limit_long = i + len - cap / 2;
                    }
                }
            }
            let distance = if off == prev {
                24
            } else {
                48 + off.ilog2() * 16
            };

            matches[used] = (distance, off, len);
            used += 1;
        }
        matches[..used].sort_unstable_by_key(|e| e.0);
        let mut first = min;
        for &(dist, off, len) in &matches[..used] {
            if first > len {
                continue;
            }
            let base = base + dist;
            let mut k = first;
            loop {
                let length_price =
                    40 + (k - min + 1).ilog2() * 13 + if k >= 15 + min { 32 } else { 0 };

                let price = base + length_price;
                if price < cost[i + k] {
                    cost[i + k] = price;
                    step[i + k] = (k as u32, off);
                    if PRICED {
                        gaps[i + k] = 0;
                    }
                }
                if k == len {
                    break;
                }
                k = if k < 24 {
                    k + 1
                } else if k < 96 {
                    (k + 4).min(len)
                } else {
                    len
                };
            }
            first = len + 1;
        }
    }
    let mut seq = vec![];
    let mut at = n;
    while at > 0 {
        let (len, off) = step[at];
        if len == 0 {
            at -= 1;
        } else {
            at -= len as usize;
            seq.push((at, len as usize, off as usize));
        }
    }

    let mut st = Streams::sized(n);
    st.minimum = min;
    let mut at = 0;
    for &(i, l, o) in seq.iter().rev() {
        st.emit(s, at, i, l, o);
        at = i + l;
    }
    st.finish_policy(s, at, false, level >= 17)
}

#[inline(always)]
fn fastkey<const W: usize, const B: usize>(w: u64) -> usize {
    ((w << (64 - W * 8)).wrapping_mul(0x9e3779b97f4a7c15) >> (64 - B)) as usize
}
#[inline(always)]
fn partial(s: &[u8], i: usize, w: u64, slot: u32, sig: u32) -> (usize, usize) {
    let p = slot as usize & MASK;
    if (slot ^ sig) > MASK as u32 || p >= i {
        return (0, 0);
    }
    let x = unsafe { s.as_ptr().add(p).cast::<u64>().read_unaligned() }.to_le() ^ w;
    ((x.trailing_zeros() / 8) as usize, p)
}

pub(crate) fn paired_cost<
    const W: usize,
    const B: usize,
    const TAIL: usize,
    const LAZY: bool,
    const PB: u32,
    const START: usize,
    const DEEP: bool,
    const OPT: usize,
>(
    s: &[u8],
) -> Vec<u8> {
    #[inline(always)]
    fn key<const W: usize, const B: usize, const OPT: usize>(w: u64) -> usize {
        if OPT & 4 == 0 {
            fastkey::<W, B>(w)
        } else {
            let h = (w << (64 - W * 8)).wrapping_mul(0x9e3779b97f4a7c15);
            (((h >> (64 - B - 2)) as usize) * 3) >> 4
        }
    }
    #[inline(always)]
    fn sig<const P: u32>(w: u64) -> u32 {
        (w as u32).wrapping_mul(0x9e3779b1) << P
    }
    #[inline(always)]
    fn part<const P: u32>(s: &[u8], i: usize, w: u64, slot: u32, sig: u32) -> (usize, usize) {
        if P == 21 {
            return partial(s, i, w, slot, sig);
        }
        let m = (1u32 << P) - 1;
        let p = (slot & m) as usize;
        if (slot ^ sig) > m || p >= i {
            return (0, 0);
        }
        (((w ^ word(s, p)).trailing_zeros() / 8) as usize, p)
    }
    let mut table = vec![0u32; if OPT & 4 == 0 { 1 << B } else { 3 << (B - 2) }];
    let mut st = Streams::sized(s.len());
    let mut i = 0;
    let mut anchor = 0;
    while i + 9 <= s.len() {
        let a = word(s, i);
        let b = word(s, i + 1);
        let ka = key::<W, B, OPT>(a);
        let kb = key::<W, B, OPT>(b);
        let sa = sig::<PB>(a);
        let sb = sig::<PB>(b);
        let pa = table[ka];
        let pb = table[kb];
        table[ka] = i as u32 | sa;
        table[kb] = (i + 1) as u32 | sb;
        let (mut la, mut pos) = part::<PB>(s, i, a, pa, sa);
        let (mut lb, p) = part::<PB>(s, i + 1, b, if ka == kb { i as u32 | sa } else { pb }, sb);
        if la == 4 && i - pos > 12000 {
            la = 0;
        }
        if lb == 4 && i + 1 - p > 12000 {
            lb = 0;
        }
        if DEEP && lb == 8 && la >= 6 {
            lb += common(s, p + 8, i + 9);
            if la == 8 {
                la += common(s, pos + 8, i + 8);
            }
        }
        if la < 4 && lb < 4 {
            i += 2;
            continue;
        }
        if la < 4
            || (LAZY
                && (lb >= la + 2
                    || (lb == la + 1
                        && (OPT & 3 == 2
                            || (OPT & 3 == 1 && i + 1 - p < (i - pos).saturating_mul(2))))))
        {
            i += 1;
            la = lb;
            pos = p;
        }
        if la == 8 {
            la += common(s, pos + 8, i + 8);
        }
        let off = i - pos;
        while i > anchor && pos > 0 && s[i - 1] == s[pos - 1] {
            i -= 1;
            pos -= 1;
            la += 1;
        }
        st.emit(s, anchor, i, la, off);
        let first = i + 2;
        for j in first..(first + START).min(i + la).min(s.len().saturating_sub(7)) {
            let w = word(s, j);
            table[key::<W, B, OPT>(w)] = j as u32 | sig::<PB>(w);
        }
        if OPT & 8 != 0 && la >= 14 {
            let j = i + la / 2;
            if j + 8 <= s.len() {
                let w = word(s, j);
                table[key::<W, B, OPT>(w)] = j as u32 | sig::<PB>(w);
            }
        }
        i += la;
        anchor = i;
        if i + 8 <= s.len() {
            for j in i - TAIL..i {
                let w = word(s, j);
                table[key::<W, B, OPT>(w)] = j as u32 | sig::<PB>(w);
            }
        }
    }
    st.finish(s, anchor, false)
}

#[cfg(test)]
mod preset_tests {
    use super::*;
    #[test]
    fn rejecting_a_short_match_after_lookahead() {
        let mut s = Vec::new();
        for i in 0..4000u32 {
            s.extend(b"abcd");
            s.extend(i.wrapping_mul(0x9e3779b1).to_le_bytes());
        }
        let b = dense::<false>(&s, 10, true);
        let mut d = vec![0; s.len()];
        decode(&b[1..], &mut d).unwrap();
        assert_eq!(d, s);
    }
    #[test]
    fn far_references_and_short_matches() {
        // Cover long offsets in both SIMD batches and the scalar tail.
        let mut s = vec![0u8; 3_010_000];
        let mut rng_state = 29u32;
        for q in &mut s {
            rng_state = rng_state.wrapping_mul(1664525).wrapping_add(1013904223);
            *q = (rng_state >> 24) as u8;
        }
        let n = s.len();
        let mut st = Streams::sized(n);
        st.minimum = 3;
        let mut anchor = 0;
        for k in 0..75 {
            let pos = n - 470 + 6 * k;
            let old = k * 17;
            let ll = if k % 2 == 0 { 3 } else { 5 };
            s.copy_within(old..old + ll, pos);
            st.emit(&s, anchor, pos, ll, pos - old);
            anchor = pos + ll;
        }
        let b = st.finish(&s, anchor, false);
        assert_eq!(b[0], 17);
        let mut out = vec![0x65; s.len()];
        decode_min::<true, 3>(&b[1..], &mut out).unwrap();
        assert_eq!(s, out);
        assert!(decode_min::<true, 3>(&b[1..b.len() - 1], &mut out).is_err());
    }
    #[cfg(target_arch = "aarch64")]
    #[test]
    fn mixed_far_lanes_advance_only_the_marked_offsets() {
        let codes = [
            255, 20, 255, 211, 255, 84, 212, 23, 255, 0, 1, 2, 3, 4, 5, 255,
        ];
        let low = [0x12; 16];
        let mid = [0x34; 16];
        let run = |bytes: &[u8], limit: usize| -> Result<([u32; 16], usize)> {
            let mut far = bytes;
            let mut out = [0; 16];
            let counts = unsafe {
                expand16::<true>(
                    codes.as_ptr(),
                    low.as_ptr(),
                    mid.as_ptr(),
                    out.as_mut_ptr(),
                    &mut far,
                    limit,
                )?
            };
            assert_eq!(counts, (8, 6));
            Ok((out, bytes.len() - far.len()))
        };
        // The unpadded scalar tail must match the padded vector path.
        let mut single = [0; 16];
        single[..5].copy_from_slice(&[0, 1, 5, 10, 42]);
        let (dense, consumed) = run(&single, 8 << 20).unwrap();
        assert_eq!(consumed, 5);
        assert_eq!(run(&single[..5], 8 << 20).unwrap(), (dense, 5));
        assert_eq!(
            dense,
            [
                0x2b3412, 20, 0x2c3412, 0x7f12, 0x303412, 0x12, 0x3412, 23, 0x353412, 0, 1, 2, 3,
                4, 5, 0x553412
            ]
        );
        let (multi, consumed) = run(&[0x81, 1, 1, 5, 10, 42], 16 << 20).unwrap();
        assert_eq!(consumed, 6);
        assert_eq!(multi[0], 0xac3412);
        assert_eq!(multi[1..], dense[1..]);
        assert!(run(&single[..4], 8 << 20).is_err());
        assert!(run(&[0x80], 8 << 20).is_err());
        assert!(run(&single, 1 << 20).is_err());
    }
}

pub(crate) mod pages;

pub(crate) mod dspan;

#[cfg(target_arch = "aarch64")]
mod emitoffset;

#[cfg(target_arch = "aarch64")]
mod lengthword;

#[cfg(target_arch = "aarch64")]
mod pairhead;

mod quicktwo;
#[cfg(target_arch = "aarch64")]
pub(crate) mod sharedentropy;
