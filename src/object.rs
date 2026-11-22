// Reversible representations of fixed-width instructions and integer fields.
use crate::{Result, bad, lz, take, u32in};
pub(crate) fn archive_lz(src: &[u8], level: u8) -> Vec<u8> {
    let effort = crate::archive_parser(level);

    if level == 22 && effort >= 5 {
        // Smaller independent jobs keep the inverse in cache. At this
        // preset, also code streams whose savings fall below ten percent.
        lz::dense::<true>(src, effort, true)
    } else {
        lz::encode_level::<true>(src, effort)
    }
}
fn pack(trans: &[u8], hdr: &mut Vec<u8>, level: u8) {
    let b = archive_lz(trans, level);
    if b.len() <= trans.len() {
        hdr.extend(b)
    } else {
        hdr.push(0);
        hdr.extend(trans)
    }
}
fn unpack<'a>(p: &'a [u8], scratch: &'a mut Vec<u8>, n: usize) -> Result<&'a [u8]> {
    match p.split_first() {
        Some((0, s)) if s.len() == n => Ok(s),

        Some((1, s)) => {
            scratch.resize(n, 0);
            lz::decode(s, scratch)?;
            Ok(scratch)
        }
        _ => Err(bad()),
    }
}
pub(crate) fn instructions(s: &[u8], level: u8) -> Vec<u8> {
    let rows = s.len() / 4;
    let mut ct = [0u32; 64];
    for x in s[..rows * 4].chunks_exact(4) {
        ct[(x[3] >> 2) as usize] += 1;
    }
    let mut out = vec![8];
    let mut writes = [0usize; 64];
    let mut next = rows * 2;
    for (i, n) in ct.iter().enumerate() {
        out.extend(n.to_le_bytes());
        writes[i] = next;
        next += 2 * (*n as usize);
    }
    let mut t = vec![0; rows * 4];
    for (i, x) in s[..rows * 4].chunks_exact(4).enumerate() {
        t[i * 2..i * 2 + 2].copy_from_slice(&x[2..]);
        let at = &mut writes[(x[3] >> 2) as usize];
        t[*at..*at + 2].copy_from_slice(&x[..2]);
        *at += 2;
    }
    out.extend(&s[rows * 4..]);
    pack(&t, &mut out, level);
    out
}
pub(crate) fn uninstructions(mut p: &[u8], dst: &mut [u8]) -> Result<()> {
    let rows = dst.len() / 4;
    let mut starts = [0usize; 64];
    let mut ends = [0usize; 64];
    let mut next = rows * 2;
    for k in 0..64 {
        let ct = u32in(&mut p)?;
        if ct > rows {
            return Err(bad());
        }
        starts[k] = next;
        next += ct * 2;
        ends[k] = next;
    }
    if next != rows * 4 {
        return Err(bad());
    }
    let tail = dst.len() - rows * 4;
    dst[rows * 4..].copy_from_slice(take(&mut p, tail)?);
    let mut tmp = vec![];
    let t = unpack(p, &mut tmp, rows * 4)?;
    for (i, x) in dst[..rows * 4].chunks_exact_mut(4).enumerate() {
        let h = &t[i * 2..i * 2 + 2];
        let c = (h[1] >> 2) as usize;
        let at = starts[c];
        if at >= ends[c] {
            return Err(bad());
        }
        x[..2].copy_from_slice(&t[at..at + 2]);
        x[2..].copy_from_slice(h);
        starts[c] += 2;
    }
    Ok(())
}
#[inline(always)]
fn val(s: &[u8], i: usize) -> u32 {
    u32::from_le_bytes(s[i..i + 4].try_into().unwrap())
}
pub(crate) fn columns(s: &[u8], w: usize, skip: usize, level: u8) -> Vec<u8> {
    let skip = skip.min(s.len());
    let rows = (s.len() - skip) / (4 * w);
    let end = skip + rows * 4 * w;
    let mut mask = 0u8;
    let mut t = vec![0; rows * 4 * w];
    for f in 0..w {
        let mut scores = [0u64; 2];
        let mut prev = 0u32;
        for i in (0..rows).step_by((rows / 1024).max(1)) {
            let at = skip + 4 * (i * w + f);
            let x = val(s, at);
            let d = if i == 0 {
                x
            } else {
                x.wrapping_sub(val(s, at - w * 4))
            };
            scores[0] += (x.leading_zeros() + x.trailing_zeros()) as u64;
            scores[1] += (d.leading_zeros().max((!d).leading_zeros()) + d.trailing_zeros()) as u64;
        }
        let diff = scores[1] > scores[0];
        if diff {
            mask |= 1 << f;
        }
        for i in 0..rows {
            let x = val(s, skip + 4 * (i * w + f));
            let v = if diff { x.wrapping_sub(prev) } else { x };
            prev = x;
            for (b, y) in v.to_le_bytes().into_iter().enumerate() {
                t[(f * 4 + b) * rows + i] = y;
            }
        }
    }
    let mut out = vec![9, w as u8, skip as u8, mask];
    out.extend(&s[..skip]);
    out.extend(&s[end..]);
    pack(&t, &mut out, level);
    out
}
pub(crate) fn uncolumns(p: &[u8], dest: &mut [u8]) -> Result<()> {
    uncolumns_slices(p, dest.len(), &mut [(0, dest)])
}
pub(crate) fn uncolumns_slices(
    mut p: &[u8],
    len: usize,
    spans: &mut [(usize, &mut [u8])],
) -> Result<()> {
    let h = take(&mut p, 3)?;
    let w = h[0] as usize;
    let skip = h[1] as usize;
    let mask = h[2];
    if w == 0 || w > 8 || skip > len || skip >= w * 4 {
        return Err(bad());
    }
    let rows = (len - skip) / (4 * w);
    let mut sink = crate::record::Sink::new(spans);
    sink.write(take(&mut p, skip)?);
    let tail = take(&mut p, len - skip - rows * 4 * w)?;
    let mut scratch = vec![];
    let t = unpack(p, &mut scratch, rows * 4 * w)?;
    let mut tile = [[0u32; 128]; 8];
    let mut tmp = [0u8; 4096];
    let mut previous = [0u32; 8];
    for j in (0..rows).step_by(128) {
        let n = (rows - j).min(128);
        for f in 0..w {
            let q = &t[f * 4 * rows..(f + 1) * 4 * rows];
            let v = &mut tile[f];
            let mut k = 0;
            #[cfg(target_arch = "aarch64")]
            unsafe {
                use std::arch::aarch64::*;
                while k + 16 <= n {
                    let at = q.as_ptr().add(j + k);
                    let fields = uint8x16x4_t(
                        vld1q_u8(at),
                        vld1q_u8(at.add(rows)),
                        vld1q_u8(at.add(2 * rows)),
                        vld1q_u8(at.add(3 * rows)),
                    );
                    vst4q_u8(v.as_mut_ptr().add(k).cast(), fields);
                    k += 16;
                }
            }
            for (i, value) in v.iter_mut().enumerate().take(n).skip(k) {
                let at = j + i;
                *value =
                    u32::from_le_bytes([q[at], q[rows + at], q[rows * 2 + at], q[rows * 3 + at]]);
            }
            if mask & (1 << f) != 0 {
                let mut prev = previous[f];
                let mut i = 0;
                #[cfg(target_arch = "aarch64")]
                unsafe {
                    use std::arch::aarch64::*;
                    let z = vdupq_n_u32(0);
                    let mut carry = vdupq_n_u32(prev);
                    while i + 4 <= n {
                        let ptr = v.as_mut_ptr().add(i);
                        let a = vld1q_u32(ptr);
                        let a = vaddq_u32(a, vextq_u32::<3>(z, a));
                        let a = vaddq_u32(a, vextq_u32::<2>(z, a));
                        let a = vaddq_u32(a, carry);
                        vst1q_u32(ptr, a);
                        carry = vdupq_laneq_u32::<3>(a);
                        i += 4;
                    }
                    prev = vgetq_lane_u32::<0>(carry);
                }
                for a in &mut v[i..n] {
                    prev = prev.wrapping_add(*a);
                    *a = prev;
                }
                previous[f] = prev;
            }
        }
        sink.tile(&tile, w, n, &mut tmp);
    }
    sink.write(tail);
    Ok(())
}

