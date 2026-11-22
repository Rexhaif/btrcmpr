// Learn a small set of conditional byte alphabets. A single ordinary entropy
// stream contains ranks; each decoder lookup emits a byte and the state for
// the next lookup. Eight independent segments hide that dependency.
use crate::{Result, ans, bad, take};
fn spans(n: usize, lane: usize) -> (usize, usize) {
    (
        lane * (n / 8),
        if lane == 7 { n } else { (lane + 1) * (n / 8) },
    )
}
fn ranked(count: &[u32]) -> Vec<u8> {
    let mut ids = (0..=255).collect::<Vec<_>>();
    ids.sort_unstable_by_key(|&x| (std::cmp::Reverse(count[x as usize]), x));
    ids
}
pub(crate) fn candidate(data: &[u8], external: Option<&[u8]>) -> Vec<u8> {
    let n = data.len();
    let mut global = [0u32; 256];
    let mut keyct = [0u32; 256];
    for &b in data {
        global[b as usize] += 1;
    }
    for &b in external.unwrap_or(data) {
        keyct[b as usize] += 1;
    }
    let mut keys = ranked(&keyct);
    keys.truncate(if n >= 200000 {
        31
    } else if n >= 44000 {
        15
    } else {
        7
    });
    let rows = keys.len() + 1;
    let k = if n >= 90000 { 64 } else { 32 };
    let mut class = [0u8; 256];
    for (i, &b) in keys.iter().enumerate() {
        class[b as usize] = (i + 1) as u8;
    }
    let mut counts = vec![[0u32; 256]; rows];
    let process = |v: &mut dyn FnMut(usize, usize, u8)| {
        for lane in 0..8 {
            let (a, b) = spans(n, lane);
            let mut prev = 0;
            for (j, &x) in data[a..b].iter().enumerate() {
                let ctx = external.map_or(prev, |t| t[a + j]);
                v(a + j, class[ctx as usize] as usize, x);
                prev = x;
            }
        }
    };
    process(&mut |_, c, v| counts[c][v as usize] += 1);
    let base = ranked(&global);
    let mut wire = Vec::new();
    wire.extend((n as u32).to_le_bytes());
    wire.extend([
        if external.is_some() { 23 } else { 22 },
        (rows - 1) as u8,
        k as u8,
    ]);
    wire.extend(keys);
    wire.extend(&base);
    let mut enc = vec![[0u8; 256]; rows];
    for (c, ct) in counts.iter().enumerate() {
        let sort = ranked(ct);
        let mut used = [false; 256];
        for (r, &s) in sort[..k].iter().enumerate() {
            used[s as usize] = true;
            enc[c][s as usize] = r as u8;
            wire.push(s);
        }
        let mut rank = k;
        for &s in &base {
            if !used[s as usize] {
                enc[c][s as usize] = rank as u8;
                rank += 1;
            }
        }
    }
    let mut ranks = vec![0; n];
    process(&mut |i, c, v| ranks[i] = enc[c][v as usize]);
    ans::put_tight(&ranks, &mut wire);
    wire
}
pub(crate) fn get(p: &mut &[u8], n: usize, external: Option<&[u8]>) -> Result<Vec<u8>> {
    let f = take(p, 2)?;
    let rows = 1 + f[0] as usize;
    let k = f[1] as usize;
    if rows > 32 || k == 0 || k > 64 || external.is_some_and(|v| v.len() != n) {
        return Err(bad());
    }
    let mut class = [0usize; 256];
    for (i, &s) in take(p, rows - 1)?.iter().enumerate() {
        if class[s as usize] != 0 {
            return Err(bad());
        }
        class[s as usize] = (i + 1) << 8;
    }
    let base = take(p, 256)?;
    let mut unique = [false; 256];
    for &s in base {
        if std::mem::replace(&mut unique[s as usize], true) {
            return Err(bad());
        }
    }
    let mut lut = vec![0u16; rows * 256];
    for row in lut.chunks_exact_mut(256) {
        let mut used = [false; 256];
        let top = take(p, k)?;
        for (r, &s) in top.iter().enumerate() {
            if std::mem::replace(&mut used[s as usize], true) {
                return Err(bad());
            }
            row[r] = (class[s as usize] | s as usize) as u16;
        }
        let mut j = k;
        for &s in base {
            if !used[s as usize] {
                row[j] = (class[s as usize] | s as usize) as u16;
                j += 1;
            }
        }
    }
    if !matches!(p.get(4), Some(0 | 1 | 2 | 4 | 5)) {
        return Err(bad());
    }
    let ranks = ans::get(p, n)?;
    if ranks.len() != n {
        return Err(bad());
    }
    let mut out = vec![0u8; n];
    if let Some(ext) = external {
        for ((d, &r), &c) in out.iter_mut().zip(ranks.iter()).zip(ext) {
            *d = lut[class[c as usize] | r as usize] as u8;
        }
    } else {
        let stride = n / 8;
        let mut state = [class[0] as u16; 8];
        for j in 0..stride {
            for (lane, st) in state.iter_mut().enumerate() {
                let at = lane * stride + j;
                let id = (*st & 0xff00) as usize | ranks[at] as usize;
                *st = lut[id];
                out[at] = *st as u8;
            }
        }
        for at in stride * 8..n {
            let id = (state[7] & 0xff00) as usize | ranks[at] as usize;
            state[7] = lut[id];
            out[at] = state[7] as u8;
        }
    }
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn internal_and_external_alphabets_with_unequal_lanes() {
        let mut rng = 71u32;
        let ctx: Vec<_> = (0..65003)
            .map(|_| {
                rng = rng.wrapping_mul(1664525).wrapping_add(1013904223);
                (rng >> 28) as u8
            })
            .collect();
        let data: Vec<_> = ctx
            .iter()
            .enumerate()
            .map(|(i, &b)| b.wrapping_mul(23).wrapping_add((i % 3) as u8))
            .collect();
        for c in [None, Some(ctx.as_slice())] {
            let b = candidate(&data, c);
            let mut p = &b[..];
            assert_eq!(&*ans::get_context(&mut p, data.len(), c).unwrap(), &data);
            assert!(p.is_empty());
            assert!(ans::get_context(&mut &b[..b.len() - 1], data.len(), c).is_err());
        }
    }
}
