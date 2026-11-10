//! The inventory document seems to live in fixed-size blocks rather than one
//! buffer. List every large run of printable text that contains inventory
//! item paths: where it is, how long, how it starts and ends.
//! cargo run --release -p wfc-core --example chunk_probe

fn printable(b: u8) -> bool {
    (0x20..0x7f).contains(&b) || b == b'\n' || b == b'\r' || b == b'\t'
}

fn main() -> anyhow::Result<()> {
    let t = std::time::Instant::now();
    let mut runs: Vec<(usize, usize, String, String, usize)> = Vec::new();
    wfc_core::memory::walk_memory(|base, buf| {
        let mut i = 0;
        while i < buf.len() {
            if !printable(buf[i]) {
                i += 1;
                continue;
            }
            let start = i;
            while i < buf.len() && printable(buf[i]) {
                i += 1;
            }
            let run = &buf[start..i];
            if run.len() >= 4096 && memchr::memmem::find(run, b"\"ItemType\":\"/Lotus/").is_some() {
                let head = String::from_utf8_lossy(&run[..48.min(run.len())]).to_string();
                let tail = String::from_utf8_lossy(&run[run.len().saturating_sub(48)..]).to_string();
                let items = memchr::memmem::find_iter(run, b"\"ItemType\"").count();
                runs.push((base + start, run.len(), head, tail, items));
            }
        }
    })?;
    runs.sort_by_key(|r| r.0);
    println!("scan {:.1}s, {} runs, total {} bytes", t.elapsed().as_secs_f32(), runs.len(), runs.iter().map(|r| r.1).sum::<usize>());
    for (addr, len, head, tail, items) in &runs {
        println!("{addr:#x} len={len:<7} items={items:<4} head={head:?}\n{:>22} tail={tail:?}", "");
    }
    Ok(())
}