// Condition an instruction's immediate on both opcode and base register.
// Consecutive immediates in those groups often address neighboring fields.
const BANKS: usize = 2048;
#[inline(always)]
fn bank<const B: usize>(hi: u16) -> usize {
    ((hi as usize >> 10) << B) | (hi as usize & ((1 << B) - 1))
}
fn vint(mut v: usize, b: &mut Vec<u8>) {
    while v >= 128 {
        b.push((v as u8) | 128);
        v >>= 7;
    }
    b.push(v as u8);
}
// Estimate how many literal bytes and match references the substream needs.
// This is only an encoder-side choice; the decoder gets one bit per bank.
fn bank_cost(s: &[u8]) -> usize {
    if s.len() < 6 {
        return s.len() * 8;
    }
    let mut table = [usize::MAX; 512];
    let mut c = [0u32; 256];
    let mut refs = 0usize;
    let mut i = 0;
    while i + 4 <= s.len() {
        let key = u32::from_le_bytes(s[i..i + 4].try_into().unwrap());
        let h = (key.wrapping_mul(0x9e3779b1) >> 23) as usize;
        let p = std::mem::replace(&mut table[h], i);
        if p != usize::MAX && s[p..p + 4] == s[i..i + 4] {
            let mut j = 4;
            while i + j < s.len() && s[p + j] == s[i + j] {
                j += 1;
            }
            refs += 9 + (i - p).ilog2() as usize + (j - 3).ilog2() as usize;
            i += j;
            continue;
        }
        c[s[i] as usize] += 1;
        i += 1;
    }
    for &v in &s[i..] {
        c[v as usize] += 1;
    }
    let n = c.iter().sum::<u32>();
    refs + c
        .into_iter()
        .filter(|&v| v != 0)
        .map(|v| (v as f64 * (n as f64 / v as f64).log2()) as usize)
        .sum::<usize>()
}
pub(crate) fn banked(s: &[u8], level: u8) -> Vec<u8> {
    match 5 {
        0 => banked_bits::<0>(s, level),
        1 => banked_bits::<1>(s, level),
        2 => banked_bits::<2>(s, level),
        3 => banked_bits::<3>(s, level),
        4 => banked_bits::<4>(s, level),
        6 => {
            let a = banked_bits::<3>(s, level);
            let b = banked_bits::<5>(s, level);
            if a.len() <= b.len() { a } else { b }
        }
        _ => banked_bits::<5>(s, level),
    }
}
fn banked_bits<const B: usize>(s: &[u8], level: u8) -> Vec<u8> {
    let active = 64 << B;
    let n = s.len() / 4;
    let mut count = [0usize; BANKS];
    for x in s[..4 * n].chunks_exact(4) {
        count[bank::<B>(u16::from_le_bytes([x[2], x[3]]))] += 1;
    }
    let mut next = 2 * n;
    let mut at = [0usize; BANKS];
    let mut hdr = vec![12];
    for (i, &c) in count[..active].iter().enumerate() {
        at[i] = next;
        next += c * 2;
        vint(c, &mut hdr);
    }
    let strategy = (if level >= 17 { 2 } else { 0 }).max(usize::from(B != 5));
    let mut t = vec![0; 4 * n];
    let mut prev = [0u16; BANKS];
    for (i, x) in s[..4 * n].chunks_exact(4).enumerate() {
        let c = bank::<B>(u16::from_le_bytes([x[2], x[3]]));
        let v = u16::from_le_bytes([x[0], x[1]]);
        let p = at[c];
        let delta = v.wrapping_sub(prev[c]);
        prev[c] = v;
        at[c] += 2;
        t[2 * i..2 * i + 2].copy_from_slice(&x[2..]);
        let emit = if strategy != 0 { v } else { delta };
        t[p..p + 2].copy_from_slice(&emit.to_le_bytes());
    }
    if strategy != 0 {
        let mut mask = [0u8; BANKS / 8];
        let mut changed = vec![];
        let mut pos = n * 2;
        for (bank, &ct) in count[..active].iter().enumerate() {
            let stream = &mut t[pos..pos + 2 * ct];
            pos += 2 * ct;
            if ct < 3 {
                continue;
            }
            changed.clear();
            let mut last = 0u16;
            for pair in stream.chunks_exact(2) {
                let x = u16::from_le_bytes([pair[0], pair[1]]);
                changed.extend(x.wrapping_sub(last).to_le_bytes());
                last = x;
            }
            let raw_cost = bank_cost(stream);
            let delta_cost = bank_cost(&changed);
            if delta_cost + if strategy == 2 { ct / 2 } else { 0 } < raw_cost {
                mask[bank / 8] |= 1 << (bank % 8);
                stream.copy_from_slice(&changed);
            }
        }
        hdr[0] = 21;
        hdr.splice(1..1, mask[..active / 8].iter().copied());
        if B != 5 {
            hdr[0] = 33;
            hdr.insert(1, B as u8);
        }
    }
    hdr.extend(&s[n * 4..]);
    pack(&t, &mut hdr, level);
    hdr
}
thread_local! {static TEMP:std::cell::RefCell<Vec<u8>>=const{std::cell::RefCell::new(Vec::new())};}

