// Find additional phrases after the local job vocabularies have been unified.
// Counts span the group, but replacements cannot cross a job's decoded extent.
use super::super::super::{Term, hash};
use std::collections::{HashMap, hash_map::Entry};
#[derive(Clone, Copy, Default)]
struct Slot {
    pair: u64,
    score: u32,
    selected: u32,
}

pub(super) struct Vocabulary<'a, 'b> {
    pub(super) nodes: &'b mut Vec<Term>,
    pub(super) rules: &'b mut Vec<(u32, u32)>,
    pub(super) spellings: &'b mut HashMap<&'a [u8], u32>,
}
pub(super) fn learn<'a>(
    raw: &'a [u8],
    shard: usize,
    roots: &mut [Vec<u32>],
    book: Vocabulary<'a, '_>,
    budget: usize,
    least: u32,
) -> usize {
    let n: usize = roots.iter().map(Vec::len).sum();
    if budget == 0 || n < 512 {
        return 0;
    }
    let mask = (n / 2).next_power_of_two().clamp(2048, 1 << 22) - 1;
    let mut slots = vec![Slot::default(); mask + 1];
    let mut added = 0;
    for round in 0..15 {
        slots.fill(Slot::default());
        for stream in roots.iter() {
            for w in stream.windows(2) {
                if u16::from(book.nodes[w[0] as usize].len)
                    + u16::from(book.nodes[w[1] as usize].len)
                    > 64
                {
                    continue;
                }
                let key = (u64::from(w[0]) << 32) | u64::from(w[1]);
                let e = &mut slots[hash(key, mask)];
                if e.pair == key {
                    e.score += 1
                } else if e.score == 0 {
                    *e = Slot {
                        pair: key,
                        score: 1,
                        selected: 0,
                    }
                } else {
                    e.score -= 1
                }
            }
        }
        let mut picks: Vec<_> = (0..slots.len())
            .filter(|&h| slots[h].score >= least)
            .collect();
        let count = picks.len().min(budget - added).min(2048 << round.min(5));
        if count == 0 {
            break;
        }
        if picks.len() > count {
            picks.select_nth_unstable_by_key(count, |&i| (std::cmp::Reverse(slots[i].score), i));
            picks.truncate(count)
        }
        for i in picks {
            slots[i].selected = u32::MAX;
        }
        let mut changed = 0;
        for (part, stream) in roots.iter_mut().enumerate() {
            let mut from = 0;
            let mut out = 0;
            let mut pos = part * shard;
            while from < stream.len() {
                let a = stream[from];
                let mut id = a;
                from += 1;
                if from < stream.len() {
                    let b = stream[from];
                    let key = (u64::from(a) << 32) | u64::from(b);
                    let e = &mut slots[hash(key, mask)];
                    if e.selected != 0 && e.pair == key {
                        let width = book.nodes[a as usize].len + book.nodes[b as usize].len;
                        if e.selected == u32::MAX {
                            let word = &raw[pos..pos + usize::from(width)];
                            let v = match book.spellings.entry(word) {
                                Entry::Occupied(k) => *k.get(),
                                Entry::Vacant(k) => {
                                    let idx = book.nodes.len() as u32;
                                    book.nodes.push(Term {
                                        pos: pos as u32,
                                        len: width,
                                    });
                                    book.rules.push((a, b));
                                    k.insert(idx);
                                    idx
                                }
                            };
                            e.selected = v + 1;
                        }
                        id = e.selected - 1;
                        from += 1;
                        changed += 1;
                    }
                }
                pos += usize::from(book.nodes[id as usize].len);
                stream[out] = id;
                out += 1;
            }
            debug_assert_eq!(pos, ((part + 1) * shard).min(raw.len()));
            stream.truncate(out);
        }
        added += count;
        if added >= budget || changed == 0 {
            break;
        }
    }
    added
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn learned_cross_worker_pairs_remain_independently_decodable() {
        let shard = 16384;
        let s: Vec<_> = (0..2 * shard).map(|i| b"ababaabacabc"[i % 12]).collect();
        let mut terms = vec![
            Term { pos: 0, len: 1 },
            Term { pos: 1, len: 1 },
            Term { pos: 8, len: 1 },
        ];
        let mut map: HashMap<&[u8], u32> = terms
            .iter()
            .enumerate()
            .map(|(i, t)| (&s[t.pos as usize..t.pos as usize + 1], i as u32))
            .collect();
        let mut roots: Vec<Vec<u32>> = s
            .chunks(shard)
            .map(|p| p.iter().map(|b| u32::from(*b - b'a')).collect())
            .collect();
        let mut rules = vec![];
        assert!(
            learn(
                &s,
                shard,
                &mut roots,
                Vocabulary {
                    nodes: &mut terms,
                    rules: &mut rules,
                    spellings: &mut map
                },
                8192,
                3
            ) > 0
        );
        assert_eq!(terms.len(), 3 + rules.len());
        for (i, r) in roots.iter().enumerate() {
            let mut got = vec![];
            for &id in r {
                let t = terms[id as usize];
                got.extend_from_slice(&s[t.pos as usize..t.pos as usize + usize::from(t.len)])
            }
            assert_eq!(got, &s[i * shard..(i + 1) * shard]);
            assert!(r.len() < shard / 2);
        }
    }
}
