// The middle block supplies each region’s entropy model. Blocks decode independently.
use super::{Streams, emitoffset};
use crate::{Frame, Result, ans, compress_with_block, palette::fixedpalette::Model};
use std::sync::Mutex;
struct Region {
    pilot: usize,
    frame: Mutex<Option<Vec<u8>>>,
    models: [Option<Model>; 3],
}
fn bounded(mut frame: Vec<u8>, s: &[u8]) -> Vec<u8> {
    if frame.len() > s.len() {
        frame.clear();
        frame.push(0);
        frame.extend(s);
    }
    frame
}
fn pilot(src: &[u8], block: usize, fields: usize, pairs: bool) -> Region {
    let pilot = (src.len().div_ceil(block) - 1) / 2;
    let s = &src[pilot * block..((pilot + 1) * block).min(src.len())];
    let (mut st, anchor) = emitoffset::parsed::<35, 0>(s, 5, (2, 1));
    st.lits.extend_from_slice(&s[anchor..]);
    let models = std::array::from_fn(|i| {
        if fields & (1 << i) == 0 {
            return None;
        }
        let data = match i {
            0 => &st.tag,
            1 => &st.offs,
            _ => &st.lits,
        };
        Model::train(data, pairs)
    });
    let frame = if crate::pixel::detect(s) {
        crate::pixel::encode_level(s, 1)
    } else {
        st.finish(s, s.len(), false)
    };
    Region {
        pilot,
        frame: Mutex::new(Some(bounded(frame, s))),
        models,
    }
}
fn encode(s: &[u8], r: &Region) -> Vec<u8> {
    if crate::pixel::detect(s) {
        return bounded(crate::pixel::encode_level(s, 1), s);
    }

    let (mut st, anchor) = emitoffset::parsed::<35, 0>(s, 5, (2, 1));
    st.lits.extend_from_slice(&s[anchor..]);
    finish(s, &mut st, &r.models)
}
#[cfg(test)]
fn with_worker_buffers(s: &[u8], r: &Region) -> Vec<u8> {
    thread_local! {
        static SLOTS: std::cell::RefCell<Streams> = std::cell::RefCell::new(Streams::new());
    }
    SLOTS.with_borrow_mut(|st| {
        for v in [
            &mut st.lits,
            &mut st.offs,
            &mut st.tag,
            &mut st.lens,
            &mut st.low,
            &mut st.mid,
            &mut st.far,
        ] {
            v.clear();
        }
        let anchor = emitoffset::parsed_into::<35, 0>(s, 5, (2, 1), st);
        st.lits.extend_from_slice(&s[anchor..]);
        finish(s, st, &r.models)
    })
}
fn finish(src: &[u8], st: &mut Streams, m: &[Option<Model>; 3]) -> Vec<u8> {
    st.low.extend([0; 16]);
    st.mid.extend([0; 16]);
    let mut out = Vec::with_capacity(src.len() / 2);
    out.push(if st.far.is_empty() { 1 } else { 13 });
    for (j, v) in [&st.tag, &st.offs, &st.lens, &st.lits, &st.low, &st.mid]
        .into_iter()
        .enumerate()
    {
        let model = match j {
            0 => m[0].as_ref(),
            1 => m[1].as_ref(),
            3 => m[2].as_ref(),
            _ => None,
        };
        if !model.is_some_and(|x| x.put(v, &mut out)) {
            ans::put_config(v, &mut out, 0.985, 0, (2, 1))
        }
    }
    if !st.far.is_empty() {
        ans::put_config(&st.far, &mut out, 0.985, 0, (2, 1))
    }
    bounded(out, src)
}

pub(crate) fn compress_per(
    data: &[u8],
    threads: usize,
    block: usize,
    fields: usize,
    period: usize,
    pairs: bool,
) -> Result<Frame<'static>> {
    let region_len = block * period;
    let mut regions: Vec<Option<Region>> =
        (0..data.len().div_ceil(region_len)).map(|_| None).collect();
    crate::parallel(
        threads.min(regions.len()),
        data.chunks(region_len).zip(&mut regions),
        |(s, r)| {
            *r = Some(pilot(s, block, fields, pairs));
            Ok(())
        },
    )?;
    let base = data.as_ptr() as usize;
    compress_with_block(data, threads, block, |s| {
        let i = (s.as_ptr() as usize - base) / block;
        let r = regions[i / period].as_ref().unwrap();
        Ok(if i % period == r.pilot {
            r.frame.lock().unwrap().take().unwrap()
        } else {
            encode(s, r)
        })
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn independent_frames_restore_across_regions_and_short_last_pilot() {
        let mut v = vec![];
        let mut x = 0xcafe5331u32;
        for i in 0..190_003 {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            v.push(if i % 122 < 20 {
                x as u8
            } else {
                b"distance:abc0\nxyz9-!@"[i % 20]
            });
        }
        for (group, pairs) in [(4, true), (3, false)] {
            let frame = compress_per(&v, 4, 8192, 7, group, pairs).unwrap();
            let mut out = vec![0; v.len()];
            crate::decompress_with(&frame, 3, &mut out, crate::decode).unwrap();
            assert_eq!(out, v);
            let region = pilot(&v, 8192, 7, pairs);
            for s in v.chunks(8192).filter(|s| !crate::pixel::detect(s)) {
                assert_eq!(encode(s, &region), with_worker_buffers(s, &region));
            }
        }
    }
}
