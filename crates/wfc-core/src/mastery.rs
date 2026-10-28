//! Mastery helper / foundry: cross the export catalogue with profile XP.

use crate::export::{Export, ExportItem};
use crate::profile::Profile;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasteryItem {
    pub unique_name: String,
    pub name: String,
    pub category: String,
    pub xp: i64,
    pub max_xp: i64,
    pub mastered: bool,
    pub owned: bool,
    pub mastery_points: u32,
    pub image: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MasterySummary {
    pub items: Vec<MasteryItem>,
    pub mastered: usize,
    pub total: usize,
    pub points: u32,
    pub points_available: u32,
    pub rank: u32,
}

/// XP needed for rank 30. Frames, companions, archwings and necramechs give
/// 200 XP/rank and 200 MR/rank; weapons give 100 XP/rank and 100 MR/rank.
fn curve(it: &ExportItem) -> (i64, u32) {
    let heavy = matches!(
        it.product_category.as_str(),
        "Suits" | "SpaceSuits" | "MechSuits" | "Sentinels" | "KubrowPets" | "Hoverboards"
    );
    if heavy { (6000, 6000) } else { (3000, 3000) }
}

pub fn mr_from_points(points: u32) -> u32 {
    // rank r needs 2500 * r^2 points
    let mut r = 0u32;
    while 2500 * (r + 1) * (r + 1) <= points {
        r += 1;
    }
    r
}

pub fn summary(ex: &Export, profile: Option<&Profile>) -> MasterySummary {
    let xp: HashMap<&str, i64> = profile
        .map(|p| p.xp.iter().map(|e| (e.item_type.as_str(), e.xp)).collect())
        .unwrap_or_default();
    let mut items: Vec<MasteryItem> = ex
        .all_masterable()
        .map(|it| {
            let (max_xp, pts) = curve(it);
            let got = xp.get(it.unique_name.as_str()).copied().unwrap_or(0);
            let mastered = got >= max_xp;
            MasteryItem {
                unique_name: it.unique_name.clone(),
                name: it.name.clone(),
                category: it.product_category.clone(),
                xp: got.min(max_xp),
                max_xp,
                mastered,
                owned: got > 0,
                mastery_points: if mastered { pts } else { (got.max(0) as u32 * pts as u32) / max_xp as u32 },
                image: ex.images.get(&it.unique_name).cloned(),
            }
        })
        .collect();
    items.sort_by(|a, b| a.category.cmp(&b.category).then(a.name.cmp(&b.name)));
    let mastered = items.iter().filter(|i| i.mastered).count();
    let points: u32 = items.iter().map(|i| i.mastery_points).sum();
    let points_available: u32 = items.iter().map(|i| curve_pts(i)).sum();
    // The account's real rank comes with the profile; item XP alone misses star chart,
    // junctions, intrinsics and some item kinds, so it is only the fallback.
    let rank = profile.map(|p| p.player_level).filter(|l| *l > 0).unwrap_or_else(|| mr_from_points(points));
    MasterySummary { total: items.len(), mastered, points, points_available, rank, items }
}

fn curve_pts(i: &MasteryItem) -> u32 {
    if i.max_xp >= 6000 { 6000 } else { 3000 }
}
