// For bit width w, every value with more than w bits needs a position byte
// and an initial 7-bit digit. Each further group of 7 high bits adds a byte.
use super::{N, WIDTHS};
#[inline]
pub(super) fn width(values: &[u32; N], n: usize) -> (usize, usize, usize) {
    let mut suffix = [0u16; 64];
    for &v in &values[..n] {
        suffix[(32 - v.leading_zeros()) as usize] += 1;
    }
    let mut above = 0;
    for x in suffix[..33].iter_mut().rev() {
        let c = *x;
        *x = above;
        above += c
    }
    let mut best = (usize::MAX, 0, 0);
    for &w in WIDTHS {
        if 16 * w >= best.0 {
            break;
        }
        let ex = suffix[w];
        let bytes = 16 * w
            + usize::from(
                2 * ex + suffix[w + 7] + suffix[w + 14] + suffix[w + 21] + suffix[w + 28],
            );
        if bytes < best.0 {
            best = (bytes, w, usize::from(ex))
        }
    }
    best
}

// On equal costs, prefer a base, as in the generic writer.
pub(super) fn constant(x: u32, prev: u32, out: &mut Vec<u8>) {
    if x == 0 {
        out.extend([0, 0]);
    } else if x == prev {
        out.extend([64, 0]);
    } else {
        let d = x.wrapping_sub(prev);
        let z = (d << 1) ^ ((d as i32 >> 31) as u32);
        if z < 128 {
            out.extend([64, 1, 0, z as u8]);
        } else if z < 16384 {
            out.extend([64, 1, 0, (z as u8) | 128, (z >> 7) as u8]);
        } else {
            out.extend([128, 0]);
            out.extend(x.to_le_bytes());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn same_width_and_full_record_for_all_inputs() {
        let mut r = 219938u32;
        let mut v = [0; 128];
        let mut data = Vec::new();
        for j in 0..211 {
            for (i, value) in v.iter_mut().enumerate() {
                r = r.wrapping_mul(1664525).wrapping_add(1013904223);
                *value = match j % 8 {
                    0 => 0,
                    1 => i as u32 % 3,
                    2 => r >> (j % 32),
                    3 => 1 << (r % 32),
                    4 => {
                        if i % 13 == 0 {
                            r
                        } else {
                            0
                        }
                    }
                    5 => j as u32,
                    6 => u32::MAX - (r % 97),
                    _ => r,
                };
                data.extend(value.to_le_bytes());
            }
            for n in [1, 2, 5, 32, 99, 127, 128] {
                assert_eq!(width(&v, n), super::super::width(&v, n))
            }
        }
        for n in [1, 4, 799, 1559, 2123, 16533, data.len()] {
            for w in [1, 2, 3, 4, 6, 8] {
                let a = super::super::encode_impl::<0>(&data[..n], w, 3);
                for m in 1..=3 {
                    assert_eq!(super::super::fast(&data[..n], w, 3, m), a)
                }
            }
        }

        for p in [0, 1, 88, 166, 8192, 8193, 84531, u32::MAX] {
            for x in [0, 1, 64, 88, 8191, 8192, 84532, u32::MAX] {
                let a = [x; 128];
                let mut s = Vec::new();
                super::super::field::<0>(&a, 39, p, &mut s);
                let mut f = Vec::new();
                constant(x, p, &mut f);
                assert_eq!(s, f);
            }
        }
    }
}
