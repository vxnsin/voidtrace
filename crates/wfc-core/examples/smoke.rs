//! End-to-end data smoke test without the UI.
//! cargo run -p wfc-core --example smoke

use wfc_core::{export::Export, market::Market};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let http = reqwest::Client::builder().user_agent("VoidTrace/0.1").build()?;
    let ex = Export::fetch(&http).await?;
    println!(
        "export: warframes={} weapons={} sentinels={} relics={} images={} item_names={}",
        ex.warframes.len(), ex.weapons.len(), ex.sentinels.len(), ex.relics.len(), ex.images.len(), ex.item_names.len()
    );
    println!("masterable: {}", ex.all_masterable().count());

    let market = Market::new(http.clone());
    println!("market items: {}", market.load_items().await?);

    let relic = ex.relics.iter().find(|r| r.name == "Lith H2").expect("Lith H2");
    for d in &relic.relic_rewards {
        let name = ex.reward_name(&d.reward_name);
        let hit = market.find_item(&name).map(|i| i.slug).unwrap_or_else(|| "NO MATCH".into());
        println!("  {:<40} -> {:<40} {}", d.reward_name.rsplit('/').next().unwrap(), name, hit);
    }

    let plan = wfc_core::relic::plan(&ex, &market, relic).await;
    println!("Lith H2 expected plat per refinement: {:?}", plan.expected_plat);

    let hits = wfc_core::relic::relics_for(&ex, "Orthos Prime Blueprint");
    println!("relics dropping Orthos Prime Blueprint: {:?}", hits.iter().map(|h| &h.0).take(5).collect::<Vec<_>>());

    let g = wfc_core::riven::grade(
        &ex,
        "Kuva Bramma",
        &[
            wfc_core::riven::StatInput { stat: "damage".into(), value: 160.0, negative: false },
            wfc_core::riven::StatInput { stat: "multishot".into(), value: 95.0, negative: false },
            wfc_core::riven::StatInput { stat: "zoom".into(), value: -50.0, negative: true },
        ],
    );
    println!("riven {} dispo {:.2}: {:?}", g.weapon, g.disposition, g.stats.iter().map(|s| format!("{} {}", s.stat, s.grade)).collect::<Vec<_>>());

    let summary = wfc_core::mastery::summary(&ex, None);
    println!("mastery total items: {} (points available {})", summary.total, summary.points_available);
    Ok(())
}
