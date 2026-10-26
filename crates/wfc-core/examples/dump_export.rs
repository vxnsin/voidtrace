//! Debug helper: print the PublicExport index and a sample of each file we use.
//! cargo run -p wfc-core --example dump_export

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    let raw = client
        .get("https://origin.warframe.com/PublicExport/index_en.txt.lzma")
        .send()
        .await?
        .bytes()
        .await?;
    let mut out = Vec::new();
    lzma_rs::lzma_decompress(&mut std::io::Cursor::new(raw), &mut out)?;
    let index = String::from_utf8_lossy(&out).to_string();
    println!("--- index ---\n{index}");
    for key in ["ExportRelicArcane", "ExportWeapons", "ExportManifest", "ExportWarframes"] {
        let Some(line) = index.lines().find(|l| l.starts_with(key)) else { continue };
        let txt = client
            .get(format!("https://content.warframe.com/PublicExport/Manifest/{}", line.trim()))
            .send()
            .await?
            .text()
            .await?;
        let txt = txt.replace(['\r', '\n'], "");
        let v: serde_json::Value = serde_json::from_str(&txt)?;
        let root = v.as_object().unwrap().keys().next().unwrap().clone();
        let arr = v[&root].as_array().unwrap();
        println!("--- {key}: root={root} len={} ---", arr.len());
        println!("{}", serde_json::to_string_pretty(&arr[0])?);
        if key == "ExportRelicArcane" {
            let relic = arr.iter().find(|e| e["name"].as_str().unwrap_or("").contains("Lith"));
            if let Some(r) = relic {
                println!("{}", serde_json::to_string_pretty(r)?);
            }
        }
    }
    Ok(())
}
