// Group like sections of archived object files. Every byte, including archive
// padding and unknown sections, has an explicit place in the inverse layout.
use crate::{Frame, Result, bad, parallel, take, u32in};
use std::{cell::RefCell, io::Write};
#[derive(Clone, Copy, Debug)]
struct Piece {
    start: usize,
    len: usize,
    w: u8,
    phase: u8,
}
#[derive(Debug)]
pub(crate) struct Layout {
    pieces: Vec<Piece>,
    pub(crate) sizes: Vec<usize>,
    deposits: Vec<(usize, usize, usize)>,
}
fn le16(d: &[u8], at: usize) -> Option<usize> {
    Some(u16::from_le_bytes(d.get(at..at + 2)?.try_into().ok()?) as usize)
}
fn le64(d: &[u8], at: usize) -> Option<usize> {
    usize::try_from(u64::from_le_bytes(d.get(at..at + 8)?.try_into().ok()?)).ok()
}
fn octal(d: &[u8]) -> Option<usize> {
    let mut val = 0usize;
    let mut seen = false;
    for &b in d {
        if b == 0 || b == b' ' {
            if seen { break } else { continue }
        }
        if !(b'0'..=b'7').contains(&b) {
            return None;
        }
        seen = true;
        val = val.checked_mul(8)?.checked_add((b - b'0') as usize)?
    }
    seen.then_some(val)
}
fn object(d: &[u8]) -> Option<Vec<([u8; 8], Piece)>> {
    if le16(d, 0)? != 0x183 {
        return None;
    }
    let count = le16(d, 2)?;
    let header = 24 + le16(d, 20)?;
    if count > 1024 || header + 64 * count > d.len() {
        return None;
    }
    let mut r = Vec::new();
    for j in 0..count {
        let h = header + 64 * j;
        let len = le64(d, h + 24)?;
        let start = le64(d, h + 32)?;
        if start != 0 && len != 0 && start.checked_add(len)? <= d.len() {
            r.push((
                d[h..h + 8].try_into().ok()?,
                Piece {
                    start,
                    len,
                    w: 0,
                    phase: 0,
                },
            ))
        }
    }
    r.sort_by_key(|a| a.1.start);
    let mut end = 0;
    for (_, p) in &r {
        if p.start < end {
            return None;
        }
        end = p.start + p.len
    }
    Some(r)
}
impl Layout {
    pub(crate) fn linear(size: usize, sizes: Vec<usize>) -> Self {
        Self::new(
            vec![Piece {
                start: 0,
                len: size,
                w: 0,
                phase: 0,
            }],
            sizes,
        )
    }
    fn new(pieces: Vec<Piece>, sizes: Vec<usize>) -> Self {
        let mut plan = Self {
            pieces,
            sizes,
            deposits: vec![],
        };
        let mut by_address = vec![];
        for (id, row) in plan.jobs().into_iter().enumerate() {
            let mut pos = 0;
            for p in row {
                by_address.push((p.start, id, pos, p.len));
                pos += p.len
            }
        }
        by_address.sort_unstable_by_key(|r| r.0);
        plan.deposits = by_address
            .into_iter()
            .map(|(_, id, pos, n)| (id, pos, n))
            .collect();
        plan
    }
    pub(crate) fn detect(src: &[u8], level: u8) -> Option<Self> {
        if src.get(257..262)? != b"ustar" {
            return None;
        }
        let mut groups: Vec<([u8; 8], Vec<Piece>)> = vec![([0; 8], vec![])];
        let mut at = 0;
        let mut last = 0;
        let mut objects = 0;
        let mut recognized = 0;
        while let Some(h) = src.get(at..at + 512) {
            if h.iter().all(|&b| b == 0) {
                break;
            }
            let sum: usize = h[..148]
                .iter()
                .chain(&h[156..])
                .map(|&b| b as usize)
                .sum::<usize>()
                + 8 * 32;
            if sum != octal(&h[148..156])? {
                return None;
            }
            let size = octal(&h[124..136])?;
            let start = at + 512;
            let end = start.checked_add(size)?;
            let data = src.get(start..end)?;
            if let Some(sections) = object(data) {
                objects += 1;
                for (name, mut p) in sections {
                    p.w = match &name[..name.iter().position(|&x| x == 0).unwrap_or(8)] {
                        b".text" if level >= 4 => 64,
                        b".rel.dyn" => 4,
                        b".dynsym" => 6,
                        b".hash" | b".conflic" => 1,
                        b".pdata" | b".msym" | b".got" => 2,
                        _ => 0,
                    };
                    p.start += start;
                    if p.start > last {
                        groups[0].1.push(Piece {
                            start: last,
                            len: p.start - last,
                            w: 0,
                            phase: 0,
                        })
                    }
                    last = p.start + p.len;
                    recognized += p.len;
                    let id = if let Some(i) = groups.iter().position(|(n, _)| *n == name) {
                        i
                    } else {
                        groups.push((name, vec![]));
                        groups.len() - 1
                    };
                    if p.w != 0 {
                        let tail = p.len % unit(p.w);
                        if tail != 0 {
                            p.len -= tail;
                            groups[0].1.push(Piece {
                                start: p.start + p.len,
                                len: tail,
                                w: 0,
                                phase: 0,
                            })
                        }
                    }
                    if p.len != 0 {
                        groups[id].1.push(p)
                    }
                }
            }
            at = end.checked_add(511)? / 512 * 512;
        }
        if objects < 4 || recognized < src.len() / 3 {
            return None;
        }
        if last < src.len() {
            groups[0].1.push(Piece {
                start: last,
                len: src.len() - last,
                w: 0,
                phase: 0,
            })
        }
        let mut pieces: Vec<Piece> = Vec::new();
        for (_, g) in groups {
            for p in g {
                if let Some(q) = pieces.last_mut()
                    && q.w == p.w
                    && q.start + q.len == p.start
                    && q.len + p.len <= u32::MAX as usize
                {
                    q.len += p.len
                } else {
                    if p.len > u32::MAX as usize {
                        return None;
                    }
                    pieces.push(p)
                }
            }
        }
        let mut sizes = vec![];
        let mut pending = 0;
        let mut typ = 255;
        let kib = [
            128, 128, 128, 128, 160, 112, 128, 144, 152, 160, 168, 176, 184, 192, 200, 272, 448,
            640, 656, 672, 704, 448,
        ][(level - 1) as usize];
        // Window order: ordinary sections, records, instructions.
        let defaults = if level == 1 && cfg!(target_arch = "aarch64") {
            [192, 128, 192]
        } else if level == 17 {
            [512, 128, 352]
        } else if level == 20 {
            // Keep the level-19 window for instruction banks.
            [kib, kib, 656]
        } else {
            [kib; 3]
        };
        let windows = defaults.map(|kib| kib * 1024);
        let flush = |count: &mut usize, sizes: &mut Vec<usize>| {
            if *count > 0 {
                sizes.push(*count);
                *count = 0
            }
        };
        for p in &pieces {
            let limit = windows[if p.w == 64 {
                2
            } else if p.w == 0 {
                0
            } else {
                1
            }];
            if typ != p.w {
                flush(&mut pending, &mut sizes);
                typ = p.w
            }
            if level <= 3 && p.w == 0 && p.len >= 64 * 1024 {
                flush(&mut pending, &mut sizes);
                let mut n = p.len;
                while n > 0 {
                    let k = n.min(512 * 1024);
                    sizes.push(k);
                    n -= k
                }
            } else {
                let mut n = p.len;
                while n > 0 {
                    let k = n.min(limit - pending);
                    pending += k;
                    n -= k;
                    if pending == limit {
                        flush(&mut pending, &mut sizes)
                    }
                }
            }
        }
        flush(&mut pending, &mut sizes);
        Some(Self::new(pieces, sizes))
    }
    fn index(&self) -> Vec<u8> {
        fn var(mut n: u64, b: &mut Vec<u8>) {
            while n >= 128 {
                b.push(n as u8 | 128);
                n >>= 7
            }
            b.push(n as u8)
        }
        let mut b = Vec::with_capacity(self.pieces.len() * 6);
        b.extend((self.sizes.len() as u32).to_le_bytes());
        for &n in &self.sizes {
            var(n as u64, &mut b)
        }
        b.extend((self.pieces.len() as u32).to_le_bytes());
        let mut prev = 0u64;
        for p in &self.pieces {
            let start = p.start as u64;
            let delta = start.wrapping_sub(prev);
            var((delta << 1) ^ ((delta as i64 >> 63) as u64), &mut b);
            var(p.len as u64, &mut b);
            prev = start
        }
        b
    }
    pub(crate) fn serialized_len(&self) -> usize {
        self.index().len()
    }
    pub(crate) fn write(&self, w: &mut impl Write) -> Result<()> {
        w.write_all(&self.index())
    }
    pub(crate) fn read(p: &mut &[u8], size: usize) -> Result<Self> {
        fn var(p: &mut &[u8]) -> Result<u64> {
            let mut n = 0;
            for shift in (0..64).step_by(7) {
                let b = take(p, 1)?[0];
                if shift == 63 && b > 1 {
                    return Err(bad());
                }
                n |= ((b & 127) as u64) << shift;
                if b < 128 {
                    return Ok(n);
                }
            }
            Err(bad())
        }
        let nj = u32in(p)?;
        if nj == 0 || nj > p.len() {
            return Err(bad());
        }
        let mut sizes = Vec::with_capacity(nj);
        let mut total = 0usize;
        for _ in 0..nj {
            let n = usize::try_from(var(p)?).map_err(|_| bad())?;
            if n == 0 || n > crate::MAX_WIDE_BLOCK {
                return Err(bad());
            }
            total = total.checked_add(n).ok_or_else(bad)?;
            sizes.push(n)
        }
        if size != total {
            return Err(bad());
        }
        let count = u32in(p)?;
        if count == 0 || count > p.len() / 2 {
            return Err(bad());
        }
        let mut pieces = Vec::with_capacity(count);
        let mut prev = 0u64;
        for _ in 0..count {
            let zz = var(p)?;
            prev = prev.wrapping_add((zz >> 1) ^ 0u64.wrapping_sub(zz & 1));
            let start = usize::try_from(prev).map_err(|_| bad())?;
            let len = usize::try_from(var(p)?).map_err(|_| bad())?;
            if len == 0 || start.checked_add(len).is_none_or(|n| n > size) {
                return Err(bad());
            }
            pieces.push(Piece {
                start,
                len,
                w: 0,
                phase: 0,
            })
        }
        let mut sorted = pieces.clone();
        sorted.sort_unstable_by_key(|p| p.start);
        let mut at = 0;
        for p in sorted {
            if p.start != at {
                return Err(bad());
            }
            at += p.len
        }
        if at != size
            || (sizes.iter().any(|&n| n > crate::MAX_BLOCK)
                && !(pieces.len() == 1 && pieces[0].start == 0 && pieces[0].len == size))
        {
            return Err(bad());
        }
        Ok(Self::new(pieces, sizes))
    }
    fn jobs(&self) -> Vec<Vec<Piece>> {
        let mut v = vec![Vec::new(); self.sizes.len()];
        let (mut id, mut at) = (0, 0);
        for &p in &self.pieces {
            let mut j = 0;
            while j < p.len {
                let n = (self.sizes[id] - at).min(p.len - j);
                v[id].push(Piece {
                    start: p.start + j,
                    len: n,
                    w: p.w,
                    phase: if p.w == 0 { 0 } else { (j % unit(p.w)) as u8 },
                });
                at += n;
                j += n;
                if at == self.sizes[id] {
                    id += 1;
                    at = 0
                }
            }
        }
        v
    }
}
thread_local! {static SCRATCH:RefCell<Vec<u8>>=const{RefCell::new(Vec::new())};}
fn unit(w: u8) -> usize {
    if w == 64 { 4 } else { 4 * w as usize }
}
pub(crate) fn compress(
    src: &[u8],
    threads: usize,
    layout: Layout,
    level: u8,
    f: impl Fn(&[u8]) -> Result<Vec<u8>> + Sync,
) -> Result<Frame<'static>> {
    let block = crate::MAX_BLOCK;
    let jobs = layout.jobs();
    let mut blocks = vec![std::borrow::Cow::default(); jobs.len()];
    parallel(
        threads.min(blocks.len()),
        jobs.into_iter().zip(&mut blocks),
        |(r, out)| {
            let n = r.iter().map(|p| p.len).sum::<usize>();
            let mut payload = vec![5];
            let mut at = 0;
            while at < r.len() {
                let w = r[at].w;
                let mut end = at + 1;
                while end < r.len() && r[end].w == w {
                    end += 1
                }
                let group = &r[at..end];
                let phase = group[0].phase as usize;
                let compress_group = |s: &[u8]| -> Result<Vec<u8>> {
                    if w == 0 || s.len() < 128 {
                        return f(s);
                    }
                    if w == 64 {
                        if level >= 4 && s.len() >= 65536 {
                            return Ok(crate::object::banked(s, level));
                        }
                        return Ok(crate::object::instructions(s, level));
                    }
                    let width = 4 * w as usize;
                    let skip = (width - phase) % width;
                    let plain = if level == 1 {
                        crate::record::quick(s, w as usize, skip)
                    } else {
                        crate::record::encode(s, w as usize, skip)
                    };
                    if level >= 5 {
                        let packed = crate::object::columns(s, w as usize, skip, level);
                        if packed.len() + (plain.len() >> if level >= 10 { 7 } else { 4 })
                            < plain.len()
                        {
                            return Ok(packed);
                        }
                    }
                    Ok(plain)
                };
                let b = if group.len() == 1 {
                    let p = group[0];
                    compress_group(&src[p.start..p.start + p.len])?
                } else {
                    SCRATCH.with_borrow_mut(|b| {
                        b.clear();
                        for p in group {
                            b.extend_from_slice(&src[p.start..p.start + p.len])
                        }
                        compress_group(b)
                    })?
                };
                if at == 0 && end == r.len() {
                    payload = b;
                    break;
                }
                let size = group.iter().map(|p| p.len).sum::<usize>();
                payload.extend((size as u32).to_le_bytes());
                payload.extend((b.len() as u32).to_le_bytes());
                payload.extend(b);
                at = end
            }
            if payload.len() > n + 1 {
                payload.clear();
                payload.push(0);
                for p in r {
                    payload.extend(&src[p.start..p.start + p.len])
                }
            }
            *out = std::borrow::Cow::Owned(payload);
            Ok(())
        },
    )?;
    Ok(Frame {
        size: src.len(),
        block,
        blocks,
        layout: Some(layout),
    })
}

