use btrcmpr::{Frame, Preset, Result, compress_preset, decompress};
use std::{env, fs, io};
fn main() -> Result<()> {
    let mut args: Vec<_> = env::args().collect();
    let mut preset = Preset::default();
    if args.get(1).is_some_and(|a| a == "c") && args.get(2).is_some_and(|a| a.starts_with('-')) {
        let opt = args.remove(2);
        preset = Preset::new(opt[1..].parse().map_err(io::Error::other)?)?;
    }
    if !(4..=5).contains(&args.len()) {
        eprintln!(
            "usage: btrcmpr c [-1..-22] INPUT OUTPUT [THREADS]\n       btrcmpr d INPUT OUTPUT [THREADS]"
        );
        std::process::exit(2)
    }
    let threads = args
        .get(4)
        .map(|s| s.parse::<usize>())
        .transpose()
        .map_err(io::Error::other)?
        .unwrap_or_else(|| std::thread::available_parallelism().map_or(1, usize::from));
    if !(1..=256).contains(&threads) {
        return Err(io::Error::other("THREADS must be in 1..=256"));
    }
    let src = fs::read(&args[2])?;
    match args[1].as_str() {
        "c" => {
            let f = compress_preset(&src, threads, preset)?;
            f.write(io::BufWriter::new(fs::File::create(&args[3])?))
        }
        "d" => {
            let f = Frame::read(&src)?;
            let mut out = Vec::new();
            out.try_reserve_exact(f.size()).map_err(io::Error::other)?;
            out.resize(f.size(), 0);
            decompress(&f, threads, &mut out)?;
            fs::write(&args[3], out)
        }
        _ => Err(io::Error::other("use c or d")),
    }
}
