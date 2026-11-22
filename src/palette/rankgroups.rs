// Choose power-of-two rank groups by splitting frequent ranks first.
// Score each split by its exact bit cost.
fn tail(hist: &[u8; 9], prefix: &[u32; 512], n: usize) -> usize {
    let mut at = 0;
    let mut cost = 0;
    for (width, &groups) in hist.iter().enumerate() {
        let next = (at + (usize::from(groups) << width)).min(n);
        cost += (prefix[next] - prefix[at]) as usize * width;
        at = next;
    }
    debug_assert_eq!(at, n);
    cost
}
pub(super) fn plan(prefix: &[u32; 512], n: usize, len: usize, policy: usize) -> (usize, Vec<u8>) {
    let mut best = usize::MAX;
    let mut best_hist = [0; 9];
    for start in 0..if policy == 1 { 1 } else { 2 } {
        let mut hist = [0u8; 9];
        if start == 0 {
            hist[n.next_power_of_two().ilog2() as usize] = 1;
        } else {
            for (w, v) in hist.iter_mut().enumerate() {
                *v = ((n >> w) & 1) as u8
            }
        }
        let mut cost = tail(&hist, prefix, n);
        let mut count = hist.iter().map(|&v| usize::from(v)).sum::<usize>();
        loop {
            if count >= 2 && count.is_power_of_two() {
                let k = count.ilog2() as usize;
                let bits = cost + k * len + 8 * (count + n + 40);
                if bits < best {
                    best = bits;
                    best_hist = hist;
                }
                if best <= (k + 1) * len + 8 * (2 * count + n + 40) {
                    break;
                }
            }
            if count >= 32 || count >= n.next_power_of_two() {
                break;
            }
            let mut split = 0;
            let mut winner = usize::MAX;
            for w in 1..=8 {
                if hist[w] == 0 {
                    continue;
                }
                hist[w] -= 1;
                hist[w - 1] += 2;
                let bits = tail(&hist, prefix, n);
                hist[w - 1] -= 2;
                hist[w] += 1;
                if bits < winner {
                    split = w;
                    winner = bits;
                }
            }
            if split == 0 {
                break;
            }
            hist[split] -= 1;
            hist[split - 1] += 2;
            cost = winner;
            count += 1;
        }
    }
    let widths = best_hist
        .into_iter()
        .enumerate()
        .flat_map(|(w, g)| std::iter::repeat_n(w as u8, g as usize))
        .collect();
    (best, widths)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn layouts_cover_an_uneven_alphabet_and_restore_its_last_short_group() {
        for n in [2usize, 3, 11, 17, 63, 129, 201, 255, 256] {
            let syms: Vec<_> = (0..n).map(|i| (i * 251) as u8).collect();
            let mut prefix = [0u32; 512];
            let mut raw = Vec::new();
            for (i, &v) in syms.iter().enumerate() {
                let count = 1 + (n - i) * (n - i) / 16;
                raw.extend(std::iter::repeat_n(v, count));
                prefix[i + 1] = raw.len() as u32;
            }
            for policy in [1, 2] {
                let (bits, widths) = plan(&prefix, n, raw.len(), policy);
                assert!(bits != usize::MAX && widths.len().is_power_of_two());
                let mut enc = Vec::new();
                super::super::write_groups(&raw, &mut enc, &syms, &prefix, widths);
                let mut p = &enc[1..];
                assert_eq!(super::super::get(&mut p, raw.len()).unwrap(), raw);
                assert!(p.is_empty());
            }
        }
    }
}
