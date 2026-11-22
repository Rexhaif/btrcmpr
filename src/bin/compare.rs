use btrcmpr::{
    Frame, Preset, Result, block_size, compress_preset, compress_with_block, decompress,
    decompress_with,
};
use std::{env, fs, time::Instant};
#[path = "../../bench/support/zstd.rs"]
mod zstd;
use zstd::*;
fn main() -> Result<()> {
    let dec_iters: usize = env::var("DEC_ITERS")
        .unwrap_or_else(|_| "4".into())
        .parse()
        .unwrap();
    assert!(dec_iters > 0);
    use std::io::Write;
    let mut trials = env::var_os("TRIALS_CSV")
        .map(fs::File::create)
        .transpose()?;
    if let Some(w) = &mut trials {
        writeln!(
            w,
            "file,codec,round,order,raw_bytes,compressed_bytes,compress_seconds,decompress_seconds"
        )?
    }
    let mut args = env::args().skip(1);
    let threads: usize = args.next().expect("THREADS").parse().unwrap();
    let rounds: usize = args.next().expect("ROUNDS").parse().unwrap();
    let which = args.next().expect("both|ours|zstd");
    let level: i32 = env::var("ZSTD_LEVEL")
        .unwrap_or_else(|_| "3".into())
        .parse()
        .unwrap();
    assert!(rounds > 0 && threads > 0);
    let wide = env::var_os("ZSTD_WIDE").is_some();
    let preset = Preset::new(
        env::var("BTR_LEVEL")
            .ok()
            .map(|s| s.parse())
            .transpose()
            .map_err(std::io::Error::other)?
            .unwrap_or(3),
    )?;
    let warmup = usize::from(env::var("WARMUP").as_deref() != Ok("0"));
    eprintln!(
        "threads={threads}, measured rounds={rounds}, zstd version={}, level={level}",
        unsafe { ZSTD_versionNumber() }
    );
    println!("file,codec,raw_bytes,compressed_bytes,compress_MBps,decompress_MBps");
    for path in args {
        let src = fs::read(&path)?;
        let mut dst = vec![0; src.len()];
        let mut c = [vec![], vec![]];
        let mut d = [vec![], vec![]];
        let mut sizes = [0; 2];
        for r in 1 - warmup..=rounds {
            for (order, x) in [r % 2, 1 - r % 2].into_iter().enumerate() {
                if (x == 0 && which == "zstd") || (x == 1 && which == "ours") {
                    continue;
                }
                let now = Instant::now();
                let f: Frame = if x == 0 {
                    compress_preset(&src, threads, preset)?
                } else {
                    let block = if wide {
                        src.len().div_ceil(threads).max(1)
                    } else {
                        block_size(src.len(), threads)
                    };
                    compress_with_block(&src, threads, block, |s| zenc(s, level))?
                };
                let ct = now.elapsed().as_secs_f64();
                sizes[x] = f.encoded_size();
                for (v, &b) in dst.iter_mut().zip(&src) {
                    *v = !b
                }
                let now = Instant::now();
                for _ in 0..dec_iters {
                    if x == 0 {
                        decompress(&f, threads, &mut dst)?
                    } else {
                        decompress_with(&f, threads, &mut dst, zdec)?
                    }
                }
                let dt = now.elapsed().as_secs_f64() / dec_iters as f64;
                assert!(dst[..] == src[..], "round trip: {path} codec {x}");
                if r > 0 {
                    c[x].push(ct);
                    d[x].push(dt);
                    if let Some(w) = &mut trials {
                        writeln!(
                            w,
                            "{path},{},{r},{order},{},{},{ct:.9},{dt:.9}",
                            ["btrcmpr", "zstd"][x],
                            src.len(),
                            sizes[x]
                        )?
                    }
                }
            }
        }
        for x in 0..2 {
            if c[x].is_empty() {
                continue;
            }
            c[x].sort_by(f64::total_cmp);
            d[x].sort_by(f64::total_cmp);
            let mb = src.len() as f64 / 1e6;
            println!(
                "{path},{},{},{},{:.2},{:.2}",
                ["btrcmpr", "zstd"][x],
                src.len(),
                sizes[x],
                mb / c[x][rounds / 2],
                mb / d[x][rounds / 2]
            )
        }
    }
    Ok(())
}
