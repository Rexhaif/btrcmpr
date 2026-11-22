// Context-sort a string terminated by a unique smallest symbol. Bucket
// induction sorts suffixes from transitions between descending/ascending runs.
use crate::{Result, ans, bad, take, u32in};
const EMPTY: u32 = u32::MAX;
pub(crate) fn suffixes(x: &[u32], alphabet: usize) -> Vec<u32> {
    let n = x.len();
    if n == 1 {
        return vec![0];
    }
    let mut st = vec![false; n];
    st[n - 1] = true;
    for i in (0..n - 1).rev() {
        st[i] = x[i] < x[i + 1] || (x[i] == x[i + 1] && st[i + 1]);
    }
    let is_mark = |i: usize| i != 0 && st[i] && !st[i - 1];
    let marks: Vec<u32> = (1..n).filter(|&i| is_mark(i)).map(|i| i as u32).collect();
    let mut ct = vec![0usize; alphabet];
    for &v in x {
        ct[v as usize] += 1;
    }
    let mut starts = vec![0; alphabet];
    let mut ends = vec![0; alphabet];
    let mut k = 0;
    for i in 0..alphabet {
        starts[i] = k;
        k += ct[i];
        ends[i] = k;
    }
    let mut sa = vec![EMPTY; n];
    let induce = |order: &[u32], sa: &mut [u32]| {
        sa.fill(EMPTY);
        let mut tail = ends.clone();
        for &j in order.iter().rev() {
            let c = x[j as usize] as usize;
            tail[c] -= 1;
            sa[tail[c]] = j;
        }
        let mut head = starts.clone();
        for i in 0..n {
            let q = sa[i];
            if q != 0 && q != EMPTY {
                let j = q as usize - 1;
                if !st[j] {
                    let c = x[j] as usize;
                    sa[head[c]] = j as u32;
                    head[c] += 1;
                }
            }
        }
        tail.copy_from_slice(&ends);
        for i in (0..n).rev() {
            let q = sa[i];
            if q != 0 && q != EMPTY {
                let j = q as usize - 1;
                if st[j] {
                    let c = x[j] as usize;
                    tail[c] -= 1;
                    sa[tail[c]] = j as u32;
                }
            }
        }
    };
    induce(&marks, &mut sa);
    let sorted: Vec<_> = sa
        .iter()
        .copied()
        .filter(|&i| is_mark(i as usize))
        .collect();
    let mut label = vec![0u32; n.div_ceil(2)];
    let mut rank = 0;
    for (k, &p) in sorted.iter().enumerate() {
        if k > 0 {
            let a = sorted[k - 1] as usize;
            let b = p as usize;
            let mut j = 0;
            loop {
                if x[a + j] != x[b + j] || st[a + j] != st[b + j] {
                    rank += 1;
                    break;
                }
                if j > 0 && (is_mark(a + j) || is_mark(b + j)) {
                    if is_mark(a + j) != is_mark(b + j) {
                        rank += 1;
                    }
                    break;
                }
                j += 1;
            }
        }
        label[p as usize / 2] = rank;
    }
    if rank as usize + 1 < marks.len() {
        let reduced: Vec<_> = marks.iter().map(|&p| label[p as usize / 2]).collect();
        let ranked = suffixes(&reduced, rank as usize + 1);
        let order: Vec<_> = ranked.iter().map(|&i| marks[i as usize]).collect();
        induce(&order, &mut sa);
    } else {
        induce(&sorted, &mut sa);
    }
    sa
}
const LANES: usize = 32;
#[cfg(test)]
fn forward(s: &[u8]) -> (Vec<u8>, [u32; LANES]) {
    let mut x = Vec::with_capacity(s.len() + 1);
    x.extend(s.iter().map(|&v| v as u32 + 1));
    x.push(0);
    let sa = suffixes(&x, 257);
    let mut starts = [0u32; LANES];
    let stride = s.len().div_ceil(LANES);
    let mut last = Vec::with_capacity(s.len());
    for (row, &v) in sa.iter().enumerate() {
        let at = v as usize;
        if at < s.len() && at.is_multiple_of(stride) {
            starts[at / stride] = row as u32;
        }
        if at > 0 {
            last.push(s[at - 1]);
        }
    }
    (last, starts)
}
fn inverse(last: &[u8], start: &[u32; LANES], out: &mut [u8]) -> Result<()> {
    let n = last.len();
    if n >= 1 << 24 || n != out.len() || n == 0 || start.iter().any(|&r| r as usize > n) {
        return Err(bad());
    }
    let primary = start[0] as usize;
    if primary == 0 {
        return Err(bad());
    }
    let mut heads = [0usize; 256];
    for &b in last {
        heads[b as usize] += 1
    }
    let mut sum = 1;
    for v in &mut heads {
        let count = *v;
        *v = sum;
        sum += count;
    }
    let mut links = vec![0u32; n + 1];
    for (i, &s) in last.iter().enumerate() {
        let ix = &mut heads[s as usize];
        links[*ix] = (((i + usize::from(i >= primary)) as u32) << 8) | s as u32;
        *ix += 1;
    }
    let stride = n.div_ceil(LANES);
    let mut state = *start;
    for j in (0..LANES).step_by(8) {
        let mut row: [u32; 8] = state[j..j + 8].try_into().unwrap();
        let full = n.saturating_sub((j + 7) * stride).min(stride);
        for k in 0..full {
            for (lane, pos) in row.iter_mut().enumerate() {
                // All checkpoints were checked above. Links contain only row
                // numbers in 0..=n, and full bounds all eight output spans.
                let v = unsafe { *links.get_unchecked(*pos as usize) };
                unsafe {
                    *out.get_unchecked_mut((j + lane) * stride + k) = v as u8;
                }
                *pos = v >> 8;
            }
        }
        for (lane, pos) in row.iter_mut().enumerate() {
            for k in full..n.saturating_sub((j + lane) * stride).min(stride) {
                let v = links[*pos as usize];
                out[(j + lane) * stride + k] = v as u8;
                *pos = v >> 8;
            }
        }
        state[j..j + 8].copy_from_slice(&row);
    }
    for j in 0..LANES {
        if j * stride < n {
            let next = if (j + 1) * stride < n {
                start[j + 1]
            } else {
                0
            };
            if state[j] != next {
                return Err(bad());
            }
        }
    }
    Ok(())
}
#[inline]
fn promote(list: &mut [u8; 256], i: usize, b: u8) {
    list.copy_within(..i, 1);
    list[0] = b;
}

