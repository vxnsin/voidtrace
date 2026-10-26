//! Offline check of inventory parsing/name resolution on a saved document.
//! cargo run --release -p wfc-core --example parse_sample

use wfc_core::{export::Export, market::Market};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let path = std::env::temp_dir().join("voidtrace_inventory_sample.json");
    let v: serde_json::Value = serde_json::from_slice(&std::fs::read(&path)?)?;
    let http = reqwest::Client::builder().user_agent("VoidTrace/0.1").build()?;
    let ex = Export::fetch(&http).await?;
    let snap = wfc_core::inventory::parse(&ex, &v);
    println!("plat={} credits={} ducats={} items={}", snap.platinum, snap.credits, snap.ducats, snap.items.len());
    let market = Market::new(http);
    market.load_items().await?;
    let unresolved: Vec<&_> = snap.items.iter().filter(|i| i.name.starts_with('/')).collect();
    println!("unresolved paths: {}", unresolved.len());
    for u in unresolved.iter().take(10) {
        println!("  ? {}", u.path);
    }
    let tradeable: Vec<_> = snap.items.iter().filter(|i| market.find_item_exact(&i.name).map(|m| m.tradable).unwrap_or(false)).collect();
    println!("tradeable on market: {}", tradeable.len());
    for it in tradeable.iter().take(40) {
        println!("  {:>4} x {}", it.count, it.name);
    }
    let prime_like: Vec<_> = snap.items.iter().filter(|i| i.name.contains("Prime") && market.find_item_exact(&i.name).is_none()).take(15).collect();
    println!("prime-looking but no market match: {}", prime_like.len());
    for it in prime_like {
        println!("  ! {} <- {}", it.name, it.path);
    }
    Ok(())
}