pub(crate) fn unbanked(p: &[u8], d: &mut [u8]) -> Result<()> {
    let n = d.len();
    unbanked_slices(p, n, &mut [(0, d)])
}
// Prefix sums are independent within each bank. After the scan, restoring
// a word only needs its upper halfword and a single indexed lower halfword.
pub(crate) fn unbanked_slices(p: &[u8], n: usize, spans: &mut [(usize, &mut [u8])]) -> Result<()> {
    restore_banks::<5>(p, n, spans, None)
}
pub(crate) fn unadaptive(mut p: &[u8], n: usize, spans: &mut [(usize, &mut [u8])]) -> Result<()> {
    let mask = take(&mut p, BANKS / 8)?;
    restore_banks::<5>(p, n, spans, Some(mask))
}
pub(crate) fn unsmall(mut p: &[u8], n: usize, spans: &mut [(usize, &mut [u8])]) -> Result<()> {
    let bits = take(&mut p, 1)?[0];
    if bits > 4 {
        return Err(bad());
    }
    let m = take(&mut p, (64 << bits) / 8)?;
    match bits {
        0 => restore_banks::<0>(p, n, spans, Some(m)),
        1 => restore_banks::<1>(p, n, spans, Some(m)),
        2 => restore_banks::<2>(p, n, spans, Some(m)),
        3 => restore_banks::<3>(p, n, spans, Some(m)),
        _ => restore_banks::<4>(p, n, spans, Some(m)),
    }
}
fn restore_banks<const B: usize>(
    mut p: &[u8],
    n: usize,
    spans: &mut [(usize, &mut [u8])],
    mask: Option<&[u8]>,
) -> Result<()> {
    let rows = n / 4;
    let mut next = rows * 2;
    let mut loc = [0u32; BANKS];
    let mut ends = [0u32; BANKS];
    for i in 0..(64 << B) {
        let mut ct = 0usize;
        for shift in (0..=21).step_by(7) {
            let x = take(&mut p, 1)?[0];
            ct |= ((x & 127) as usize) << shift;
            if x < 128 {
                break;
            }
            if shift == 21 {
                return Err(bad());
            }
        }
        loc[i] = next as u32;
        next += 2 * ct;
        if next > rows * 4 {
            return Err(bad());
        }
        ends[i] = next as u32;
    }
    if next != rows * 4 {
        return Err(bad());
    }
    let tail = take(&mut p, n - rows * 4)?;
    let restore = |t: &mut Vec<u8>| -> Result<()> {
        match p.split_first() {
            Some((0, raw)) if raw.len() == 4 * rows => {
                t.clear();
                t.extend(raw);
            }
            Some((1, src)) => {
                t.resize(rows * 4, 0);
                lz::decode(src, t)?;
            }

            Some((18, src)) => unrank(src, t, rows)?,
            _ => return Err(bad()),
        }

        let short_delta = cfg!(all(target_arch = "aarch64", target_endian = "little"))
            && B == 5
            && mask.is_none()
            && n <= 128 * 1024;
        let skip_trivial = short_delta;

        for (bank, (&start, &end)) in loc[..64 << B].iter().zip(&ends).enumerate() {
            // The previous halfword is zero at the start of every bank.
            if skip_trivial && end - start <= 2 {
                continue;
            }
            if mask.is_some_and(|m| m[bank / 8] >> (bank % 8) & 1 == 0) {
                continue;
            }
            let lo = &mut t[start as usize..end as usize];
            let mut i = 0;
            let mut prev: u16;
            #[cfg(not(target_arch = "aarch64"))]
            {
                prev = 0;
            }
            #[cfg(target_arch = "aarch64")]
            unsafe {
                use std::arch::aarch64::*;
                let z = vdupq_n_u16(0);
                let mut carry = z;
                while i + 16 <= lo.len() {
                    let ptr = lo.as_mut_ptr().add(i).cast::<u16>();
                    let v = vld1q_u16(ptr);
                    let v = vaddq_u16(v, vextq_u16::<7>(z, v));
                    let v = vaddq_u16(v, vextq_u16::<6>(z, v));
                    let v = vaddq_u16(v, vextq_u16::<4>(z, v));
                    let v = vaddq_u16(v, carry);
                    vst1q_u16(ptr, v);
                    carry = vdupq_laneq_u16::<7>(v);
                    i += 16;
                }
                prev = vgetq_lane_u16::<0>(carry);
            }
            for x in lo[i..].chunks_exact_mut(2) {
                prev = prev.wrapping_add(u16::from_le_bytes([x[0], x[1]]));
                x.copy_from_slice(&prev.to_le_bytes());
            }
        }

        #[cfg(all(target_arch = "aarch64", target_endian = "little"))]
        if short_delta {
            return write_banks_hint::<128, B, true, false>(t, rows, spans, loc, ends, tail);
        }
        match usize::from(n >= 128 * 1024) {
            #[cfg(all(target_arch = "aarch64", target_endian = "little"))]
            1 if mask.is_some() && (128 * 1024..=704 * 1024).contains(&n) => {
                return write_banks::<64, B, true>(t, rows, spans, loc, ends, tail);
            }
            1 => return write_banks::<64, B, false>(t, rows, spans, loc, ends, tail),
            2 => return write_banks::<128, B, false>(t, rows, spans, loc, ends, tail),
            3 => return write_banks::<256, B, false>(t, rows, spans, loc, ends, tail),
            _ => {}
        }
        let mut row = 0;
        for (offset, d) in spans.iter_mut() {
            if *offset != row * 4 || (d.len() % 4 != 0 && *offset + d.len() != n) {
                return Err(bad());
            }
            let full = d.len() / 4;
            for (base, chunk) in d[..full * 4].chunks_mut(128).enumerate() {
                let k = chunk.len() / 4;
                let mut positions = [0u32; 32];
                let h = &t[(row + base * 32) * 2..(row + base * 32 + k) * 2];
                for (j, hi) in h.chunks_exact(2).enumerate() {
                    let c = bank::<B>(u16::from_le_bytes([hi[0], hi[1]]));
                    let at = loc[c];
                    positions[j] = at;
                    if at >= ends[c] {
                        return Err(bad());
                    }
                    loc[c] = at + 2;
                    #[cfg(target_arch = "aarch64")]
                    unsafe {
                        std::arch::asm!("prfm pldl1keep, [{p}]",p=in(reg)t.as_ptr().add(at as usize),options(readonly,nostack,preserves_flags));
                    }
                }
                for (j, &pos) in positions[..k].iter().enumerate() {
                    let at = pos as usize;
                    // All positions were checked against even bank ends above.
                    unsafe {
                        let low = t.as_ptr().add(at).cast::<u16>().read_unaligned().to_le() as u32;
                        let high =
                            h.as_ptr().add(j * 2).cast::<u16>().read_unaligned().to_le() as u32;
                        chunk
                            .as_mut_ptr()
                            .add(4 * j)
                            .cast::<u32>()
                            .write_unaligned(((high << 16) | low).to_le());
                    }
                }
            }
            row += full;
            if d.len() % 4 != 0 {
                d[full * 4..].copy_from_slice(tail);
            }
        }
        if row != rows {
            return Err(bad());
        }
        Ok(())
    };
    TEMP.with_borrow_mut(restore)
}

