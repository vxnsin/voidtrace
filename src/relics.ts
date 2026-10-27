import { invoke } from "@tauri-apps/api/core";
import type { RelicPlan } from "./api";

export type RelicRow = { name: string; owned: [number, number, number, number]; vaulted: boolean; plan: RelicPlan; missing: number };
export type RelicOverview = { rows: RelicRow[]; synced: boolean; pending: number };
export const relicOverview = (onlyOwned: boolean) => invoke<RelicOverview>("relic_overview", { onlyOwned });

/** Void traces to refine an intact relic to each refinement. */
export const TRACES = [0, 25, 50, 100];

/**
 * Expected value of one crack when `squad` players open the same relic at
 * refinement `r` and you take the best of the rewards shown (radshare).
 * With the drops sorted by value, P(best <= v_k) = F(k)^n.
 */
export function squadValue(plan: RelicPlan, r: number, squad: number, value: (d: RelicPlan["drops"][number]) => number): number {
  const drops = [...plan.drops].sort((a, b) => value(a) - value(b));
  let cdf = 0, prev = 0, ev = 0;
  for (const d of drops) {
    cdf += d.chance[r] / 100;
    const now = Math.pow(Math.min(1, cdf), squad);
    ev += value(d) * (now - prev);
    prev = now;
  }
  return ev;
}

/** Chance that at least one of `squad` rolls is this drop. */
export const squadChance = (p: number, squad: number) => 1 - Math.pow(1 - p / 100, squad);
