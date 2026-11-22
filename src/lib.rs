mod ans;

mod context;

mod grammar;
mod mixed;

mod huff;
mod layout;

mod lz;
mod object;

mod palette;

mod pixel;
mod prefixtree;

mod record;

mod signalgrid;
mod sorted;

mod weight;
mod wordbase;

mod words;
use std::{
    io::{Error, ErrorKind},
    sync::Mutex,
};
pub type Result<T> = std::io::Result<T>;
pub(crate) fn bad() -> Error {
    Error::new(ErrorKind::InvalidData, "invalid compressed block")
}
pub(crate) fn take<'a>(p: &mut &'a [u8], n: usize) -> Result<&'a [u8]> {
    let (a, b) = p.split_at_checked(n).ok_or_else(bad)?;
    *p = b;
    Ok(a)
}
pub(crate) fn u32in(p: &mut &[u8]) -> Result<usize> {
    Ok(u32::from_le_bytes(take(p, 4)?.try_into().unwrap()) as usize)
}
pub const MAX_BLOCK: usize = 2 << 20;
pub const MAX_WIDE_BLOCK: usize = 32 << 20;
pub const MAGIC: &[u8; 8] = b"BTRCMP01";
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Preset(u8);
impl Preset {
    pub fn new(level: u8) -> Result<Self> {
        if (1..=22).contains(&level) {
            Ok(Self(level))
        } else {
            Err(Error::new(
                ErrorKind::InvalidInput,
                "compression level must be 1..=22",
            ))
        }
    }
    pub fn level(self) -> u8 {
        self.0
    }
}
impl Default for Preset {
    fn default() -> Self {
        Self(3)
    }
}
pub fn block_size(size: usize, threads: usize) -> usize {
    if size == 0 {
        return 1;
    }
    // Equal waves avoid leaving three cores idle for the final fraction of an image.
    let blocks = size
        .div_ceil(MAX_BLOCK)
        .next_multiple_of(threads.clamp(1, 256));
    size.div_ceil(blocks).next_multiple_of(64).min(MAX_BLOCK)
}
fn parallel<I: Iterator + Send>(
    threads: usize,
    it: I,
    f: impl Fn(I::Item) -> Result<()> + Sync,
) -> Result<()>
where
    I::Item: Send,
{
    let jobs = Mutex::new(it);
    std::thread::scope(|s| {
        let f = &f;
        let jobs = &jobs;
        let hs: Vec<_> = (0..threads.clamp(1, 256))
            .map(|_| {
                s.spawn(move || -> Result<()> {
                    loop {
                        let job = jobs.lock().unwrap().next();
                        match job {
                            Some(x) => f(x)?,
                            None => return Ok(()),
                        }
                    }
                })
            })
            .collect();
        for h in hs {
            h.join().unwrap()?
        }
        Ok(())
    })
}
pub struct Frame<'a> {
    size: usize,
    block: usize,
    blocks: Vec<std::borrow::Cow<'a, [u8]>>,
    layout: Option<layout::Layout>,
}
impl<'a> Frame<'a> {
    pub fn size(&self) -> usize {
        self.size
    }
    pub fn encoded_size(&self) -> usize {
        20 + self
            .layout
            .as_ref()
            .map_or(0, layout::Layout::serialized_len)
            + self.blocks.iter().map(|v| 4 + v.len()).sum::<usize>()
    }
    pub fn write(&self, mut w: impl std::io::Write) -> Result<()> {
        w.write_all(MAGIC)?;
        w.write_all(&(self.size as u64).to_le_bytes())?;
        let word = self.block as u32 | ((self.layout.is_some() as u32) << 31);
        w.write_all(&word.to_le_bytes())?;
        if let Some(l) = &self.layout {
            l.write(&mut w)?
        }
        for b in &self.blocks {
            w.write_all(&(b.len() as u32).to_le_bytes())?;
            w.write_all(b)?
        }
        w.flush()
    }
    pub fn read(mut p: &'a [u8]) -> Result<Self> {
        if take(&mut p, 8)? != MAGIC {
            return Err(bad());
        }
        let size = usize::try_from(u64::from_le_bytes(take(&mut p, 8)?.try_into().unwrap()))
            .map_err(|_| bad())?;
        let word = u32in(&mut p)?;
        let block = word & 0x7fffffff;
        if !(1..=MAX_WIDE_BLOCK).contains(&block) {
            return Err(bad());
        }
        let layout = if word >> 31 != 0 {
            Some(layout::Layout::read(&mut p, size)?)
        } else {
            None
        };
        let mut blocks = vec![];
        for _ in 0..layout
            .as_ref()
            .map_or_else(|| size.div_ceil(block), |l| l.sizes.len())
        {
            let n = u32in(&mut p)?;
            if n == 0 || n > block + 1 {
                return Err(bad());
            }
            blocks.push(std::borrow::Cow::Borrowed(take(&mut p, n)?))
        }
        if !p.is_empty() {
            return Err(bad());
        }
        Ok(Self {
            size,
            block,
            blocks,
            layout,
        })
    }
}
pub fn compress_with(
    data: &[u8],
    threads: usize,
    f: impl Fn(&[u8]) -> Result<Vec<u8>> + Sync,
) -> Result<Frame<'static>> {
    let block = block_size(data.len(), threads);
    compress_with_block(data, threads, block, f)
}
/// Run independent jobs of a specified size. Benchmark codecs may use their
/// own frame sizes; only native archives are read by `Frame::read`.
pub fn compress_with_block(
    data: &[u8],
    threads: usize,
    block: usize,
    f: impl Fn(&[u8]) -> Result<Vec<u8>> + Sync,
) -> Result<Frame<'static>> {
    if block == 0 || block > 0x7fff_ffff {
        return Err(bad());
    }
    let mut blocks: Vec<_> = data
        .chunks(block)
        .map(|_| std::borrow::Cow::default())
        .collect();
    parallel(
        threads.min(blocks.len()),
        data.chunks(block).zip(&mut blocks),
        |(src, slot)| {
            *slot = std::borrow::Cow::Owned(f(src)?);
            Ok(())
        },
    )?;
    Ok(Frame {
        size: data.len(),
        block,
        blocks,
        layout: None,
    })
}
pub fn decompress_with(
    frame: &Frame,
    threads: usize,
    dst: &mut [u8],
    f: impl Fn(&[u8], &mut [u8]) -> Result<()> + Sync,
) -> Result<()> {
    if dst.len() != frame.size
        || frame.block == 0
        || frame
            .layout
            .as_ref()
            .map_or_else(|| dst.len().div_ceil(frame.block), |l| l.sizes.len())
            != frame.blocks.len()
    {
        return Err(bad());
    }
    if frame.layout.is_some() {
        return layout::restore(frame, threads, dst, false, f);
    }
    parallel(
        threads.min(frame.blocks.len()),
        frame.blocks.iter().zip(dst.chunks_mut(frame.block)),
        |(src, dst)| f(src, dst),
    )
}
pub fn decompress(frame: &Frame, threads: usize, dst: &mut [u8]) -> Result<()> {
    let first = frame.blocks.first().and_then(|b| b.first());
    if first == Some(&50) {
        return grammar::worddag::shared::decompress(frame, threads, dst);
    }
    if let Some(l) = &frame.layout {
        if dst.len() != frame.size || l.sizes.len() != frame.blocks.len() {
            return Err(bad());
        }
        layout::restore(frame, threads, dst, true, decode)
    } else if ((4 << 20) + 1..=5 << 20).contains(&frame.block) && frame.blocks.len() > threads {
        if dst.len() != frame.size || dst.len().div_ceil(frame.block) != frame.blocks.len() {
            return Err(bad());
        }
        // The first stream declares the number of LZ commands. It is only a
        // scheduling hint: decode still validates every block independently.
        let mut jobs: Vec<_> = frame
            .blocks
            .iter()
            .zip(dst.chunks_mut(frame.block))
            .collect();
        jobs.sort_unstable_by_key(|(src, _)| {
            let commands = if matches!(src.first(), Some(1 | 13 | 16 | 17)) {
                src.get(1..5)
                    .map_or(0, |v| u64::from(u32::from_le_bytes(v.try_into().unwrap())))
            } else {
                0
            };
            std::cmp::Reverse(src.len() as u64 + 4 * commands)
        });
        parallel(threads.min(jobs.len()), jobs.into_iter(), |(src, dst)| {
            decode(src, dst)
        })
    } else {
        decompress_with(frame, threads, dst, decode)
    }
}
pub fn compress(data: &[u8], threads: usize) -> Result<Frame<'static>> {
    compress_preset(data, threads, Preset::default())
}

