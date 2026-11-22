// PERF_CTL and PERF_ACK can be connected to perf stat/record --control FIFOs.
use btrcmpr::{Preset, Result, compress_preset, compress_with_block, decompress, decompress_with};
#[path = "../bench/support/perf.rs"]
mod perf;
#[path = "../bench/support/zstd.rs"]
mod zstd;
use perf::signal;
fn main() -> Result<()> {
    eprintln!("zstd version {}", unsafe { zstd::ZSTD_versionNumber() });
    let arg: Vec<_> = std::env::args().collect();
    assert_eq!(arg.len(), 6, "profile c|d ours|zstd FILE THREADS REPEATS");
    let src = std::fs::read(&arg[3])?;
    let nt = arg[4].parse().unwrap();
    let reps: usize = arg[5].parse().unwrap();
    let baseline = arg[2] == "zstd";
    let level = std::env::var("BTR_LEVEL")
        .unwrap_or_else(|_| "3".into())
        .parse::<u8>()
        .unwrap();
    let preset = Preset::new(level)?;
    let wide = std::env::var_os("ZSTD_WIDE").is_some();
    let enc = || {
        if baseline {
            let block = if wide {
                src.len().div_ceil(nt).max(1)
            } else {
                btrcmpr::block_size(src.len(), nt)
            };
            compress_with_block(&src, nt, block, |s| zstd::zenc(s, level as i32))
        } else {
            compress_preset(&src, nt, preset)
        }
    };
    let dec = |f: &btrcmpr::Frame, d: &mut [u8]| {
        if baseline {
            decompress_with(f, nt, d, zstd::zdec)
        } else {
            decompress(f, nt, d)
        }
    };
    let frame = enc()?;
    eprintln!(
        "level={level} threads={nt} archive_bytes={}",
        frame.encoded_size()
    );
    let mut dst = vec![0; src.len()];
    dec(&frame, &mut dst)?;
    assert!(src == dst);
    signal(b"enable\n")?;
    let now = std::time::Instant::now();
    for _ in 0..reps {
        match arg[1].as_str() {
            "c" => {
                std::hint::black_box(enc()?);
            }
            "d" => {
                dec(&frame, &mut dst)?;
                std::hint::black_box(&dst);
            }
            _ => panic!("c|d"),
        }
    }
    let sec = now.elapsed().as_secs_f64();
    signal(b"disable\n")?;
    eprintln!("{:.2} MB/s", reps as f64 * src.len() as f64 / 1e6 / sec);
    Ok(())
}