pub(crate) fn decode(mut p: &[u8], dst: &mut [u8]) -> Result<()> {
    if dst.is_empty() {
        return Err(bad());
    }
    let mut starts = [0; LANES];
    for x in &mut starts {
        *x = u32in(&mut p)? as u32;
    }
    let shift = take(&mut p, 1)?[0];
    if !(12..=18).contains(&shift) {
        return Err(bad());
    }
    let mut bwt = vec![0u8; dst.len()];
    for t in bwt.chunks_mut(1usize << shift) {
        let rank = ans::get(&mut p, t.len())?;
        let lens = ans::get(&mut p, rank.len())?;
        if lens.len() != rank.len() {
            return Err(bad());
        }
        let n = u32in(&mut p)?;
        let mut extra = take(&mut p, n)?;
        let mut list: [u8; 256] = std::array::from_fn(|i| i as u8);
        let mut at = 0;
        for (&r, &l) in rank.iter().zip(lens.iter()) {
            let n = if l == 255 {
                u32in(&mut extra)?
            } else {
                l as usize
            };
            let v = list[r as usize];
            promote(&mut list, r as usize, v);
            let end = at + n + 1;
            t.get_mut(at..end).ok_or_else(bad)?.fill(v);
            at = end;
        }
        if at != t.len() || !extra.is_empty() {
            return Err(bad());
        }
    }
    if !p.is_empty() {
        return Err(bad());
    }
    inverse(&bwt, &starts, dst)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn suffix_order_and_parallel_inverse() {
        let mut seed = 1u32;
        for n in 1..200 {
            let s: Vec<_> = (0..n)
                .map(|_| {
                    seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                    ((seed >> 24) % 7) as u8
                })
                .collect();
            let mut x: Vec<u32> = s.iter().map(|&b| b as u32 + 1).collect();
            x.push(0);
            let mut expected: Vec<u32> = (0..=n as u32).collect();
            expected.sort_unstable_by(|a, b| x[*a as usize..].cmp(&x[*b as usize..]));
            assert_eq!(suffixes(&x, 8), expected);
            let (l, idx) = forward(&s);
            let mut d = vec![0; n];
            inverse(&l, &idx, &mut d).unwrap();
            assert_eq!(d, s);
        }
    }
}
