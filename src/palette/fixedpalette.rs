// The complete alphabet, including unseen bytes, lets one block price a run
// of neighboring blocks. Each block carries its own canonical palette header.
use super::{Code, rankgroups};
use std::mem::MaybeUninit;

#[inline(always)]
fn store<const N: usize>(dst: &mut [MaybeUninit<u8>], at: usize, bytes: [u8; N]) {
    dst[at..at + N].copy_from_slice(&bytes.map(MaybeUninit::new));
}

pub(crate) struct Model {
    map: [Code; 256],
    pairs: Vec<u32>,
    header: Vec<u8>,
    k: usize,
}
#[inline(always)]
fn both<const K: usize>(a: Code, b: Code) -> u32 {
    let rank = u32::from(a.rank) | (u32::from(b.rank) << a.width);
    let head = u32::from(a.head) | (u32::from(b.head) << K);
    u32::from(a.width + b.width) | (rank << 5) | (head << 21)
}
impl Model {
    pub(crate) fn train(data: &[u8], pairs: bool) -> Option<Self> {
        if data.len() < 1536 {
            return None;
        }
        let mut counts = [1u32; 256];
        for &b in data {
            counts[b as usize] += 1
        }
        let mut alphabet: Vec<u8> = (0..=255).collect();
        alphabet.sort_unstable_by_key(|&c| std::cmp::Reverse(counts[c as usize]));
        let mut prefix = [0; 512];
        for (i, &b) in alphabet.iter().enumerate() {
            prefix[i + 1] = prefix[i] + counts[b as usize]
        }
        let (bits, widths) = rankgroups::plan(&prefix, 256, data.len() + 256, 1);
        if bits >= (data.len() + 256) * 8 {
            return None;
        }
        let k = widths.len().ilog2() as usize;
        let mut header = vec![5, k as u8, 255];
        header.extend(&widths);
        header.extend(&alphabet);
        let mut map = [Code::default(); 256];
        let mut at = 0;
        for (h, &w) in widths.iter().enumerate() {
            let end = (at + (1 << w)).min(256);
            for (rank, &b) in alphabet[at..end].iter().enumerate() {
                map[b as usize] = Code {
                    head: h as u8,
                    rank: rank as u8,
                    width: w,
                };
            }
            at = end;
        }
        debug_assert_eq!(at, 256);
        let mut m = Self {
            map,
            header,
            k,
            pairs: Vec::new(),
        };
        if pairs {
            macro_rules! make {
                ($k:expr) => {
                    (0..65536)
                        .map(|i| both::<$k>(m.map[i & 255], m.map[i >> 8]))
                        .collect()
                };
            }
            m.pairs = match k {
                1 => make!(1),
                2 => make!(2),
                3 => make!(3),
                4 => make!(4),
                _ => make!(5),
            };
        }
        Some(m)
    }
    pub(crate) fn put(&self, data: &[u8], out: &mut Vec<u8>) -> bool {
        self.put_padded::<true>(data, out)
    }
    fn put_padded<const FULL: bool>(&self, data: &[u8], out: &mut Vec<u8>) -> bool {
        if data.len() < 2048 {
            return false;
        }
        let begin = out.len();
        out.extend((data.len() as u32).to_le_bytes());
        out.extend(&self.header);
        let at = out.len();
        let hn = (data.len() * self.k).div_ceil(8) + 16;
        let capacity = hn + 4 + data.len() + 17;
        out.reserve(capacity);
        let slots = &mut out.spare_capacity_mut()[..capacity];
        if FULL {
            slots.fill(MaybeUninit::new(0));
        }
        let (h, r) = slots.split_at_mut(hn);
        macro_rules! put {
            ($k:expr) => {
                if self.pairs.is_empty() {
                    self.pack::<$k, false>(data, h, &mut r[4..])
                } else {
                    self.pack::<$k, true>(data, h, &mut r[4..])
                }
            };
        }
        let bits = match self.k {
            1 => put!(1),
            2 => put!(2),
            3 => put!(3),
            4 => put!(4),
            _ => put!(5),
        };
        store(r, 0, (bits as u32).to_le_bytes());
        let tail_end = 4 + bits.div_ceil(8);
        let end = at + hn + tail_end + 17;
        if end - begin > data.len() + 5 {
            out.truncate(begin);
            return false;
        }
        if !FULL {
            h[hn - 16..].fill(MaybeUninit::new(0));
            r[tail_end..tail_end + 17].fill(MaybeUninit::new(0));
        }
        // The head writer advances K bytes per eight-byte store; the rank writer
        // advances at most its store width. Together with both pads and the rank
        // bit count, these stores initialize every byte exposed by set_len.
        unsafe { out.set_len(end) };
        true
    }
    fn pack<const K: usize, const PAIRED: bool>(
        &self,
        data: &[u8],
        heads: &mut [MaybeUninit<u8>],
        tail: &mut [MaybeUninit<u8>],
    ) -> usize {
        let mut bit = 0usize;
        let mut carry = 0u64;
        let mut hp = 0;
        let mut chunks = data.chunks_exact(8);
        let look = |a: u8, b: u8| {
            if PAIRED {
                // The table is initialized for every 16-bit value.
                unsafe {
                    *self
                        .pairs
                        .get_unchecked(usize::from(u16::from_le_bytes([a, b])))
                }
            } else {
                both::<K>(self.map[a as usize], self.map[b as usize])
            }
        };
        for g in &mut chunks {
            let pairs = [
                look(g[0], g[1]),
                look(g[2], g[3]),
                look(g[4], g[5]),
                look(g[6], g[7]),
            ];
            let h = u64::from(pairs[0] >> 21)
                | (u64::from(pairs[1] >> 21) << (2 * K))
                | (u64::from(pairs[2] >> 21) << (4 * K))
                | (u64::from(pairs[3] >> 21) << (6 * K));
            store(heads, hp, h.to_le_bytes());
            hp += K;
            for &[a, b] in pairs.as_chunks::<2>().0 {
                let aw = (a & 31) as usize;
                let bits = u64::from((a >> 5) & 65535) | (u64::from((b >> 5) & 65535) << aw);
                let w = aw + (b & 31) as usize;
                let shift = bit & 7;
                let v = carry | (bits << shift);
                store(tail, bit / 8, v.to_le_bytes());
                carry = v >> ((shift + w) & !7);
                bit += w;
            }
        }
        let mut h = 0u64;
        for (j, &s) in chunks.remainder().iter().enumerate() {
            let c = self.map[s as usize];
            h |= u64::from(c.head) << (j * K);
            let shift = bit & 7;
            let v = carry | (u64::from(c.rank) << shift);
            store(tail, bit / 8, (v as u16).to_le_bytes());
            carry = v >> ((shift + c.width as usize) & !7);
            bit += c.width as usize;
        }
        store(heads, hp, h.to_le_bytes());
        bit
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reused_models_cover_unseen_bytes_and_pair_tables_match_scalar_bytes() {
        let train: Vec<_> = (0..32000)
            .map(|i| if i % 7 == 0 { (i % 113) as u8 } else { 42 })
            .collect();
        let a = Model::train(&train, false).unwrap();
        let b = Model::train(&train, true).unwrap();
        for n in 4096..4112 {
            let mut data = train[..n].to_vec();
            data[..256].copy_from_slice(&(0..=255).collect::<Vec<_>>());
            let mut x = vec![0x5a];
            let mut y = x.clone();
            assert!(a.put(&data, &mut x) && b.put(&data, &mut y));
            assert_eq!(x, y);
            for model in [&a, &b] {
                let mut reused = vec![0xa7; n * 2];
                reused.truncate(1);
                reused[0] = 0x5a;
                assert!(model.put_padded::<false>(&data, &mut reused));
                assert_eq!(reused, x);
            }
            let mut p = &x[1..];
            assert_eq!(crate::ans::get(&mut p, n).unwrap().as_ref(), data);
            assert!(p.is_empty());
        }
        let flat: Vec<_> = (0..=255).cycle().take(8192).collect();
        let mut v = vec![13];
        assert!(!a.put(&flat, &mut v));
        assert_eq!(v, [13]);
        assert!(!a.put_padded::<false>(&flat, &mut v));
        assert_eq!(v, [13]);
    }
}
