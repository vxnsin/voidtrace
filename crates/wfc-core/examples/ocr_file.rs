//! Run the riven OCR crops on a saved screenshot with every preparation mode.
//! cargo run --release -p wfc-core --example ocr_file -- <png>

use wfc_core::ocr::{prep_text, recognize, Crop, Prep};

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("png path");
    let img = xcap::image::open(&path)?.to_rgba8();
    let crops = [
        ("card", Crop { x: 0.40, y: 0.58, w: 0.20, h: 0.22 }),
        ("fits", Crop { x: 0.82, y: 0.64, w: 0.16, h: 0.26 }),
    ];
    for (name, crop) in crops {
        for prep in [Prep::Color, Prep::InvertGray, Prep::Threshold] {
            for scale in [1, 2, 3] {
                let (w, h, px) = prep_text(&img, crop, scale, prep);
                let lines = recognize(w, h, &px).unwrap_or_default();
                println!("{name:<4} {prep:?} x{scale}: {lines:?}");
            }
        }
    }
    Ok(())
}
