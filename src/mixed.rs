// Independent, differently sized text jobs. The grammar jobs produce denser
// archives but take longer to decode per input byte than the byte-oriented jobs.
use crate::{Frame, Result, bad, layout::Layout, parallel};
use std::borrow::Cow;
#[derive(Clone, Copy)]
struct Plan {
    grammar_jobs: usize,
    share: usize,
    plain: u8,
    plain_kib: usize,
    rotate: usize,
    flat: bool,
    split_jobs: usize,
    weight: u8,
}
pub(crate) fn preset(data: &[u8], threads: usize, lv: u8) -> Result<Frame<'static>> {
    // Reserve some tasks for denser token-domain coding. The byte-oriented
    // workers restore the remaining spans and can share split jobs dynamically.
    let t = threads.clamp(2, 256);
    let (g, share, plain, parts) = match lv {
        16 => (1, 220, 16, 1),
        _ => return Err(bad()),
    };
    let grammar_jobs = ((g * t + 2) / 4).clamp(1, t - 1);
    let split_jobs = (parts * (t - grammar_jobs))
        .div_ceil(4 - g)
        .clamp(1, t - grammar_jobs);
    run(
        data,
        t,
        Plan {
            grammar_jobs,
            share,
            plain,
            plain_kib: 32768,
            rotate: 0,
            flat: true,
            split_jobs,
            weight: 1,
        },
        crate::grammar::native,
    )
}

fn run(
    data: &[u8],
    threads: usize,
    plan: Plan,
    word: fn(&[u8]) -> Vec<u8>,
) -> Result<Frame<'static>> {
    let Plan {
        grammar_jobs,
        share,
        plain,
        plain_kib,
        rotate,
        flat,
        split_jobs,
        weight,
    } = plan;
    let t = threads.clamp(1, 256);
    if grammar_jobs == 0
        || grammar_jobs >= t
        || !(1..1000).contains(&share)
        || (!(5..=12).contains(&plain) && plain != 16)
        || plain_kib == 0
        || plain_kib > 32768
    {
        return Err(bad());
    }
    if data.len() < t {
        return Err(bad());
    }
    let g = grammar_jobs;
    let b = t - g;
    let plain_size = (data.len() as u128 * (1000 - share) as u128).div_ceil((1000 * b) as u128);
    let word_size = (data.len() as u128 * share as u128).div_ceil((1000 * g) as u128);
    let waves = word_size
        .max(plain_size)
        .div_ceil(crate::MAX_WIDE_BLOCK as u128)
        .max(1) as usize;
    let mut tasks = Vec::new();
    let mut sizes = Vec::new();
    let mut last = 0usize;
    for wave in 0..waves {
        let next = ((wave + 1) as u128 * data.len() as u128 / waves as u128) as usize;
        let n = next - last;
        let ng = ((n as u128 * share as u128) / 1000) as usize;
        let nb = n - ng;
        let (mut a, mut c) = (0usize, 0usize);
        for j in 0..t {
            let grammar = (j + rotate % t + wave % t) % t < g;
            let size = if grammar {
                let old = ng * a / g;
                a += 1;
                ng * a / g - old
            } else {
                let old = nb * c / b;
                c += 1;
                nb * c / b - old
            };
            if size == 0 || size > crate::MAX_WIDE_BLOCK {
                return Err(bad());
            }
            let parts = if flat && !grammar && (split_jobs == 0 || c <= split_jobs) {
                size.div_ceil(plain_kib * 1024)
            } else {
                1
            };
            for part in data[last..last + size].chunks(size.div_ceil(parts)) {
                sizes.push(part.len());
                tasks.push((grammar, part));
            }
            last += size;
        }
    }
    if weight != 0 {
        let (mut j, mut raw) = (0usize, 0usize);
        while j < tasks.len() {
            let grammar = tasks[j].0;
            let mut k = j + 1;
            while k < tasks.len() && tasks[k].0 == grammar {
                k += 1;
            }
            let end = raw + sizes[j..k].iter().sum::<usize>();
            let fixed = crate::weight::partitions(&data[raw..end], &sizes[j..k], grammar, weight);
            for (slot, n) in (j..k).zip(fixed) {
                sizes[slot] = n;
                tasks[slot].1 = &data[raw..raw + n];
                raw += n;
            }
            j = k;
        }
    }
    let block = *sizes.iter().max().ok_or_else(bad)?;
    let mut blocks = vec![Cow::default(); tasks.len()];
    parallel(
        t,
        tasks.into_iter().zip(&mut blocks),
        |((grammar, s), dst)| {
            let mut wire = if grammar {
                word(s)
            } else {
                let part = |src: &[u8]| match plain {
                    16 => crate::grammar::worddag::preset(src, 15),
                    5 => crate::lz::legacy_width::<false, 17, 18, false, 4, 0, 25>(src),
                    6 => crate::lz::history_model::<
                        false,
                        17,
                        20,
                        false,
                        3,
                        16,
                        25,
                        true,
                        4,
                        0,
                        true,
                        true,
                        0,
                        false,
                    >(src),
                    7 => crate::lz::legacy_policy::<false, 18, 21, false, 1, 16, 25, true, 4>(src),
                    8 => crate::lz::history_model::<
                        false,
                        18,
                        21,
                        false,
                        1,
                        19,
                        25,
                        true,
                        2,
                        0,
                        true,
                        true,
                        1,
                        false,
                    >(src),
                    _ => crate::lz::middle_text(src, plain),
                };
                let parts = if flat {
                    1
                } else {
                    s.len().div_ceil(plain_kib * 1024)
                };
                if parts == 1 {
                    part(s)
                } else {
                    let mut wire = vec![5];
                    let chunk = s.len().div_ceil(parts);
                    for row in s.chunks(chunk) {
                        let v = part(row);
                        wire.extend((row.len() as u32).to_le_bytes());
                        wire.extend((v.len() as u32).to_le_bytes());
                        wire.extend(v);
                    }
                    wire
                }
            };
            if wire.len() > s.len() {
                wire = vec![0];
                wire.extend(s);
            }
            *dst = Cow::Owned(wire);
            Ok(())
        },
    )?;
    Ok(Frame {
        size: data.len(),
        block,
        blocks,
        layout: Some(Layout::linear(data.len(), sizes)),
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_mixtures_restore_from_the_serialized_layout() {
        let mut src = Vec::new();
        for j in 0..2300 {
            src.extend(
                format!(
                    "The boat carried {} passengers across the river to town {}.\n",
                    j % 103,
                    j % 29
                )
                .bytes(),
            );
        }
        for (level, threads) in [(16, 2), (16, 4), (16, 3)] {
            let frame = preset(&src, threads, level).unwrap();
            assert!(frame.blocks.iter().any(|s| s.starts_with(&[43, 1])));
            let mut wire = vec![];
            frame.write(&mut wire).unwrap();
            let parsed = Frame::read(&wire).unwrap();
            let mut out = vec![0xdc; src.len()];
            crate::decompress(&parsed, 4, &mut out).unwrap();
            assert_eq!(src, out);
        }
    }
}
