//! DE PublicExport loader. Cached on disk, refreshed when the index hash changes.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

const INDEX_URL: &str = "https://origin.warframe.com/PublicExport/index_en.txt.lzma";
const MANIFEST_URL: &str = "https://content.warframe.com/PublicExport/Manifest/";
const IMAGE_URL: &str = "https://content.warframe.com/PublicExport/";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExportItem {
    #[serde(rename = "uniqueName")]
    pub unique_name: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default, rename = "productCategory")]
    pub product_category: String,
    /// Derived at load time; DE's export has no such flag.
    #[serde(default)]
    pub masterable: bool,
    #[serde(default, rename = "excludeFromCodex")]
    pub exclude_from_codex: bool,
    /// Riven disposition, weapons only.
    #[serde(default, rename = "omegaAttenuation")]
    pub disposition: f32,
    #[serde(default, rename = "totalDamage")]
    pub total_damage: f32,
    #[serde(default, rename = "masteryReq")]
    pub mastery_req: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelicReward {
    #[serde(rename = "rewardName")]
    pub reward_name: String,
    pub rarity: String,
    pub tier: u8,
    #[serde(default, rename = "itemCount")]
    pub item_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportRelic {
    #[serde(rename = "uniqueName")]
    pub unique_name: String,
    pub name: String,
    #[serde(default, rename = "relicRewards")]
    pub relic_rewards: Vec<RelicReward>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Manifest {
    #[serde(rename = "uniqueName")]
    pub unique_name: String,
    #[serde(rename = "textureLocation")]
    pub texture_location: String,
}

#[derive(Debug, Clone, Deserialize)]
struct Resource {
    #[serde(rename = "uniqueName")]
    unique_name: String,
    name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ingredient {
    #[serde(rename = "ItemType")]
    pub item_type: String,
    #[serde(rename = "ItemCount")]
    pub count: i64,
}

/// A foundry blueprint: what it builds and from what.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recipe {
    #[serde(rename = "uniqueName")]
    pub unique_name: String,
    #[serde(rename = "resultType")]
    pub result_type: String,
    #[serde(default)]
    pub ingredients: Vec<Ingredient>,
    #[serde(default, rename = "buildPrice")]
    pub build_price: i64,
}

/// Bump when the cached shape changes so stale caches are refetched.
const CACHE_VERSION: u32 = 3;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Export {
    #[serde(default)]
    pub cache_version: u32,
    pub index_hash: String,
    pub warframes: Vec<ExportItem>,
    pub weapons: Vec<ExportItem>,
    pub sentinels: Vec<ExportItem>,
    /// Lith/Meso/Neo/Axi/Requiem relics with their drop tables.
    pub relics: Vec<ExportRelic>,
    pub images: HashMap<String, String>,
    /// Any /Lotus item path (resources, parts, blueprints, mods, relics) -> display name.
    /// Blueprint recipes resolve to "<result> Blueprint", matching warframe.market.
    #[serde(default)]
    pub item_names: HashMap<String, String>,
    /// Blueprints of masterable items and their components.
    #[serde(default)]
    pub recipes: Vec<Recipe>,
}

const MASTERABLE_CATEGORIES: &[&str] = &[
    "Suits", "SpaceSuits", "MechSuits", "Sentinels", "KubrowPets",
    "LongGuns", "Pistols", "Melee", "SpaceGuns", "SpaceMelee", "SentinelWeapons", "Hoverboards",
];

fn derive_masterable(it: &mut ExportItem) {
    let modular = it.unique_name.contains("/Modular")
        || it.unique_name.contains("/Zaw/")
        || it.unique_name.contains("/Kitgun/")
        || it.unique_name.contains("/Tips/")
        || it.unique_name.contains("/Handles/")
        || it.unique_name.contains("/Barrel/")
        || it.unique_name.contains("/Grip/");
    it.masterable = !it.exclude_from_codex
        && !modular
        && MASTERABLE_CATEGORIES.contains(&it.product_category.as_str());
}

impl Export {
    pub fn cache_path() -> PathBuf {
        crate::data_dir().join("export.json")
    }

    pub fn load_cached() -> Option<Export> {
        let s = std::fs::read_to_string(Self::cache_path()).ok()?;
        let ex: Export = serde_json::from_str(&s).ok()?;
        (ex.cache_version == CACHE_VERSION).then_some(ex)
    }

    pub async fn fetch(client: &reqwest::Client) -> Result<Export> {
        let raw = client.get(INDEX_URL).send().await?.bytes().await?;
        let mut out = Vec::new();
        lzma_rs::lzma_decompress(&mut std::io::Cursor::new(raw), &mut out)?;
        let index = String::from_utf8_lossy(&out).to_string();
        let index_hash = {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            index.hash(&mut h);
            format!("{:x}", h.finish())
        };

        if let Some(cached) = Self::load_cached() {
            if cached.index_hash == index_hash {
                return Ok(cached);
            }
        }

        let mut files: HashMap<&str, &str> = HashMap::new();
        for line in index.lines() {
            let line = line.trim();
            if let Some(name) = line.split('!').next() {
                // "ExportWeapons_en.json" and "ExportManifest.json" both map to the bare key.
                let key = name.trim_end_matches(".json").trim_end_matches("_en");
                files.insert(key, line);
            }
        }

        async fn get<T: for<'a> Deserialize<'a>>(
            client: &reqwest::Client,
            files: &HashMap<&str, &str>,
            key: &str,
            root: &str,
        ) -> Result<Vec<T>> {
            let file = files.get(key).with_context(|| format!("{key} missing in index"))?;
            let txt = client
                .get(format!("{MANIFEST_URL}{file}"))
                .send()
                .await?
                .text()
                .await?;
            // DE ships raw CR/LF inside JSON strings.
            let txt = txt.replace(['\r', '\n'], "");
            let mut v: serde_json::Value = serde_json::from_str(&txt)?;
            let arr = v
                .get_mut(root)
                .and_then(|a| a.as_array_mut())
                .map(std::mem::take)
                .with_context(|| format!("{root} missing"))?;
            Ok(arr
                .into_iter()
                .filter_map(|x| serde_json::from_value(x).ok())
                .collect())
        }

        let mut warframes: Vec<ExportItem> = get(client, &files, "ExportWarframes", "ExportWarframes").await?;
        let mut weapons: Vec<ExportItem> = get(client, &files, "ExportWeapons", "ExportWeapons").await?;
        let mut sentinels: Vec<ExportItem> = get(client, &files, "ExportSentinels", "ExportSentinels").await?;
        let relics: Vec<ExportRelic> =
            get(client, &files, "ExportRelicArcane", "ExportRelicArcane").await?;
        let manifest: Vec<Manifest> =
            get(client, &files, "ExportManifest", "Manifest").await.unwrap_or_default();
        let resources: Vec<Resource> =
            get(client, &files, "ExportResources", "ExportResources").await.unwrap_or_default();
        let recipes: Vec<Recipe> =
            get(client, &files, "ExportRecipes", "ExportRecipes").await.unwrap_or_default();
        let upgrades: Vec<Resource> =
            get(client, &files, "ExportUpgrades", "ExportUpgrades").await.unwrap_or_default();
        // Relic names before the " Relic" trim, for inventory display.
        let relic_names: Vec<(String, String)> =
            relics.iter().map(|r| (r.unique_name.clone(), r.name.clone())).collect();

        for it in warframes.iter_mut().chain(weapons.iter_mut()).chain(sentinels.iter_mut()) {
            // "<ARCHWING> Odonata" style prefixes are UI markup, not part of the name.
            if let Some(end) = it.name.find('>') {
                if it.name.starts_with('<') {
                    it.name = it.name[end + 1..].trim_start().to_string();
                }
            }
            derive_masterable(it);
        }

        // One entry per refinement (Bronze/Silver/Gold/Platinum suffix) with the
        // same display name; the drop table is identical, so keep one.
        let mut seen_relics = std::collections::HashSet::new();
        let relics: Vec<ExportRelic> = relics
            .into_iter()
            .filter(|r| {
                !r.relic_rewards.is_empty()
                    && r.name.ends_with(" Relic")
                    && ["Lith", "Meso", "Neo", "Axi", "Requiem"].iter().any(|t| r.name.starts_with(t))
            })
            .map(|mut r| {
                r.name = r.name.trim_end_matches(" Relic").to_string();
                r
            })
            .filter(|r| seen_relics.insert(r.name.clone()))
            .collect();

        // One name table for every path we may meet (relic rewards, inventory).
        // A recipe's resultType names what it builds -> "<name> Blueprint".
        let mut item_names: HashMap<String, String> = HashMap::new();
        for r in resources.iter().chain(upgrades.iter()) {
            item_names.insert(r.unique_name.clone(), r.name.clone());
        }
        for (u, n) in relic_names {
            item_names.insert(u, n);
        }
        for it in warframes.iter().chain(weapons.iter()).chain(sentinels.iter()) {
            item_names.insert(it.unique_name.clone(), it.name.clone());
        }
        for r in &recipes {
            if item_names.contains_key(&r.unique_name) {
                continue;
            }
            if let Some(n) = item_names.get(&r.result_type) {
                let n = if n.ends_with("Blueprint") { n.clone() } else { format!("{n} Blueprint") };
                item_names.insert(r.unique_name.clone(), n);
            }
        }

        let images = manifest
            .into_iter()
            .map(|m| (m.unique_name, format!("{IMAGE_URL}{}", m.texture_location.trim_start_matches('/'))))
            .collect();

        let ex = Export {
            cache_version: CACHE_VERSION,
            index_hash,
            warframes,
            weapons,
            sentinels,
            relics,
            images,
            item_names,
            recipes: recipes
                .into_iter()
                .filter(|r| !r.ingredients.is_empty())
                .collect(),
        };
        std::fs::write(Self::cache_path(), serde_json::to_string(&ex)?)?;
        Ok(ex)
    }

    pub fn all_masterable(&self) -> impl Iterator<Item = &ExportItem> {
        self.warframes
            .iter()
            .chain(self.weapons.iter())
            .chain(self.sentinels.iter())
            .filter(|i| i.masterable)
    }

    /// Display name for any /Lotus path; store paths (/StoreItems) map to the
    /// same item. Falls back to a camel-case split of the last segment.
    pub fn item_name(&self, path: &str) -> String {
        let plain = path.replacen("/StoreItems", "", 1);
        self.item_names
            .get(plain.as_str())
            .or_else(|| self.item_names.get(path))
            .cloned()
            .unwrap_or_else(|| crate::relic::reward_display(path))
    }

    pub fn reward_name(&self, path: &str) -> String {
        self.item_name(path)
    }

    pub fn find_by_unique(&self, unique: &str) -> Option<&ExportItem> {
        self.warframes
            .iter()
            .chain(self.weapons.iter())
            .chain(self.sentinels.iter())
            .find(|i| i.unique_name == unique)
    }
}
