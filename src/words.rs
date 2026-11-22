// Each block stores its own dictionary of phrases drawn from the input.
use crate::{Result, ans, bad, lz, take, u32in};
const DIRECT: usize = 64;
const LIMIT: usize = DIRECT + 64 * 256;
#[inline]
fn letter(c: u8) -> bool {
    (c | 32).wrapping_sub(b'a') < 26
}
pub fn detect(s: &[u8]) -> bool {
    if s.len() < 16384 {
        return false;
    }
    let mut alpha = 0;
    let mut printable = 0;
    for j in 0..512 {
        let c = s[(s.len() - 1) * j / 511];
        alpha += letter(c) as usize;
        printable += (c == 10 || c == 13 || (32..127).contains(&c)) as usize;
    }
    alpha > 240 && printable > 470
}

pub fn decode(mut p: &[u8], dst: &mut [u8]) -> Result<()> {
    let count = u16::from_le_bytes(take(&mut p, 2)?.try_into().unwrap()) as usize;
    if !(DIRECT..=LIMIT).contains(&count) {
        return Err(bad());
    }
    let n = u32in(&mut p)?;
    if n > dst.len() {
        return Err(bad());
    }
    let def = ans::get(&mut p, count * 34)?;
    let mut defs = &*def;
    let mut words = vec![[0u8; 32]; count];
    let mut sizes = vec![0u8; count];
    let mut prev = [0u8; 32];
    let mut len = 0;
    for (w, sz) in words.iter_mut().zip(&mut sizes) {
        let h = take(&mut defs, 2)?;
        let keep = h[0] as usize;
        let add = h[1] as usize;
        if keep > len || keep + add > 32 || keep + add == 0 {
            return Err(bad());
        }
        len = keep + add;
        prev[keep..len].copy_from_slice(take(&mut defs, add)?);
        *w = prev;
        *sz = len as u8;
    }
    if !defs.is_empty() {
        return Err(bad());
    }
    let mut storage = vec![];
    let kind = take(&mut p, 1)?[0];
    let t = match kind {
        0 if p.len() == n => p,
        1 | 13 | 16 | 17 => {
            storage.resize(n, 0);
            match kind {
                13 => lz::decode_impl::<true>(p, &mut storage),
                16 => lz::decode_min::<false, 3>(p, &mut storage),
                17 => lz::decode_min::<true, 3>(p, &mut storage),
                _ => lz::decode(p, &mut storage),
            }?;
            &storage
        }
        11 => {
            storage.resize(n, 0);
            crate::sorted::decode(p, &mut storage)?;
            &storage
        }
        _ => return Err(bad()),
    };
    let mut i = 0;
    let mut at = 0;
    while i < t.len() {
        let b = t[i];
        i += 1;
        if b < 128 {
            let byte = if b == 127 {
                let v = *t.get(i).ok_or_else(bad)?;
                i += 1;
                v
            } else {
                b
            };
            *dst.get_mut(at).ok_or_else(bad)? = byte;
            at += 1;
        } else {
            let id = if b < 192 {
                (b - 128) as usize
            } else {
                let v = *t.get(i).ok_or_else(bad)?;
                i += 1;
                DIRECT + ((b - 192) as usize) * 256 + v as usize
            };
            let w = words.get(id).ok_or_else(bad)?;
            let l = sizes[id] as usize;
            if at + 32 <= dst.len() {
                unsafe {
                    std::ptr::copy_nonoverlapping(w.as_ptr(), dst.as_mut_ptr().add(at), 32);
                }
            } else {
                dst.get_mut(at..at + l)
                    .ok_or_else(bad)?
                    .copy_from_slice(&w[..l]);
            }
            at += l;
        }
    }
    if at != dst.len() {
        return Err(bad());
    }
    Ok(())
}
