fn sample(s: &[u8], lo: usize, hi: usize, word: bool, mode: u8, table: &mut [usize]) -> u64 {
    let n = hi - lo;
    let mid = lo + n / 2;
    let start = mid.saturating_sub(4096).max(lo);
    let warm = start.saturating_sub(16384);
    let end = (start + 8192).min(hi);
    if end - start < 24 {
        return n as u64 * 64;
    }
    #[inline(always)]
    fn key(s: &[u8], p: usize) -> usize {
        (((crate::lz::word(s, p) << 24).wrapping_mul(0x9e3779b97f4a7c15)) >> 50) as usize
    }
    for i in (warm..start).step_by(2) {
        if i + 8 <= end {
            table[key(s, i)] = i;
        }
    }
    let (mut i, mut copies, mut literal, mut far) = (start, 0usize, 0usize, 0usize);
    while i + 8 <= end {
        let k = key(s, i);
        let old = std::mem::replace(&mut table[k], i);
        if old >= warm && old < i && s[old..old + 4] == s[i..i + 4] {
            let len = 4 + crate::lz::common(&s[..end], old + 4, i + 4);
            copies += 1;
            far += usize::from(i - old >= 4096);
            i += len;
        } else {
            literal += 1;
            i += 1
        }
    }
    literal += end - i;
    let mut changes = 0;
    if word || mode == 3 {
        let class = |v: u8| {
            if v.is_ascii_alphabetic() {
                1
            } else if v.is_ascii_digit() {
                2
            } else {
                0
            }
        };
        let mut prev = 3;
        for &v in &s[start..end] {
            let k = class(v);
            changes += usize::from(prev != k);
            prev = k;
        }
    }
    let m = end - start;
    let score = match (word, mode) {
        (_, 3) => 2 * m + 18 * changes + 8 * literal,
        (true, 2) => m + 35 * changes + 10 * literal,
        (true, _) => 3 * m + 24 * changes + 7 * literal,
        (false, 2) => 2 * m + 88 * copies + 23 * literal + 16 * far,
        (false, _) => 8 * m + 68 * copies + 14 * literal + 7 * far,
    };
    (n as u64) * (score as u64) * 16 / (m as u64)
}
pub(crate) fn partitions(s: &[u8], original: &[usize], word: bool, mode: u8) -> Vec<usize> {
    const UNIT: usize = 512 << 10;
    if mode == 0 || original.len() < 2 || s.len() < UNIT * 2 {
        return original.to_vec();
    }
    let mut table = vec![usize::MAX; 1 << 14];
    let mut sum = 0u64;
    let mut bounds = vec![(0usize, 0u64)];
    for a in (0..s.len()).step_by(UNIT) {
        let b = (a + UNIT).min(s.len());
        sum += sample(s, a, b, word, mode, &mut table);
        bounds.push((b, sum));
    }
    let mut sizes = Vec::with_capacity(original.len());
    let (mut cursor, mut goal, mut last) = (1usize, 0usize, 0usize);
    let max = crate::MAX_WIDE_BLOCK;
    for (j, &want) in original[..original.len() - 1].iter().enumerate() {
        goal += want;
        let target = (u128::from(sum) * goal as u128 / s.len() as u128) as u64;
        while bounds[cursor].1 < target {
            cursor += 1;
        }
        let (lo, a) = bounds[cursor - 1];
        let (hi, b) = bounds[cursor];
        let pos = lo + ((target - a) as u128 * (hi - lo) as u128 / (b - a) as u128) as usize;
        let rem = original.len() - j - 1;
        let low = (last + 1).max(s.len().saturating_sub(rem.saturating_mul(max)));
        let high = (last + max).min(s.len() - rem);
        let next = (pos / 64 * 64).clamp(low, high);
        sizes.push(next - last);
        last = next;
    }
    sizes.push(s.len() - last);
    sizes
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn skewed_data_produces_a_bounded_complete_partition() {
        let mut s = vec![b'x'; 3 << 20];
        let mut v = 11u32;
        for b in &mut s[1 << 20..] {
            v = v.wrapping_mul(1664525).wrapping_add(1013904223);
            *b = (v >> 24) as u8;
        }
        let original = [1 << 20, 1 << 20, 1 << 20];
        for k in 1..=3 {
            let sizes = partitions(&s, &original, false, k);
            assert_eq!(sizes.len(), original.len());
            assert_eq!(sizes.iter().sum::<usize>(), s.len());
            assert!(sizes.iter().all(|&x| x > 0 && x <= crate::MAX_WIDE_BLOCK));
            assert!(sizes[0] > original[0]);
        }
    }
}
