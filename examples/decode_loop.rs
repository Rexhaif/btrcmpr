// Reuse the same compressed frame to profile decompression in isolation.
use btrcmpr::{Frame, Result, decompress};
#[path = "../bench/support/perf.rs"]
mod perf;
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    let src = std::fs::read(&args[1])?;
    let f = Frame::read(&src)?;
    let n: usize = args[2].parse().unwrap();
    let threads = args[3].parse().unwrap();
    let mut dst = vec![0; f.size()];
    let reference = args.get(4).map(std::fs::read).transpose()?;
    decompress(&f, threads, &mut dst)?;
    if let Some(raw) = &reference {
        assert_eq!(raw.as_slice(), dst.as_slice(), "warmup decode");
    }
    perf::signal(b"enable\n")?;
    let ts = std::time::Instant::now();
    for _ in 0..n {
        decompress(&f, threads, &mut dst)?;
        std::hint::black_box(&dst);
    }
    let elapsed = ts.elapsed().as_secs_f64();
    perf::signal(b"disable\n")?;
    if let Some(raw) = &reference {
        assert_eq!(raw.as_slice(), dst.as_slice(), "timed decode");
    }
    eprintln!("{:.1} MB/s", n as f64 * f.size() as f64 / 1e6 / elapsed);
    Ok(())
}
