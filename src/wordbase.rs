// The shared tokenizer and bounded entropy streams used by word encoders.
use crate::{Result, ans, bad};

#[derive(Clone, Copy)]
pub(crate) struct Term {
    pub(crate) pos: u32,
    pub(crate) len: u8,
    pub(crate) count: u32,
}

pub(crate) fn phrase_tokens<const RULES: bool>(s: &[u8]) -> (Vec<Term>, Vec<u32>) {
    let (terms, ids) = tokenize(s, 0);

    (terms, ids)
}
pub(crate) fn tokenize(s: &[u8], kind: usize) -> (Vec<Term>, Vec<u32>) {
    fn class(b: u8) -> u8 {
        if b.is_ascii_alphabetic() {
            1
        } else if b.is_ascii_digit() {
            2
        } else {
            0
        }
    }
    let mut table = vec![u32::MAX; (s.len() / 16).max(64).next_power_of_two()];
    let mut terms = Vec::<Term>::new();
    let mut ids = Vec::with_capacity(s.len() / 4);
    let mut hashes = Vec::new();
    let mut i = 0;
    while i < s.len() {
        let start = i;
        let c = class(s[i]);
        i += 1;
        while i < s.len() && i - start < 16 && class(s[i]) == c {
            i += 1;
        }
        if c != 0 && kind != 1 {
            if kind == 2 {
                while i < s.len() && i - start < 16 && class(s[i]) == 0 {
                    i += 1;
                }
            } else if i - start < 16 && s.get(i) == Some(&b' ') {
                i += 1;
            }
        }
        let raw = &s[start..i];
        let mut b = [0u8; 16];
        b[..raw.len()].copy_from_slice(raw);
        let a = u64::from_le_bytes(b[..8].try_into().unwrap());
        let z = u64::from_le_bytes(b[8..].try_into().unwrap());
        let h = a.wrapping_mul(0x9e3779b97f4a7c15)
            ^ z.wrapping_mul(0xa24baed4963ee407)
            ^ (raw.len() as u64).wrapping_mul(0x94d049bb133111eb);
        let h = (h ^ (h >> 31)) as usize;
        let mut k = h & (table.len() - 1);
        let id = loop {
            let id = table[k];
            if id == u32::MAX {
                table[k] = terms.len() as u32;
                hashes.push(h);
                terms.push(Term {
                    pos: start as u32,
                    len: raw.len() as u8,
                    count: 0,
                });
                break table[k];
            }
            let v = terms[id as usize];
            if v.len as usize == raw.len() && s[v.pos as usize..v.pos as usize + raw.len()] == *raw
            {
                break id;
            }
            k = (k + 1) & (table.len() - 1);
        };
        terms[id as usize].count += 1;
        ids.push(id);
        if terms.len() * 3 > table.len() * 2 {
            table = vec![u32::MAX; table.len() * 2];
            for (j, &ha) in hashes.iter().enumerate() {
                let mut x = ha & (table.len() - 1);
                while table[x] != u32::MAX {
                    x = (x + 1) & (table.len() - 1);
                }
                table[x] = j as u32;
            }
        }
    }
    (terms, ids)
}
pub(crate) fn plain<'a>(p: &mut &'a [u8], limit: usize) -> Result<std::borrow::Cow<'a, [u8]>> {
    if !matches!(p.get(4), Some(0 | 1 | 2 | 4 | 5)) {
        return Err(bad());
    }
    ans::get(p, limit)
}
