//! Ground truth for the inventory scan timing: tail EE.log, and from the moment
//! the game requests the inventory (SyncInventoryFromDB) scan memory repeatedly
//! for a while, reporting every `"MiscItems":[` hit with the length of the
//! intact JSON run around it. Run as release while playing.
//! cargo run --release -p wfc-core --example inv_probe

use std::io::{Read, Seek, SeekFrom};
use std::time::{Duration, Instant};

fn extent(buf: &[u8], pos: usize) -> (usize, usize) {
    // bytes of printable JSON-ish text before/after the anchor
    let ok = |b: u8| (0x20..0x7f).contains(&b) || b == b'\n' || b == b'\r' || b == b'\t';
    let back = buf[..pos].iter().rev().take_while(|&&b| ok(b)).count();
    let fwd = buf[pos..].iter().take_while(|&&b| ok(b)).count();
    (back, fwd)
}

fn scan_once(t0: Instant, label: &str) {
    let t = Instant::now();
    let mut hits = Vec::new();
    let _ = wfc_core::memory::walk_memory(|base, buf| {
        for pos in memchr::memmem::find_iter(buf, b"\"MiscItems\":[") {
            let (b, f) = extent(buf, pos);
            hits.push(format!("{:#x}+{} back={} fwd={} region={}", base, pos, b, f, buf.len()));
        }
    });
    let doc = wfc_core::memory::read_inventory_json().map(|v| v.get("MiscItems").and_then(|m| m.as_array()).map(|a| a.len()).unwrap_or(0));
    println!(
        "[{:6.1}s] {label}: scan {:.1}s, anchors={} intact_doc={:?}\n    {}",
        t0.elapsed().as_secs_f32(),
        t.elapsed().as_secs_f32(),
        hits.len(),
        doc.ok(),
        hits.join("\n    ")
    );
}

fn main() -> anyhow::Result<()> {
    let path = wfc_core::ee_log_path();
    let t0 = Instant::now();
    println!("probe armed on {}", path.display());
    scan_once(t0, "baseline");
    let mut off = std::fs::metadata(&path)?.len();
    loop {
        std::thread::sleep(Duration::from_millis(150));
        let Ok(mut f) = std::fs::File::open(&path) else { continue };
        let len = f.metadata()?.len();
        if len < off {
            off = 0;
        }
        if len == off {
            continue;
        }
        f.seek(SeekFrom::Start(off))?;
        let mut s = String::new();
        f.read_to_string(&mut s).ok();
        off = len;
        if s.contains("SyncInventoryFromDB") || s.contains("OnInventoryResults, body") {
            println!("[{:6.1}s] log: {}", t0.elapsed().as_secs_f32(), s.lines().find(|l| l.contains("Inventory")).unwrap_or(""));
            let until = Instant::now() + Duration::from_secs(40);
            let mut n = 0;
            while Instant::now() < until {
                n += 1;
                scan_once(t0, &format!("after-request #{n}"));
            }
        }
    }
}
