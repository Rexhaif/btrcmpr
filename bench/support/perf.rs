// Optional perf control FIFOs keep warmup and input I/O out of counters.
pub fn signal(cmd: &[u8]) -> std::io::Result<()> {
    use std::io::{BufRead, Write};
    if let (Ok(ctl), Ok(ack)) = (std::env::var("PERF_CTL"), std::env::var("PERF_ACK")) {
        let mut f = std::fs::OpenOptions::new().write(true).open(ctl)?;
        let ack = std::fs::OpenOptions::new().read(true).open(ack)?;
        f.write_all(cmd)?;
        let mut reply = String::new();
        std::io::BufReader::new(ack).read_line(&mut reply)?;
        if reply.trim_end_matches(['\0', '\n']) != "ack" {
            return Err(std::io::Error::other("unexpected perf acknowledgment"));
        }
    }
    Ok(())
}
