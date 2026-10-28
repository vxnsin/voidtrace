//! Look for the account id the game sends with its requests ("accountId=<24 hex>").
//! cargo run --release -p wfc-core --example find_account

fn main() -> anyhow::Result<()> {
    let mut counts: std::collections::HashMap<String, usize> = Default::default();
    wfc_core::memory::walk_memory(|_, buf| {
        for key in [&b"accountId="[..], &b"\"accountId\":\""[..]] {
            for pos in memchr::memmem::find_iter(buf, key) {
                let s = pos + key.len();
                if let Some(id) = buf.get(s..s + 24) {
                    if id.iter().all(|c| c.is_ascii_hexdigit()) {
                        *counts.entry(String::from_utf8_lossy(id).to_string()).or_default() += 1;
                    }
                }
            }
        }
    })?;
    let mut v: Vec<_> = counts.into_iter().collect();
    v.sort_by(|a, b| b.1.cmp(&a.1));
    for (id, n) in v.iter().take(5) {
        println!("{id}  x{n}");
    }
    Ok(())
}
