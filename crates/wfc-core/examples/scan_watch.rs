//! Poll the game memory for an intact inventory document and report when it
//! appears (use while triggering a loading screen in game).
//! cargo run --release -p wfc-core --example scan_watch

fn main() -> anyhow::Result<()> {
    let started = std::time::Instant::now();
    loop {
        let t = std::time::Instant::now();
        match wfc_core::memory::read_inventory_json() {
            Ok(v) => {
                let o = v.as_object().unwrap();
                let misc = o.get("MiscItems").and_then(|m| m.as_array()).map(|a| a.len()).unwrap_or(0);
                let rec = o.get("Recipes").and_then(|m| m.as_array()).map(|a| a.len()).unwrap_or(0);
                println!(
                    "FOUND after {:.0}s (scan {:.1}s): keys={} MiscItems={misc} Recipes={rec} plat={} credits={}",
                    started.elapsed().as_secs_f32(),
                    t.elapsed().as_secs_f32(),
                    o.len(),
                    o.get("PremiumCredits").cloned().unwrap_or_default(),
                    o.get("RegularCredits").cloned().unwrap_or_default()
                );
                std::fs::write(std::env::temp_dir().join("voidtrace_inventory_sample.json"), serde_json::to_vec(&v)?)?;
                return Ok(());
            }
            Err(e) => println!("{:>4.0}s  not yet ({e}) scan {:.1}s", started.elapsed().as_secs_f32(), t.elapsed().as_secs_f32()),
        }
        if started.elapsed().as_secs() > 600 {
            anyhow::bail!("gave up after 10 minutes");
        }
        std::thread::sleep(std::time::Duration::from_secs(2));
    }
}