// Build the gather addresses in one pass, then fetch the low words and
// interleave them with contiguous upper words. Per-bank totals can be
// checked once at the end: only the global bound is needed for safe reads.
fn write_banks<const TILE: usize, const B: usize, const VEC: bool>(
    t: &[u8],
    rows: usize,
    spans: &mut [(usize, &mut [u8])],
    loc: [u32; BANKS],
    ends: [u32; BANKS],
    tail: &[u8],
) -> Result<()> {
    write_banks_hint::<TILE, B, VEC, true>(t, rows, spans, loc, ends, tail)
}
fn write_banks_hint<const TILE: usize, const B: usize, const VEC: bool, const HINT: bool>(
    t: &[u8],
    rows: usize,
    spans: &mut [(usize, &mut [u8])],
    mut loc: [u32; BANKS],
    ends: [u32; BANKS],
    tail: &[u8],
) -> Result<()> {
    let bound = 4 * rows;
    #[cfg(target_arch = "aarch64")]
    let hint = usize::from(HINT) != 0;
    let mut positions = [0u32; TILE];
    let mut row = 0;
    for (offset, d) in spans {
        if *offset != row * 4 || (d.len() % 4 != 0 && *offset + d.len() != bound + tail.len()) {
            return Err(bad());
        }
        let full = d.len() / 4;
        for chunk in d[..full * 4].chunks_mut(TILE * 4) {
            let k = chunk.len() / 4;
            let h = &t[2 * row..2 * (row + k)];
            let mut beyond = 0;
            let mut keys = [0u16; TILE];
            #[cfg(all(target_arch = "aarch64", target_endian = "little"))]
            if VEC {
                // The high halfwords are contiguous. Extract eight bank IDs
                // before the indexed loads of bank cursors.
                use std::arch::aarch64::*;
                let mut j = 0;
                while j + 8 <= k {
                    unsafe {
                        let high = vld1q_u16(h.as_ptr().add(2 * j).cast());
                        let key = vorrq_u16(
                            vmulq_n_u16(vshrq_n_u16::<10>(high), 1 << B),
                            vandq_u16(high, vdupq_n_u16((1 << B) - 1)),
                        );
                        vst1q_u16(keys.as_mut_ptr().add(j), key);
                    }
                    j += 8;
                }
                for j in j..k {
                    keys[j] = bank::<B>(u16::from_le_bytes([h[2 * j], h[2 * j + 1]])) as u16;
                }
            }
            for (j, (pos, bytes)) in positions[..k].iter_mut().zip(h.chunks_exact(2)).enumerate() {
                let c = if VEC {
                    keys[j] as usize
                } else {
                    bank::<B>(u16::from_le_bytes([bytes[0], bytes[1]]))
                };
                let at = loc[c];
                loc[c] = at + 2;
                *pos = at;
                beyond = beyond.max(at);
                #[cfg(target_arch = "aarch64")]
                if hint {
                    unsafe {
                        core::arch::asm!("prfm pldl1keep, [{p}]",p=in(reg)t.as_ptr().wrapping_add(at as usize),options(readonly,nostack,preserves_flags));
                    }
                }
            }
            if beyond as usize >= bound {
                return Err(bad());
            }
            // Every saved offset is even, so the largest validated offset
            // leaves two readable bytes. Both destinations are disjoint.
            #[cfg(all(target_arch = "aarch64", target_endian = "little"))]
            let first_scalar = if VEC {
                use std::arch::aarch64::*;
                let mut low = [0u16; TILE];
                for (v, &pos) in low[..k].iter_mut().zip(&positions[..k]) {
                    // All offsets are even and below the validated bound.
                    *v = unsafe { t.as_ptr().add(pos as usize).cast::<u16>().read_unaligned() };
                }
                let full = k & !7;
                for j in (0..full).step_by(8) {
                    unsafe {
                        let pair = uint16x8x2_t(
                            vld1q_u16(low.as_ptr().add(j)),
                            vld1q_u16(h.as_ptr().add(2 * j).cast()),
                        );
                        vst2q_u16(chunk.as_mut_ptr().add(4 * j).cast(), pair);
                    }
                }
                full
            } else {
                0
            };
            #[cfg(not(all(target_arch = "aarch64", target_endian = "little")))]
            let first_scalar = 0;
            for (j, &pos) in positions[..k].iter().enumerate().skip(first_scalar) {
                unsafe {
                    let lo = t
                        .as_ptr()
                        .add(pos as usize)
                        .cast::<u16>()
                        .read_unaligned()
                        .to_le() as u32;
                    let hi = h.as_ptr().add(2 * j).cast::<u16>().read_unaligned().to_le() as u32;
                    chunk
                        .as_mut_ptr()
                        .add(4 * j)
                        .cast::<u32>()
                        .write_unaligned(((hi << 16) | lo).to_le());
                }
            }
            row += k;
        }
        if d.len() % 4 != 0 {
            d[full * 4..].copy_from_slice(tail);
        }
    }
    if row != rows || loc != ends {
        return Err(bad());
    }
    Ok(())
}

