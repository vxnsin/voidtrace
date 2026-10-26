//! Count inventory-related markers in Warframe's memory (UTF-8 and UTF-16) and
//! show context around the first hits, to learn how the document is laid out.
//! cargo run -p wfc-core --example mem_probe

fn utf16(s: &str) -> Vec<u8> {
    s.encode_utf16().flat_map(|c| c.to_le_bytes()).collect()
}

fn main() -> anyhow::Result<()> {
    let markers = ["\"RegularCredits\"", "\"PremiumCredits\"", "\"MiscItems\"", "\"XPInfo\"", "\"RawUpgrades\"", "\"Recipes\"", "PrimeBucks", "\"ItemCount\""];
    let mut counts8 = vec![0usize; markers.len()];
    let mut counts16 = vec![0usize; markers.len()];
    let mut contexts: Vec<String> = Vec::new();
    let mut total = 0usize;
    let mut regions = 0usize;
    let pats16: Vec<Vec<u8>> = markers.iter().map(|m| utf16(m)).collect();

    wfc_core::memory::walk_memory(|base, buf| {
        regions += 1;
        total += buf.len();
        for (i, m) in markers.iter().enumerate() {
            let n8 = memchr::memmem::find_iter(buf, m.as_bytes()).count();
            counts8[i] += n8;
            counts16[i] += memchr::memmem::find_iter(buf, &pats16[i]).count();
            if i == 2 && n8 > 0 && contexts.len() < 4 {
                for pos in memchr::memmem::find_iter(buf, m.as_bytes()).take(2) {
                    let lo = pos.saturating_sub(300);
                    let hi = (pos + 200).min(buf.len());
                    contexts.push(format!("@{:#x}: {}", base + pos, String::from_utf8_lossy(&buf[lo..hi]).replace('\n', "\\n")));
                }
            }
        }
    })?;

    println!("regions={regions} bytes={} MB", total >> 20);
    for (i, m) in markers.iter().enumerate() {
        println!("{m:<20} utf8={:<6} utf16={}", counts8[i], counts16[i]);
    }
    for c in contexts {
        println!("---\n{c}");
    }
    Ok(())
}
