//! Live check of the memory scanner against a running Warframe.
//! cargo run -p wfc-core --example scan_inventory

use wfc_core::{export::Export, market::Market};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let t = std::time::Instant::now();
    let pid = wfc_core::memory::find_pid();
    println!("warframe pid: {pid:?}");
    let v = wfc_core::memory::read_inventory_json()?;
    println!("found inventory JSON in {:.1}s, {} top-level keys", t.elapsed().as_secs_f32(), v.as_object().map(|o| o.len()).unwrap_or(0));
    let mut keys: Vec<&String> = v.as_object().unwrap().keys().collect();
    keys.sort();
    println!("keys: {}", keys.iter().take(60).map(|s| s.as_str()).collect::<Vec<_>>().join(", "));

    let http = reqwest::Client::builder().user_agent("VoidTrace/0.1").build()?;
    let ex = Export::fetch(&http).await?;
    let snap = wfc_core::inventory::parse(&ex, &v);
    println!("plat={} credits={} ducats={} items={}", snap.platinum, snap.credits, snap.ducats, snap.items.len());

    let market = Market::new(http);
    market.load_items().await?;
    let mut tradeable = 0;
    let mut unresolved = 0;
    for it in &snap.items {
        if it.name.starts_with('/') || it.name.contains("Prime") && it.name.split(' ').count() == 1 {
            unresolved += 1;
        }
        if market.find_item_exact(&it.name).is_some() {
            tradeable += 1;
        }
    }
    println!("tradeable on market: {tradeable}, unresolved names: {unresolved}");
    for it in snap.items.iter().filter(|i| market.find_item_exact(&i.name).is_some()).take(25) {
        println!("  {:>4} x {}", it.count, it.name);
    }
    Ok(())
}