// Give common immediate differences a byte code before the LZ stage. The
// escape payload is separate, so rare 16-bit values do not break long runs.
fn unrank(mut p: &[u8], raw: &mut Vec<u8>, rows: usize) -> Result<()> {
    let entries = take(&mut p, 1)?[0] as usize;
    let mut tab = [0u16; 256];
    for e in &mut tab[..entries] {
        *e = u16::from_le_bytes(take(&mut p, 2)?.try_into().unwrap());
    }
    let ex = u32in(&mut p)?;
    if ex > rows {
        return Err(bad());
    }
    let mut tmp = vec![];
    let t = unpack(p, &mut tmp, 3 * rows + 2 * ex)?;
    raw.resize(4 * rows, 0);
    raw[..2 * rows].copy_from_slice(&t[..2 * rows]);
    let codes = &t[2 * rows..3 * rows];
    let lo = &t[3 * rows..3 * rows + ex];
    let hi = &t[3 * rows + ex..];
    let mut j = 0;
    for (c, d) in codes.iter().zip(raw[2 * rows..].chunks_exact_mut(2)) {
        let v = if (*c as usize) < entries {
            tab[*c as usize]
        } else if *c == 255 && j < ex {
            let v = u16::from_le_bytes([lo[j], hi[j]]);
            j += 1;
            v
        } else {
            return Err(bad());
        };
        d.copy_from_slice(&v.to_le_bytes());
    }
    if j != ex {
        return Err(bad());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bank_and_plane_scatter_roundtrip() {
        let mut state = 1u32;
        let bytes: Vec<_> = (0..32773)
            .map(|_| {
                state = state.wrapping_mul(1664525).wrapping_add(1013904223);
                (state >> 24) as u8
            })
            .collect();
        for n in [129, 8192, 32773] {
            let b = banked(&bytes[..n], 4);
            let mut d = vec![0; n];
            let (lo, hi) = d.split_at_mut(48);
            unbanked_slices(&b[1..], n, &mut [(0, lo), (48, hi)]).unwrap();
            assert_eq!(d, &bytes[..n]);
            assert!(unbanked(&b[1..b.len() - 1], &mut d).is_err());
        }
        for b in [
            banked_bits::<0>(&bytes, 18),
            banked_bits::<3>(&bytes, 18),
            banked_bits::<4>(&bytes, 18),
        ] {
            let mut out = vec![0; bytes.len()];
            let (first, rest) = out.split_at_mut(100);
            assert_eq!(b[0], 33);
            unsmall(&b[1..], bytes.len(), &mut [(0, first), (100, rest)]).unwrap();
            assert_eq!(out, bytes);
        }
        for w in [1, 2, 4, 6, 8] {
            let src = &bytes[333..];
            let b = columns(src, w, 3, 4);
            let mut d = vec![0; src.len()];
            let (lo, hi) = d.split_at_mut(17);
            uncolumns_slices(&b[1..], src.len(), &mut [(0, lo), (17, hi)]).unwrap();
            assert_eq!(d, src);
            assert!(uncolumns(&b[1..b.len() - 1], &mut d).is_err());
        }
    }
    #[cfg(all(target_arch = "aarch64", target_endian = "little"))]
    #[test]
    fn adaptive_banks_restore_across_partial_vector_spans() {
        let n = 144_029;
        let mut bytes = Vec::with_capacity(n);
        for i in 0..n / 4 + 1 {
            let high = (((i * 19) as u16) << 10) | ((i % 31) as u16);
            let low = (i as u16).wrapping_mul(173);
            bytes.extend(low.to_le_bytes());
            bytes.extend(high.to_le_bytes());
        }
        bytes.truncate(n);
        let wire = banked_bits::<5>(&bytes, 20);
        assert_eq!(wire[0], 21);
        let mut out = vec![0xff; n];
        let (first, rest) = out.split_at_mut(68);
        let (mid, tail) = rest.split_at_mut(260);
        unadaptive(&wire[1..], n, &mut [(0, first), (68, mid), (328, tail)]).unwrap();
        assert_eq!(out, bytes);
    }
}
