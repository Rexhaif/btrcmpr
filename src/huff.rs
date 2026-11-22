// Length-limited prefix codes and a multi-symbol decode table.
use crate::{Result, bad, take, u32in};
use std::{cmp::Reverse, collections::BinaryHeap};
const K: usize = 11;
const SIZE: usize = 1 << K;
fn lengths(counts: &[u32; 256]) -> [u8; 256] {
    let mut heap = BinaryHeap::new();
    let mut parents = [0u16; 512];
    let mut sorted = Vec::new();
    for (i, &c) in counts.iter().enumerate() {
        if c != 0 {
            heap.push(Reverse((c, i)));
            sorted.push(i)
        }
    }
    let mut next = 256;
    while heap.len() > 1 {
        let Reverse((a, x)) = heap.pop().unwrap();
        let Reverse((b, y)) = heap.pop().unwrap();
        parents[x] = next as u16;
        parents[y] = next as u16;
        heap.push(Reverse((a + b, next)));
        next += 1
    }
    let mut depths = [0usize; 512];
    let mut buckets = [0; K + 1];
    let mut units = 0;
    for i in (0..next - 1).rev() {
        depths[i] = depths[parents[i] as usize] + 1;
        if i < 256 && counts[i] != 0 {
            let l = depths[i].min(K);
            buckets[l] += 1;
            units += 1 << (K - l)
        }
    }
    while units > SIZE {
        let j = (1..K).rev().find(|&j| buckets[j] > 0).unwrap();
        buckets[j] -= 1;
        buckets[j + 1] += 2;
        buckets[K] -= 1;
        units -= 1
    }
    sorted.sort_unstable_by_key(|&i| Reverse(counts[i]));
    let mut lens = [0u8; 256];
    let mut at = 0;
    for (l, &n) in buckets.iter().enumerate().skip(1) {
        for &i in &sorted[at..at + n] {
            lens[i] = l as u8
        }
        at += n
    }
    lens
}
fn codes(lens: &[u8; 256]) -> Result<[u16; 256]> {
    let mut count = [0u32; K + 1];
    for &l in lens {
        if l as usize > K {
            return Err(bad());
        }
        count[l as usize] += 1
    }
    count[0] = 0;
    let mut next = [0u32; K + 1];
    for i in 1..=K {
        next[i] = (next[i - 1] + count[i - 1]) * 2
    }
    if next[K] + count[K] != SIZE as u32 {
        return Err(bad());
    }
    let mut table = [0; 256];
    for i in 0..256 {
        let l = lens[i] as usize;
        if l != 0 {
            let val = next[l];
            next[l] += 1;
            table[i] = (val.reverse_bits() >> (32 - l)) as u16
        }
    }
    Ok(table)
}
pub fn put(data: &[u8], counts: &[u32; 256], out: &mut Vec<u8>, ideal: f64) -> bool {
    let lens = lengths(counts);
    let nbits: usize = lens
        .iter()
        .zip(counts)
        .map(|(&l, &c)| l as usize * c as usize)
        .sum();
    // rANS wins decisively for sharply skewed streams.
    if nbits as f64 > ideal * 1.06 + 1200. || nbits / 8 + 176 >= data.len() {
        return false;
    }
    let tab = codes(&lens).unwrap();
    out.push(4);
    for a in lens.chunks_exact(2) {
        out.push(a[0] | (a[1] << 4))
    }
    let step = data.len().div_ceil(4);

    for lane in data.chunks(step) {
        let mark = out.len();
        out.extend([0; 4]);
        let mut bits = 0u64;
        let mut k = 0;
        let mut nb = 0;
        for &c in lane {
            let len = lens[c as usize] as u32;
            bits |= (tab[c as usize] as u64) << k;
            k += len;
            nb += len;
            if k >= 32 {
                out.extend((bits as u32).to_le_bytes());
                bits >>= 32;
                k -= 32
            }
        }
        out.extend_from_slice(&bits.to_le_bytes()[..k.div_ceil(8) as usize]);
        out.extend([0; 4]);
        out[mark..mark + 4].copy_from_slice(&nb.to_le_bytes());
    }
    true
}

#[derive(Clone, Copy, Default)]
#[repr(C, align(8))]
struct E {
    word: u32,
    bits: u8,
    n: u8,
}
#[derive(Clone, Copy)]
struct Lane<'a> {
    src: &'a [u8],
    end: usize,
    nb: usize,
    pos: usize,
    bit: usize,
}
pub fn get(p: &mut &[u8], n: usize) -> Result<Vec<u8>> {
    if n < 16 {
        return Err(bad());
    }
    let mut lens = [0u8; 256];
    for (c, &b) in lens.chunks_exact_mut(2).zip(take(p, 128)?) {
        c[0] = b & 15;
        c[1] = b >> 4
    }
    let codes = codes(&lens)?;
    let mut single = [0u16; SIZE];
    for c in 0..256 {
        let l = lens[c];
        if l != 0 {
            for x in (codes[c] as usize..SIZE).step_by(1 << l) {
                single[x] = ((l as u16) << 8) | c as u16
            }
        }
    }

    let mut tab = [E::default(); SIZE];
    for (x, e) in tab.iter_mut().enumerate() {
        let mut key = x;
        while e.n < 4 {
            let v = single[key];
            let b = (v >> 8) as u8;
            if e.bits + b > K as u8 {
                break;
            }
            e.word |= ((v & 255) as u32) << (e.n * 8);
            e.bits += b;
            e.n += 1;
            key >>= b
        }
    }
    let mut out = vec![0u8; n];
    let mut lanes = [Lane {
        src: &[],
        end: 0,
        nb: 0,
        pos: 0,
        bit: 0,
    }; 4];
    let step = n.div_ceil(4);
    for (j, l) in lanes.iter_mut().enumerate() {
        let nb = u32in(p)?;
        let src = take(p, nb.div_ceil(8) + 4)?;
        *l = Lane {
            src,
            end: ((j + 1) * step).min(n),
            nb,
            pos: j * step,
            bit: 0,
        }
    }
    while lanes
        .iter()
        .all(|l| l.pos + 32 <= l.end && l.bit + 8 * K <= l.nb)
    {
        for _ in 0..8 {
            for l in &mut lanes {
                // At most 32 bytes of output and 88 input bits per eight iterations.
                unsafe {
                    let w = std::ptr::read_unaligned(l.src.as_ptr().add(l.bit / 8).cast::<u32>())
                        .to_le();
                    let e = tab[((w >> (l.bit & 7)) as usize) & (SIZE - 1)];
                    std::ptr::write_unaligned(
                        out.as_mut_ptr().add(l.pos).cast::<u32>(),
                        e.word.to_le(),
                    );
                    l.bit += e.bits as usize;
                    l.pos += e.n as usize
                }
            }
        }
    }
    for l in &mut lanes {
        while l.pos < l.end {
            if l.bit >= l.nb {
                return Err(bad());
            }
            let w = u32::from_le_bytes(l.src[l.bit / 8..l.bit / 8 + 4].try_into().unwrap());
            let v = single[((w >> (l.bit & 7)) as usize) & (SIZE - 1)];
            out[l.pos] = v as u8;
            l.pos += 1;
            l.bit += (v >> 8) as usize;
        }
        if l.bit != l.nb {
            return Err(bad());
        }
    }
    Ok(out)
}
