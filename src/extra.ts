import { invoke } from "@tauri-apps/api/core";
import type { RelicHit } from "./api";

// Foundry, sets, mastery routes and the riven market (v0.4).
export type Part = { path: string; name: string; need: number; have: number; blueprint: string | null; bp_have: number; slug: string | null; vaulted: boolean };
export type FoundryRow = {
  unique_name: string; name: string; category: string; image: string | null; mastery_req: number;
  blueprint: string | null; bp_have: number; bp_slug: string | null; parts: Part[]; credits: number; ready: boolean; vaulted: boolean;
  mastered: boolean; xp: number;
};
export type FoundryOverview = { rows: FoundryRow[]; synced: boolean; mastery_rank: number };
export type PartSource = { name: string; slug: string | null; plat: number | null; relics: RelicHit[] };
export type SetPart = { slug: string; name: string; need: number; have: number };
export type SetInfo = { slug: string; name: string; parts: SetPart[]; complete: number };
export type RoutePart = { name: string; chance: number; have: boolean };
export type RelicRoute = { unique_name: string; name: string; parts: RoutePart[]; chance_all: number };
export type PlatRoute = { unique_name: string; name: string; category: string; image: string | null; missing: { name: string; slug: string; plat: number | null }[]; total: number | null };
export type MasteryRoutes = { relics: RelicRoute[]; plat: PlatRoute[]; intrinsics: { name: string; rank: number }[]; synced: boolean; pending: number };
export type RivenWeapon = { slug: string; name: string; riven_type: string; disposition: number; icon: string };
export type RivenAttr = { url_name: string; value: number; positive: boolean };
export type RivenAuction = { id: string; name: string; buyout: number | null; starting: number; top_bid: number | null; direct: boolean; rerolls: number; mastery: number; polarity: string; attributes: RivenAttr[]; owner: string; owner_status: string; created: string };
export type RivenPrices = { count: number; median: number; trimmed_avg: number; low: number; high: number; attrs: { url_name: string; count: number; median: number; ratio: number }[] };
export type WeeklyRiven = { itemType: string; compatibility: string | null; rerolled: boolean; avg: number; stddev: number; min: number; max: number; pop: number; median: number };
export type RivenSearch = { auctions: RivenAuction[]; prices: RivenPrices; weekly: WeeklyRiven[] };
export type RivenSnipe = { weapon: string; name: string; positives: string[]; negative?: string | null; max: number };

export const extra = {
  foundry: () => invoke<FoundryOverview>("foundry_overview"),
  sources: (uniqueName: string) => invoke<PartSource[]>("foundry_sources", { uniqueName }),
  sets: () => invoke<SetInfo[]>("inventory_sets"),
  mastery: () => invoke<MasteryRoutes>("mastery_routes"),
  rivenWeapons: () => invoke<RivenWeapon[]>("riven_weapons"),
  rivenSearch: (weapon: string, name: string) => invoke<RivenSearch>("riven_search", { weapon, name }),
  snipes: () => invoke<RivenSnipe[]>("riven_snipes"),
  setSnipes: (snipes: RivenSnipe[]) => invoke<void>("set_riven_snipes", { snipes }),
};