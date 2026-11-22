// Every accepted candidate already has one full word at either end. Count
// short copies on both sides with straight-line word comparisons before the
// slow path. Confirmed endpoints and all emitted bytes remain unchanged.
use super::{common, word};
#[inline(always)]
pub(super) fn forward(s: &[u8], q: usize, i: usize, d: u64) -> usize {
    if s.len() - i < 16 {
        return if d == 0 {
            8 + common(s, q + 8, i + 8)
        } else {
            d.trailing_zeros() as usize / 8
        };
    }
    let n = d.trailing_zeros() as usize / 8;
    let e = word(s, q + 8) ^ word(s, i + 8);
    let m = e.trailing_zeros() as usize / 8;
    let short = if n == 8 { 8 + m } else { n };
    if short == 16 {
        16 + common(s, q + 16, i + 16)
    } else {
        short
    }
}
#[inline(always)]
pub(super) fn backward(s: &[u8], anchor: usize, q: usize, i: usize) -> usize {
    let max = (i - anchor).min(q);
    let mut n = 0;
    while n < max && s[i - n - 1] == s[q - n - 1] {
        n += 1
    }
    n
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn speculative_reads_handle_nearby_sources_partial_words_and_arbitrary_anchors() {
        let mut s: Vec<u8> = (0..300).map(|i| b"abcabcxyzxy"[i % 11]).collect();
        for phase in [0, 1, 8, 17] {
            if phase != 0 {
                s[200 + phase] ^= 37
            }
            for i in 8..=s.len() - 8 {
                for q in [0, 1, i / 2, i - 1] {
                    let d = word(&s, i) ^ word(&s, q);
                    let n = s[q..]
                        .iter()
                        .zip(&s[i..])
                        .take_while(|(a, b)| a == b)
                        .count();
                    assert_eq!(forward(&s, q, i, d), n);
                    for anchor in [0, i / 3, i - 1, i] {
                        let bound = (i - anchor).min(q);
                        let m = (0..bound)
                            .take_while(|&j| s[i - j - 1] == s[q - j - 1])
                            .count();
                        assert_eq!(backward(&s, anchor, q, i), m);
                    }
                }
            }
        }
    }
}
