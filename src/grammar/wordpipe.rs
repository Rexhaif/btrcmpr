// Interleave future dictionary hints with independent current loads. The byte
// representation is unchanged; short stores can overlap until the last tile.
use crate::{Result, bad};
#[inline(always)]
fn hint(p: *const u8) {
    #[cfg(target_arch = "aarch64")]
    unsafe {
        core::arch::asm!("prfm pldl1keep, [{p}]",p=in(reg)p,options(readonly,nostack,preserves_flags));
    }
    #[cfg(not(target_arch = "aarch64"))]
    let _ = p;
}
#[cfg(test)]
fn remap(data: &mut [u32], desc: &[u32]) -> Result<()> {
    if data
        .iter()
        .copied()
        .max()
        .is_some_and(|r| r as usize >= desc.len())
    {
        return Err(bad());
    }
    let mut i = 0;
    while i + 8 <= data.len() {
        if i + 72 <= data.len() {
            for &rank in &data[i + 64..i + 72] {
                hint(unsafe { desc.as_ptr().add(rank as usize).cast() });
            }
        }
        let mut v = [0; 8];
        for (d, &r) in v.iter_mut().zip(&data[i..i + 8]) {
            // All ranks passed the single bounds reduction above.
            *d = unsafe { *desc.get_unchecked(r as usize) };
        }
        data[i..i + 8].copy_from_slice(&v);
        i += 8;
    }
    for r in &mut data[i..] {
        *r = desc[*r as usize];
    }
    Ok(())
}
#[cfg(target_arch = "aarch64")]
#[inline(always)]
unsafe fn store(word: u32, q: core::arch::aarch64::uint8x16_t, spell: *const u8, d: *mut u8) {
    use core::arch::aarch64::vst1q_u8;
    unsafe {
        vst1q_u8(d, q);
        let n = (word & 63) + 1;
        if n > 16 {
            let s = spell.add((word >> 6) as usize);
            crate::lz::copy16(s.add(16), d.add(16));
            if n > 32 {
                crate::lz::copy16(s.add(32), d.add(32));
                if n > 48 {
                    crate::lz::copy16(s.add(48), d.add(48));
                }
            }
        }
    }
}
#[cfg(target_arch = "aarch64")]
pub(super) fn expand(ids: &[u32], spell: &[u8], dst: &mut [u8]) -> Result<()> {
    use core::arch::aarch64::*;
    let mut at = 0usize;
    let mut pos = [0u32; 32];
    for (k, tile) in ids.chunks(32).enumerate() {
        let ahead = (k + 2) * 32;
        if ahead + 32 <= ids.len() {
            for &v in &ids[ahead..ahead + 32] {
                hint(unsafe { spell.as_ptr().add((v >> 6) as usize) });
            }
        }
        let mut end = at as u32;
        let mut j = 0;
        unsafe {
            let zero = vdupq_n_u32(0);
            while j + 4 <= tile.len() {
                let widths = vaddq_u32(
                    vandq_u32(vld1q_u32(tile.as_ptr().add(j)), vdupq_n_u32(63)),
                    vdupq_n_u32(1),
                );
                let pair = vaddq_u32(widths, vextq_u32::<3>(zero, widths));
                let total = vaddq_u32(pair, vextq_u32::<2>(zero, pair));
                vst1q_u32(
                    pos.as_mut_ptr().add(j),
                    vaddq_u32(vdupq_n_u32(end), vextq_u32::<3>(zero, total)),
                );
                end += vgetq_lane_u32::<3>(total);
                j += 4;
            }
        }
        for (i, &v) in tile.iter().enumerate().skip(j) {
            pos[i] = end;
            end += (v & 63) + 1;
        }
        if end as usize > dst.len() {
            return Err(bad());
        }
        if end as usize + 64 <= dst.len() {
            let mut i = 0;
            unsafe {
                while i + 4 <= tile.len() {
                    let a = tile[i];
                    let b = tile[i + 1];
                    let c = tile[i + 2];
                    let f = tile[i + 3];
                    let (u, v, w, x): (uint8x16_t, uint8x16_t, uint8x16_t, uint8x16_t);
                    core::arch::asm!("ldr {u:q}, [{a}]","ldr {v:q}, [{b}]","ldr {w:q}, [{c}]","ldr {x:q}, [{f}]",
                        a=in(reg)spell.as_ptr().add((a>>6)as usize),b=in(reg)spell.as_ptr().add((b>>6)as usize),c=in(reg)spell.as_ptr().add((c>>6)as usize),f=in(reg)spell.as_ptr().add((f>>6)as usize),
                        u=lateout(vreg)u,v=lateout(vreg)v,w=lateout(vreg)w,x=lateout(vreg)x,options(readonly,nostack,preserves_flags));
                    let d = dst.as_mut_ptr();
                    store(a, u, spell.as_ptr(), d.add(pos[i] as usize));
                    store(b, v, spell.as_ptr(), d.add(pos[i + 1] as usize));
                    store(c, w, spell.as_ptr(), d.add(pos[i + 2] as usize));
                    store(f, x, spell.as_ptr(), d.add(pos[i + 3] as usize));
                    i += 4;
                }
                for (j, &a) in tile.iter().enumerate().skip(i) {
                    let q = vld1q_u8(spell.as_ptr().add((a >> 6) as usize));
                    store(a, q, spell.as_ptr(), dst.as_mut_ptr().add(pos[j] as usize));
                }
            }
        } else {
            for (j, &v) in tile.iter().enumerate() {
                let n = (v as usize & 63) + 1;
                let q = (v >> 6) as usize;
                let at = pos[j] as usize;
                dst[at..at + n].copy_from_slice(&spell[q..q + n]);
            }
        }
        at = end as usize;
    }
    if at == dst.len() { Ok(()) } else { Err(bad()) }
}
#[cfg(all(test, target_arch = "aarch64"))]
mod tests {
    use super::*;
    #[test]
    fn pipelined_copies_preserve_all_widths_and_guard_bytes() {
        let mut spell = Vec::new();
        let mut desc = Vec::new();
        for n in 1..=64 {
            desc.push((spell.len() as u32) << 6 | (n - 1) as u32);
            spell.extend((0..n).map(|j| (21 * n + 7 * j) as u8));
        }
        spell.extend([0; 64]);
        let mut ids: Vec<u32> = (0..197).map(|i| (19 * i) % 64).collect();
        let mut want = Vec::new();
        for &r in &ids {
            let w = desc[r as usize] as usize;
            want.extend(&spell[w >> 6..(w >> 6) + (w & 63) + 1]);
        }
        remap(&mut ids, &desc).unwrap();
        let mut out = vec![0xd7; want.len() + 64];
        expand(&ids, &spell, &mut out[32..32 + want.len()]).unwrap();
        assert_eq!(&out[32..32 + want.len()], want);
        assert!(
            out[..32]
                .iter()
                .chain(&out[32 + want.len()..])
                .all(|&b| b == 0xd7)
        );
        assert!(expand(&ids, &spell, &mut out[..want.len() - 1]).is_err());
        assert!(remap(&mut [0, 64], &desc).is_err());
    }
}
