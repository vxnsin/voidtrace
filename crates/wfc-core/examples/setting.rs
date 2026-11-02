//! Read or write one app setting in the local DB. Dev aid.
//! cargo run -p wfc-core --example setting -- <key> [value]

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let db = wfc_core::db::Db::open()?;
    match args.as_slice() {
        [k] => println!("{k} = {:?}", db.get(k)),
        [k, v] => {
            db.set(k, v)?;
            println!("{k} = {v:?}");
        }
        _ => anyhow::bail!("usage: setting <key> [value]"),
    }
    Ok(())
}