pub(crate) fn restore(
    frame: &Frame,
    threads: usize,
    dst: &mut [u8],
    native: bool,
    f: impl Fn(&[u8], &mut [u8]) -> Result<()> + Sync,
) -> Result<()> {
    let layout = frame.layout.as_ref().unwrap();
    let mut chunks: Vec<Vec<(usize, &mut [u8])>> =
        (0..frame.blocks.len()).map(|_| vec![]).collect();
    let mut rest = dst;
    for &(id, pos, n) in &layout.deposits {
        let (a, b) = rest.split_at_mut(n);
        rest = b;
        chunks[id].push((pos, a))
    }
    let mut jobs: Vec<_> = chunks.into_iter().zip(&frame.blocks).enumerate().collect();
    jobs.sort_unstable_by_key(|(id, (_, src))| {
        let n = layout.sizes[*id];
        let score = n + src.len();
        std::cmp::Reverse(score)
    });

    parallel(
        threads.min(jobs.len()),
        jobs.into_iter(),
        |(id, (mut row, src))| {
            if row.len() == 1 {
                return f(src, row[0].1);
            }
            let n = frame.layout.as_ref().unwrap().sizes[id];
            if native
                && matches!(src.first(), Some(12 | 21 | 33))
                && row
                    .iter()
                    .all(|(at, d)| at % 4 == 0 && (d.len() % 4 == 0 || at + d.len() == n))
            {
                row.sort_unstable_by_key(|r| r.0);
                return if src[0] == 33 {
                    crate::object::unsmall(&src[1..], n, &mut row)
                } else if src[0] == 21 {
                    crate::object::unadaptive(&src[1..], n, &mut row)
                } else {
                    crate::object::unbanked_slices(&src[1..], n, &mut row)
                };
            }
            if native && src.first() == Some(&9) {
                row.sort_unstable_by_key(|r| r.0);
                return crate::object::uncolumns_slices(&src[1..], n, &mut row);
            }
            if native && src.first() == Some(&4) {
                row.sort_unstable_by_key(|r| r.0);
                return crate::record::decode_slices(&src[1..], n, &mut row);
            }
            if native && src.first() == Some(&0) && src.len() == n + 1 {
                for (at, d) in row {
                    d.copy_from_slice(&src[1 + at..1 + at + d.len()])
                }
                return Ok(());
            }
            SCRATCH.with_borrow_mut(|b| {
                b.resize(n, 0);
                f(src, b)?;
                for (at, d) in row {
                    d.copy_from_slice(&b[at..at + d.len()])
                }
                Ok(())
            })
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_index_covers_every_byte_once() {
        let pieces = vec![
            Piece {
                start: 300,
                len: 150,
                w: 0,
                phase: 0,
            },
            Piece {
                start: 0,
                len: 300,
                w: 0,
                phase: 0,
            },
            Piece {
                start: 450,
                len: 310,
                w: 0,
                phase: 0,
            },
        ];
        let plan = Layout::new(pieces, vec![365, 395]);
        let b = plan.index();
        let mut p = &b[..];
        let decoded = Layout::read(&mut p, 760).unwrap();
        assert!(p.is_empty());
        assert_eq!(decoded.index(), b);
        assert_eq!(plan.deposits, decoded.deposits);
        assert!(Layout::read(&mut &b[..], 761).is_err());
        assert!(Layout::read(&mut &b[..b.len() - 1], 760).is_err());
    }
}
