// Share exact phrase spellings across neighboring independent workers.
// The wire includes the union dictionary, then a separate token stream per job.
use super::super::{Term, merge_rules};
use super::{EncodeBook, build_book, put_root, read_book, read_root};
use crate::{Frame, MAX_WIDE_BLOCK, Result, bad, parallel, take, u32in};
use std::{borrow::Cow, collections::HashMap};

type Training = (Vec<Term>, Vec<u32>, Vec<(u32, u32)>);
#[derive(Clone, Copy)]
pub(crate) struct Plan {
    local: usize,
    width: usize,
    share: usize,
    pooled: usize,
    minimum: u32,
    conditional: bool,
    storage: usize,
}
impl Plan {
    pub(crate) fn preset(level: u8) -> Option<Self> {
        let (local, share, pooled, minimum, conditional, storage) = match level {
            16 => (65536, 2, 0, 3, false, 0),
            17 => (65536, 4, 0, 3, false, 0),
            18 => (65536, 4, 65536, 3, false, 0),
            19 => (32768, 4, 131072, 2, true, 0),
            20 => (32768, 4, 131072, 2, false, 1),
            21 => (32768, 4, 131072, 2, true, 1),
            22 => (32768, 4, 131072, 2, true, 5),
            _ => return None,
        };
        Some(Self {
            local,
            width: 64,
            share,
            pooled,
            minimum,
            conditional,
            storage,
        })
    }
    #[cfg(test)]
    fn for_test(local: usize, width: usize, share: usize) -> Self {
        Self {
            local,
            width,
            share,
            pooled: 0,
            minimum: 3,
            conditional: false,
            storage: 0,
        }
    }
}
struct GroupWire {
    wire: Vec<u8>,
    book: EncodeBook,
    roots: Vec<Vec<u32>>,
}
fn union(raw: &[u8], shard: usize, jobs: Vec<Training>, plan: Plan) -> Result<GroupWire> {
    let mut intern: HashMap<&[u8], u32> = HashMap::new();
    let mut terms = Vec::new();
    let mut rules = Vec::new();
    let mut ids: Vec<_> = jobs
        .iter()
        .map(|(ts, _, _)| vec![u32::MAX; ts.len()])
        .collect();
    for composites in [false, true] {
        for (i, (ts, _, rs)) in jobs.iter().enumerate() {
            let leaf = ts.len() - rs.len();
            let range = if composites { leaf..ts.len() } else { 0..leaf };
            for j in range {
                let t = ts[j];
                if t.pos == u32::MAX {
                    continue;
                }
                let pos = i * shard + t.pos as usize;
                let v = &raw[pos..pos + usize::from(t.len)];
                let id = match intern.entry(v) {
                    std::collections::hash_map::Entry::Occupied(e) => *e.get(),
                    std::collections::hash_map::Entry::Vacant(e) => {
                        if composites {
                            let (a, b) = rs[j - leaf];
                            let children = (ids[i][a as usize], ids[i][b as usize]);
                            if children.0 == u32::MAX || children.1 == u32::MAX {
                                return Err(bad());
                            }
                            rules.push(children);
                        }
                        let id = terms.len() as u32;
                        terms.push(Term {
                            pos: pos as u32,
                            len: t.len,
                        });
                        e.insert(id);
                        id
                    }
                };
                ids[i][j] = id;
            }
        }
    }
    let mut uses = vec![0u32; terms.len()];
    let mut roots = Vec::new();
    for ((_, mut stream, _), lookup) in jobs.into_iter().zip(ids) {
        for id in &mut stream {
            *id = lookup[*id as usize];
            uses[*id as usize] += 1;
        }
        roots.push(stream);
    }
    if plan.pooled != 0 {
        wordpool::learn(
            raw,
            shard,
            &mut roots,
            wordpool::Vocabulary {
                nodes: &mut terms,
                rules: &mut rules,
                spellings: &mut intern,
            },
            plan.pooled,
            plan.minimum,
        );
        uses = vec![0; terms.len()];
        for part in &roots {
            for &r in part {
                uses[r as usize] += 1;
            }
        }
    }
    drop(intern);

    let nt = roots.iter().map(Vec::len).sum();
    let mut wire = vec![];
    let book = build_book(raw, &terms, &rules, uses, nt, &mut wire).ok_or_else(bad)?;
    if plan.storage != 0 {
        wire = super::bookpack::repack(&wire, raw.len(), plan.storage)?;
    }
    Ok(GroupWire { wire, book, roots })
}
pub(crate) fn compress(raw: &[u8], threads: usize, plan: Plan) -> Result<Frame<'static>> {
    let Plan {
        local: cap,
        width,
        share,
        ..
    } = plan;
    if raw.is_empty() || !(1..=64).contains(&share) {
        return Err(bad());
    }
    let workers = threads.clamp(1, 256);
    let jobs = raw.len().div_ceil(MAX_WIDE_BLOCK).next_multiple_of(workers);
    let shard = raw
        .len()
        .div_ceil(jobs)
        .next_multiple_of(64)
        .min(MAX_WIDE_BLOCK);
    let count = raw.len().div_ceil(shard);
    let mut trained: Vec<Training> = (0..count).map(|_| Default::default()).collect();
    parallel(
        workers.min(count),
        raw.chunks(shard).zip(&mut trained),
        |(bytes, slot)| {
            *slot = merge_rules::<true>(bytes, cap, width, 3);
            Ok(())
        },
    )?;
    let mut groups: Vec<_> = trained
        .chunks_mut(share)
        .map(|v| Some(v.iter_mut().map(std::mem::take).collect::<Vec<_>>()))
        .collect();
    let mut books: Vec<_> = groups.iter().map(|_| None).collect();
    parallel(
        workers.min(books.len()),
        groups
            .iter_mut()
            .zip(&mut books)
            .zip(raw.chunks(shard * share)),
        |((g, slot), bytes)| {
            *slot = Some(union(bytes, shard, g.take().unwrap(), plan)?);
            Ok(())
        },
    )?;
    let books: Vec<_> = books.into_iter().map(Option::unwrap).collect();
    let mut blocks: Vec<Cow<'static, [u8]>> = vec![Default::default(); count];
    parallel(
        workers.min(count),
        blocks.iter_mut().enumerate(),
        |(ix, slot)| {
            let GroupWire {
                wire: dict,
                book,
                roots,
            } = &books[ix / share];
            let leader = 50;
            let v = &roots[ix % share];
            let mut wire = vec![];
            if ix % share == 0 {
                wire.extend([leader, roots.len() as u8]);
                wire.extend((dict.len() as u32).to_le_bytes());
                wire.extend(dict);
            } else {
                wire.push(leader + 1)
            }
            wire.extend((v.len() as u32).to_le_bytes());
            put_root(v, book, &mut wire, plan.conditional);
            if wire.len() > shard + 1 {
                return Err(bad());
            }
            *slot = Cow::Owned(wire);
            Ok(())
        },
    )?;
    Ok(Frame {
        size: raw.len(),
        block: shard,
        blocks,
        layout: None,
    })
}
pub(crate) fn decompress(f: &Frame<'_>, threads: usize, dst: &mut [u8]) -> Result<()> {
    if f.block == 0
        || f.block > MAX_WIDE_BLOCK
        || dst.len() != f.size
        || f.layout.is_some()
        || dst.len().div_ceil(f.block) != f.blocks.len()
    {
        return Err(bad());
    }
    let mut groups = Vec::new();
    let mut jobs = Vec::new();
    let mut i = 0;
    while i < f.blocks.len() {
        let mut p = &f.blocks[i][..];
        let leader = take(&mut p, 1)?[0];
        if leader != 50 {
            return Err(bad());
        }
        let count = take(&mut p, 1)?[0] as usize;
        if !(1..=64).contains(&count) || count > f.blocks.len() - i {
            return Err(bad());
        }
        let ds = u32in(&mut p)?;
        let dict = take(&mut p, ds)?;
        let limit = (f.size - i * f.block).min(f.block * count);
        let mut total = 0;
        for k in 0..count {
            if k != 0 {
                p = &f.blocks[i + k];
                if take(&mut p, 1)? != [leader + 1] {
                    return Err(bad());
                }
            }
            let nt = u32in(&mut p)?;
            if nt == 0 || nt > f.block {
                return Err(bad());
            }
            total += nt;
            jobs.push((groups.len(), nt, p));
        }
        groups.push((dict, limit, total));
        i += count;
    }
    let mut books: Vec<_> = groups.iter().map(|_| None).collect();
    parallel(
        threads.min(books.len()),
        groups.into_iter().zip(&mut books),
        |((mut p, n, want), b)| {
            let (nt, book) = read_book(&mut p, n)?;
            if nt != want || !p.is_empty() {
                return Err(bad());
            }
            *b = Some(book);
            Ok(())
        },
    )?;

    parallel(
        threads.min(jobs.len()),
        jobs.into_iter().zip(dst.chunks_mut(f.block)),
        |((group, nt, p), d)| read_root(p, nt, books[group].as_ref().unwrap(), d),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shared_dictionary_is_serialized_and_workers_can_change() {
        let mut s = vec![];
        for i in 0..6400 {
            s.extend(
                format!(
                    "The winter river is high in {} when a cold wind crosses the river at {}. ",
                    i % 19,
                    i % 31
                )
                .as_bytes(),
            )
        }
        for group in [1, 2, 4] {
            let f = compress(&s, 4, Plan::for_test(2048, 48, group)).unwrap();
            let mut bytes = vec![];
            f.write(&mut bytes).unwrap();
            let mut g = Frame::read(&bytes).unwrap();
            let mut out = vec![0x83; s.len()];
            for t in [1, 3] {
                decompress(&g, t, &mut out).unwrap();
                assert_eq!(&out, &s)
            }
            if group > 1 {
                g.blocks[1].to_mut()[0] = 50;
                assert!(decompress(&g, 2, &mut out).is_err())
            }
            let n = g.blocks[0].len();
            g.blocks[0].to_mut()[2..6].copy_from_slice(&(n as u32).to_le_bytes());
            assert!(decompress(&g, 2, &mut out).is_err());
        }
    }
}

mod wordpool;
