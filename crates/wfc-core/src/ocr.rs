//! Relic reward screen OCR using the Windows built-in OCR engine. No Tesseract
//! binary to ship. Captures the primary monitor, crops the reward row, reads
//! the text and fuzzy-matches it against the market catalogue.

use crate::market::{Market, PriceSummary};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardHit {
    pub raw: String,
    pub price: Option<PriceSummary>,
    pub best: bool,
}

/// Fraction of the screen where the four reward cards sit at 1080p/16:9.
/// Overridable from settings for other aspect ratios.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Crop {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Default for Crop {
    fn default() -> Self {
        Self { x: 0.18, y: 0.40, w: 0.64, h: 0.14 }
    }
}

pub fn capture_crop(crop: Crop) -> Result<(u32, u32, Vec<u8>)> {
    let monitors = xcap::Monitor::all()?;
    let m = monitors
        .into_iter()
        .find(|m| m.is_primary().unwrap_or(false))
        .context("no primary monitor")?;
    let img = m.capture_image()?;
    let (fw, fh) = (img.width() as f32, img.height() as f32);
    let x = (crop.x * fw) as u32;
    let y = (crop.y * fh) as u32;
    let w = (crop.w * fw) as u32;
    let h = (crop.h * fh) as u32;
    let sub = image_crop(&img, x, y, w, h);
    // RGBA -> BGRA for the Windows bitmap.
    let mut bgra = Vec::with_capacity((w * h * 4) as usize);
    for px in sub.chunks(4) {
        bgra.extend_from_slice(&[px[2], px[1], px[0], 255]);
    }
    Ok((w, h, bgra))
}

fn image_crop(img: &xcap::image::RgbaImage, x: u32, y: u32, w: u32, h: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity((w * h * 4) as usize);
    for yy in y..y + h {
        for xx in x..x + w {
            let p = img.get_pixel(xx.min(img.width() - 1), yy.min(img.height() - 1));
            out.extend_from_slice(&p.0);
        }
    }
    out
}

#[cfg(windows)]
pub fn recognize(w: u32, h: u32, bgra: &[u8]) -> Result<Vec<String>> {
    use windows::Graphics::Imaging::{BitmapPixelFormat, SoftwareBitmap};
    use windows::Media::Ocr::OcrEngine;
    use windows::Storage::Streams::DataWriter;

    let writer = DataWriter::new()?;
    writer.WriteBytes(bgra)?;
    let buffer = writer.DetachBuffer()?;
    let bmp = SoftwareBitmap::CreateCopyFromBuffer(&buffer, BitmapPixelFormat::Bgra8, w as i32, h as i32)?;
    let engine = OcrEngine::TryCreateFromUserProfileLanguages().context("no OCR language installed")?;
    let result = engine.RecognizeAsync(&bmp)?.get()?;
    let mut lines = Vec::new();
    for line in result.Lines()? {
        lines.push(line.Text()?.to_string());
    }
    Ok(lines)
}

#[cfg(not(windows))]
pub fn recognize(_w: u32, _h: u32, _bgra: &[u8]) -> Result<Vec<String>> {
    anyhow::bail!("OCR only on Windows")
}

/// The reward row prints up to four names side by side; the OCR gives us lines
/// that may merge cards. Split on wide gaps and on known suffixes.
fn split_candidates(lines: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for l in lines {
        for part in l.split("  ") {
            let p = part.trim();
            if p.len() > 3 {
                out.push(p.to_string());
            }
        }
    }
    out
}

pub async fn read_rewards(market: &Market, crop: Crop) -> Result<Vec<RewardHit>> {
    let (w, h, bgra) = tokio::task::spawn_blocking(move || capture_crop(crop)).await??;
    let lines = tokio::task::spawn_blocking(move || recognize(w, h, &bgra)).await??;
    let mut hits = Vec::new();
    for raw in split_candidates(&lines) {
        let price = match market.find_item(&raw) {
            Some(mi) => market.price(&mi.slug).await.ok(),
            None => None,
        };
        hits.push(RewardHit { raw, price, best: false });
    }
    let best = hits
        .iter()
        .enumerate()
        .max_by_key(|(_, h)| h.price.as_ref().and_then(|p| p.sell).unwrap_or(0))
        .map(|(i, _)| i);
    if let Some(i) = best {
        hits[i].best = true;
    }
    Ok(hits)
}
