//! Hex/ASCII dump of game memory around absolute addresses (debug aid).
//! cargo run --release -p wfc-core --example dump_at -- 0x20c979d7fb1 0x20c979eeaf6

fn main() -> anyhow::Result<()> {
    let addrs: Vec<usize> = std::env::args().skip(1).map(|a| usize::from_str_radix(a.trim_start_matches("0x"), 16).unwrap()).collect();
    wfc_core::memory::walk_memory(|base, buf| {
        for &a in &addrs {
            if a >= base && a < base + buf.len() {
                let off = a - base;
                let lo = off.saturating_sub(200);
                let hi = (off + 200).min(buf.len());
                println!("== {a:#x} (region {base:#x} len {} prot?) bytes [{}..{}):", buf.len(), lo, hi);
                let s: String = buf[lo..hi]
                    .iter()
                    .map(|&b| if (0x20..0x7f).contains(&b) { (b as char).to_string() } else { format!("\\x{b:02x}") })
                    .collect();
                println!("{s}\n");
            }
        }
    })?;
    Ok(())
}
