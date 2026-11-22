// Static order-zero rANS. Four independent 32-bit states share a 16-bit digit stream.
use crate::{Result, bad, take, u32in};
const LOG: u32 = 11;
const M: u32 = 1 << LOG;
const L: u32 = 1 << 16;
pub fn put_policy(data: &[u8], out: &mut Vec<u8>, raw_fraction: f64) {
    put_inner(data, out, raw_fraction, 0)
}
pub fn put_tight(data: &[u8], out: &mut Vec<u8>) {
    let mut best = Vec::new();
    put_inner(data, &mut best, 0.999, 0);
    if best.get(4).is_some_and(|&m| m == 4 || m == 5) {
        let mut exact = Vec::new();
        put_inner(data, &mut exact, 0.999, 2);
        if exact.len() + best.len() / 64 < best.len() {
            best = exact;
        }
    }
    out.extend(best);
}
pub(crate) fn put_alternative(data: &[u8], out: &mut Vec<u8>, mode: usize) {
    if mode == 1 {
        return put_tight(data, out);
    }
    let mut best = Vec::new();
    if mode == 2 || mode == 5 {
        put_inner(data, &mut best, 0.995, 1);
    } else if mode == 3 {
        put_inner(data, &mut best, 0.995, 2);
    } else {
        put_inner(data, &mut best, 0.995, 0);
    }
    if mode >= 4 {
        for m in 1..=2 {
            let mut v = Vec::new();
            put_inner(data, &mut v, 0.997, m);
            if v.len() + best.len() / 256 < best.len() {
                best = v;
            }
        }
    }
    out.extend(best);
}
fn put_inner(data: &[u8], out: &mut Vec<u8>, raw_fraction: f64, method: u8) {
    put_config(data, out, raw_fraction, method, (0, 0))
}
pub(crate) fn put_config(
    data: &[u8],
    out: &mut Vec<u8>,
    raw_fraction: f64,
    method: u8,
    entropy: (usize, usize),
) {
    let faster = entropy.0;
    let counts = if faster == 0 {
        let mut counts = [0u32; 256];
        for &b in data {
            counts[b as usize] += 1
        }
        counts
    } else {
        fast_counts(data)
    };
    let n = data.len();
    out.extend((n as u32).to_le_bytes());
    if n > 3 && counts.iter().any(|&v| v as usize == n) {
        out.push(1);
        out.push(data[0]);
        return;
    }
    if n < 192 {
        out.push(0);
        out.extend(data);
        return;
    }
    let mut freq = [0u32; 256];
    let mut sum = 0;
    let mut most = 0;
    let mut estimate = if faster == 2 && n != 0 {
        n as f64 * fast_log(n)
    } else {
        0.
    };
    for i in 0..256 {
        let c = counts[i];
        if c == 0 {
            continue;
        }
        if faster == 0 {
            freq[i] = ((c as u64 * M as u64 + n as u64 / 2) / n as u64).max(1) as u32;
            sum += freq[i];
            if c > counts[most] {
                most = i
            }
        }
        if faster == 2 {
            estimate -= c as f64 * fast_log(c as usize)
        } else {
            estimate += c as f64 * ((n as f64) / (c as f64)).log2()
        }
    }
    if estimate / 8. + 125. > n as f64 * raw_fraction {
        out.push(0);
        out.extend(data);
        return;
    }
    if method == 0 && n >= 1536 && crate::palette::put(data, &counts, out, estimate, entropy.1) {
        return;
    }
    if method != 2 && crate::huff::put(data, &counts, out, estimate) {
        return;
    }
    if faster != 0 {
        for (i, &c) in counts.iter().enumerate() {
            if c != 0 {
                freq[i] = ((u64::from(c) * u64::from(M) + n as u64 / 2) / n as u64).max(1) as u32;
                sum += freq[i];
                if c > counts[most] {
                    most = i
                }
            }
        }
    }
    if sum <= M {
        freq[most] += M - sum
    } else {
        while sum > M {
            let i = (0..256).max_by_key(|&i| freq[i]).unwrap();
            let k = (sum - M).min((freq[i] / 2).max(1));
            freq[i] -= k;
            sum -= k
        }
    }
    let mut starts = [0u32; 256];
    let mut inv = [0u64; 256];
    let mut acc = 0;
    let mut present = [0u8; 32];
    for i in 0..256 {
        starts[i] = acc;
        let f = freq[i];
        acc += f;
        if f != 0 {
            inv[i] = (1u64 << 32).div_ceil(f as u64);
            present[i / 8] |= 1 << (i % 8)
        }
    }
    let start = out.len();
    out.push(2);
    out.extend(present);
    for &f in &freq {
        if f != 0 {
            out.extend((f as u16).to_le_bytes())
        }
    }
    let mut states = [L; 4];
    let mut words = Vec::with_capacity(n / 2);
    for (i, &c) in data.iter().enumerate().rev() {
        let c = c as usize;
        let f = freq[c];
        let mut x = states[i & 3];
        if x >= (f << (32 - LOG)) {
            words.push(x as u16);
            x >>= 16
        }
        let mut q = ((x as u64 * inv[c]) >> 32) as u32;
        q -= u32::from((q as u64 * f as u64) > x as u64);
        states[i & 3] = (q << LOG) + (x - q * f) + starts[c];
    }
    for x in states {
        out.extend(x.to_le_bytes())
    }
    out.extend((words.len() as u32).to_le_bytes());
    for &w in words.iter().rev() {
        out.extend(w.to_le_bytes())
    }
    if out.len() - start > n {
        out.truncate(start);
        out.push(0);
        out.extend(data)
    }
}
fn fast_counts(data: &[u8]) -> [u32; 256] {
    let mut lanes = [[0u32; 256]; 4];
    let mut chunks = data.chunks_exact(4);
    for g in &mut chunks {
        for j in 0..4 {
            lanes[j][g[j] as usize] += 1
        }
    }
    for &v in chunks.remainder() {
        lanes[0][v as usize] += 1
    }
    std::array::from_fn(|j| lanes[0][j] + lanes[1][j] + lanes[2][j] + lanes[3][j])
}
#[inline]
fn fast_log(n: usize) -> f64 {
    // Linear interpolation of log2 on sixteen subintervals. This is used
    // only to choose a representation; all frequencies in the wire are exact.
    static LUT: std::sync::LazyLock<[f64; 17]> =
        std::sync::LazyLock::new(|| std::array::from_fn(|i| (1. + i as f64 / 16.).log2()));
    let e = (n as f64).to_bits();
    let exp = ((e >> 52) as i32 - 1023) as f64;
    let i = (e >> 48) as usize & 15;
    let part = ((e & ((1u64 << 48) - 1)) >> 25) as f64 * (1. / 8388608.);
    exp + LUT[i] + part * (LUT[i + 1] - LUT[i])
}
pub fn get<'a>(p: &mut &'a [u8], limit: usize) -> Result<std::borrow::Cow<'a, [u8]>> {
    get_context(p, limit, None)
}
pub fn get_context<'a>(
    p: &mut &'a [u8],
    limit: usize,
    ext: Option<&[u8]>,
) -> Result<std::borrow::Cow<'a, [u8]>> {
    use std::borrow::Cow;
    let n = u32in(p)?;
    if n > limit {
        return Err(bad());
    }
    let mode = take(p, 1)?[0];

    if mode == 22 || mode == 23 {
        if mode == 23 && ext.is_none() {
            return Err(bad());
        }
        return Ok(Cow::Owned(crate::context::get(
            p,
            n,
            if mode == 23 { ext } else { None },
        )?));
    }
    if mode == 0 {
        return Ok(Cow::Borrowed(take(p, n)?));
    }
    if mode == 1 {
        return Ok(Cow::Owned(vec![take(p, 1)?[0]; n]));
    }

    if mode == 5 {
        return Ok(Cow::Owned(crate::palette::get(p, n)?));
    }
    if mode == 4 {
        return Ok(Cow::Owned(crate::huff::get(p, n)?));
    }
    if mode != 2 {
        return Err(bad());
    }
    #[repr(C, align(8))]
    #[derive(Copy, Clone, Default)]
    struct Entry {
        f: u16,
        rem: u16,
        sym: u8,
    }
    let present = take(p, 32)?;
    let mut tab = [Entry::default(); M as usize];
    let mut i = 0;
    for c in 0..256 {
        if present[c / 8] >> (c % 8) & 1 != 0 {
            let f = u16::from_le_bytes(take(p, 2)?.try_into().unwrap());
            if f == 0 || i + f as usize > tab.len() {
                return Err(bad());
            }
            for r in 0..f {
                tab[i] = Entry {
                    f,
                    rem: r,
                    sym: c as u8,
                };
                i += 1
            }
        }
    }
    if i != tab.len() {
        return Err(bad());
    }
    let mut states = [0u32; 4];
    for x in &mut states {
        *x = u32in(p)? as u32;
        if *x < L {
            return Err(bad());
        }
    }
    let nw = u32in(p)?;
    let digits = take(p, nw.checked_mul(2).ok_or_else(bad)?)?;
    let mut at = 0;
    let mut out = vec![0u8; n];
    let mut k = 0;
    let mut xs = states;
    while k + 16 <= n && at + 32 <= digits.len() {
        for _ in 0..4 {
            let mut y = [0; 4];
            for j in 0..4 {
                let x = xs[j];
                let e = tab[(x & (M - 1)) as usize];
                out[k + j] = e.sym;
                y[j] = (x >> LOG) * e.f as u32 + e.rem as u32
            }
            for j in 0..4 {
                let w = unsafe { std::ptr::read_unaligned(digits.as_ptr().add(at).cast::<u16>()) }
                    .to_le() as u32;
                let small = y[j] < L;
                xs[j] = if small { (y[j] << 16) + w } else { y[j] };
                at += 2 * small as usize
            }
            k += 4;
        }
    }
    states = xs;
    for i in k..n {
        let x = states[i & 3];
        let e = tab[(x & (M - 1)) as usize];
        let mut y = (x >> LOG) * e.f as u32 + e.rem as u32;
        if y < L {
            let w = digits.get(at..at + 2).ok_or_else(bad)?;
            at += 2;
            y = (y << 16) + u16::from_le_bytes(w.try_into().unwrap()) as u32
        }
        states[i & 3] = y;
        out[i] = e.sym;
    }
    if at != digits.len() || states != [L; 4] {
        return Err(bad());
    }
    Ok(Cow::Owned(out))
}