pub fn compress_preset(data: &[u8], threads: usize, preset: Preset) -> Result<Frame<'static>> {
    if let Some(layout) = layout::Layout::detect(data, preset.level()) {
        layout::compress(data, threads, layout, preset.level(), |s| {
            encode_archive(s, preset)
        })
    } else {
        let lv = preset.level();
        let large_text = lv >= 5 && words::detect(data);
        if large_text
            && let Some(plan) = grammar::worddag::shared::Plan::preset(lv)
            && threads >= 2
            && data.len() >= 8 << 20
            && !pixel::detect(data)
        {
            if let Ok(frame) = grammar::worddag::shared::compress(data, threads, plan) {
                return Ok(frame);
            }
            // A dictionary can exceed its first block on unusual inputs. The
            // ordinary independent encoders always have a raw-block fallback.
            if lv == 16 {
                return mixed::preset(data, threads, lv);
            }
        }
        let fast_text = lv == 2 && words::detect(data);
        let signal_job = if lv >= 2 && pixel::detect(data) {
            signalgrid::plan(lv).map(|p| p.kib)
        } else {
            None
        };
        let default_kib = if let Some(kib) = signal_job {
            kib
        } else if fast_text {
            896
        } else if large_text && lv == 5 {
            4096
        } else if large_text && lv == 6 {
            5120
        } else if large_text && lv == 7 {
            12288
        } else if large_text && (8..=15).contains(&lv) {
            24576
        } else if large_text {
            [
                3072, 3584, 4096, 5120, 6144, 7168, 8192, 9216, 10240, 12288, 14336, 16384,
            ][(lv - 11) as usize]
        } else if lv == 1 && !pixel::detect(data) {
            if cfg!(target_arch = "aarch64") {
                384
            } else {
                256
            }
        } else {
            2048
        };
        let limit = default_kib * 1024;
        let jobs = data
            .len()
            .div_ceil(limit)
            .next_multiple_of(threads.clamp(1, 256))
            .max(1);
        let block = data
            .len()
            .div_ceil(jobs)
            .max(1)
            .next_multiple_of(64)
            .min(limit);
        // Large level-one jobs reuse distance and literal models within each region.
        #[cfg(target_arch = "aarch64")]
        if lv == 1 && data.len() >= 1 << 20 && !pixel::detect(data) {
            return lz::sharedentropy::compress_per(data, threads, block, 6, 128, false);
        }

        compress_with_block(data, threads, block, |s| encode_preset(s, preset))
    }
}
pub fn encode(src: &[u8]) -> Result<Vec<u8>> {
    encode_preset(src, Preset::default())
}
pub fn encode_preset(src: &[u8], preset: Preset) -> Result<Vec<u8>> {
    let level = preset.level();
    if src.len() > MAX_WIDE_BLOCK || (src.len() > MAX_BLOCK && matches!(level, 1..=4)) {
        return Err(bad());
    }

    let v = if pixel::detect(src) {
        if let Some(plan) = signalgrid::plan(level) {
            signalgrid::encode_plan(src, level, plan)
        } else {
            pixel::encode_level(src, level)
        }
    } else if level == 2 && words::detect(src) {
        lz::paired_cost::<6, 17, 3, true, 21, 2, true, 14>(src)
    } else if level == 5 && (src.len() > MAX_BLOCK || words::detect(src)) {
        lz::legacy_width::<false, 17, 18, false, 4, 0, 25>(src)
    } else if level == 6 && (src.len() > MAX_BLOCK || words::detect(src)) {
        if src.len() <= 1 << 23 {
            lz::history_model::<false, 17, 20, false, 3, 16, 23, true, 4, 0, true, true, 0, false>(
                src,
            )
        } else {
            lz::history_model::<false, 17, 20, false, 3, 16, 25, true, 4, 0, true, true, 0, false>(
                src,
            )
        }
    } else if level == 7 && (src.len() > MAX_BLOCK || words::detect(src)) {
        lz::legacy_policy::<false, 18, 21, false, 1, 16, 25, true, 4>(src)
    } else if level == 8 && (src.len() > MAX_BLOCK || words::detect(src)) {
        lz::eighth_text(src)
    } else if (9..=12).contains(&level) && (src.len() > MAX_BLOCK || words::detect(src)) {
        lz::middle_text(src, level)
    } else if (13..=15).contains(&level) && (src.len() > MAX_BLOCK || words::detect(src)) {
        grammar::worddag::preset(src, level)
    } else if level >= 10 && (src.len() > MAX_BLOCK || words::detect(src)) {
        lz::optimal(src, level)
    } else {
        lz::encode_level::<false>(src, level)
    };
    if v.len() > src.len() {
        let mut raw = vec![0];
        raw.extend(src);
        Ok(raw)
    } else {
        Ok(v)
    }
}
pub(crate) fn archive_parser(level: u8) -> u8 {
    let chosen = [
        1, 2, 3, 2, 2, 4, 6, 6, 7, 7, 7, 7, 7, 7, 7, 8, 9, 10, 10, 10, 10, 10,
    ][(level - 1) as usize];
    chosen as u8
}
fn encode_archive(src: &[u8], preset: Preset) -> Result<Vec<u8>> {
    let v = object::archive_lz(src, preset.level());
    if v.len() > src.len() {
        let mut r = vec![0];
        r.extend(src);
        Ok(r)
    } else {
        Ok(v)
    }
}
pub fn decode(src: &[u8], dst: &mut [u8]) -> Result<()> {
    if dst.len() > MAX_WIDE_BLOCK {
        return Err(bad());
    }
    let (&mode, mut p) = src.split_first().ok_or_else(bad)?;
    match mode {
        0 if dst.len() == p.len() => {
            dst.copy_from_slice(p);
            Ok(())
        }
        1 => lz::decode(p, dst),
        41 => lz::pages::decode(p, dst),
        2 => pixel::decode(p, dst),
        6 | 10 => pixel::unadjacent(p, dst, mode == 10),
        7 => words::decode(p, dst),
        8 => object::uninstructions(p, dst),
        9 => object::uncolumns(p, dst),
        11 => sorted::decode(p, dst),
        12 => object::unbanked(p, dst),

        33 => object::unsmall(p, dst.len(), &mut [(0, dst)]),
        21 => object::unadaptive(p, dst.len(), &mut [(0, dst)]),
        13 => lz::decode_impl::<true>(p, dst),
        16 => lz::decode_min::<false, 3>(p, dst),
        17 => lz::decode_min::<true, 3>(p, dst),

        42 => lz::dspan::read(p, dst),
        43 => grammar::decode(p, dst),
        48 => grammar::worddag::decode(p, dst),

        4 => record::decode(p, dst),

        63 => signalgrid::get(p, dst),
        5 => {
            let mut rest = dst;
            while !p.is_empty() {
                let n = u32in(&mut p)?;
                let len = u32in(&mut p)?;
                let b = take(&mut p, len)?;
                if n == 0 || n > rest.len() || b.first() == Some(&5) {
                    return Err(bad());
                }
                let (d, r) = rest.split_at_mut(n);
                decode(b, d)?;
                rest = r
            }
            if rest.is_empty() { Ok(()) } else { Err(bad()) }
        }
        _ => Err(bad()),
    }
}
