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

/// How small game text is prepared for the OCR engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Prep {
    /// Colour as on screen, only scaled.
    Color,
    /// Grey, inverted: dark text on light background.
    InvertGray,
    /// Inverted with a hard knee (strong glow suppression).
    Threshold,
}

/// Crop, scale up and prepare a region of a screenshot for OCR (BGRA out).
pub fn prep_text(img: &xcap::image::RgbaImage, crop: Crop, scale: u32, prep: Prep) -> (u32, u32, Vec<u8>) {
    let (fw, fh) = (img.width() as f32, img.height() as f32);
    let (x, y) = ((crop.x * fw) as u32, (crop.y * fh) as u32);
    let (w, h) = ((crop.w * fw) as u32, (crop.h * fh) as u32);
    let sub = image_crop(img, x, y, w, h);
    let (sw, sh) = (w * scale, h * scale);
    let mut out = vec![0u8; (sw * sh * 4) as usize];
    for yy in 0..sh {
        for xx in 0..sw {
            let s = (((yy / scale) * w + xx / scale) * 4) as usize;
            let (r, g, b) = (sub[s], sub[s + 1], sub[s + 2]);
            let d = ((yy * sw + xx) * 4) as usize;
            let px = match prep {
                Prep::Color => [b, g, r, 255],
                Prep::InvertGray | Prep::Threshold => {
                    let l = (0.114 * b as f32 + 0.587 * g as f32 + 0.299 * r as f32) as i32;
                    let v = if prep == Prep::Threshold {
                        if l > 150 { 0 } else if l < 90 { 255 } else { (255 - (l - 90) * 255 / 60).clamp(0, 255) }
                    } else {
                        255 - l
                    } as u8;
                    [v, v, v, 255]
                }
            };
            out[d..d + 4].copy_from_slice(&px);
        }
    }
    (sw, sh, out)
}

pub fn screenshot() -> Result<xcap::image::RgbaImage> {
    let m = xcap::Monitor::all()?
        .into_iter()
        .find(|m| m.is_primary().unwrap_or(false))
        .context("no primary monitor")?;
    Ok(m.capture_image()?)
}

/// Screenshot, crop and prepare small game text (riven card, "Fits in" box).
pub fn capture_text(crop: Crop, scale: u32, prep: Prep) -> Result<(u32, u32, Vec<u8>)> {
    Ok(prep_text(&screenshot()?, crop, scale, prep))
}
