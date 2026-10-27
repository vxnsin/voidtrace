import { emit } from "@tauri-apps/api/event";
import type { Component } from "vue";
import { api } from "../api";
import CyclesWidget from "./CyclesWidget.vue";
import FissuresWidget from "./FissuresWidget.vue";
import RewardsWidget from "./RewardsWidget.vue";
import SessionWidget from "./SessionWidget.vue";
import BaroWidget from "./BaroWidget.vue";
import NoticesWidget from "./NoticesWidget.vue";

export type WidgetId = "cycles" | "fissures" | "rewards" | "session" | "baro" | "notices";
export type FissureFilter = { tiers: string[]; missions: string[]; steelPath: "all" | "only" | "hide"; storms: boolean };

/** One widget's settings, stored as JSON under the `widgets` setting.
 *  Position: `anchor` 0..8 is a 3x3 grid (0 top-left … 8 bottom-right); `x`/`y`
 *  are the distance from that anchor in % of the screen, so layouts survive
 *  resolution changes. */
export type WidgetCfg = {
  id: WidgetId;
  pinned: boolean;
  dashboard: boolean;
  anchor: number;
  x: number;
  y: number;
  scale: number;
  opacity: number;
  size: "compact" | "full";
  show: "always" | "mission" | "orbiter";
  filter?: FissureFilter;
};

export const CATEGORIES = ["World", "Relics", "You"] as const;

export const WIDGETS: { id: WidgetId; cat: (typeof CATEGORIES)[number]; name: string; desc: string; c: Component }[] = [
  { id: "cycles", cat: "World", name: "World cycles", desc: "Cetus · Vallis · Cambion · Zariman · Duviri · Earth", c: CyclesWidget },
  { id: "fissures", cat: "World", name: "Fissures", desc: "Active void fissures, filtered your way", c: FissuresWidget },
  { id: "baro", cat: "World", name: "Baro Ki'Teer", desc: "Countdown, relay and his offer with plat values", c: BaroWidget },
  { id: "rewards", cat: "Relics", name: "Relic rewards", desc: "Plat and ducats per card; shows itself on the reward screen", c: RewardsWidget },
  { id: "session", cat: "You", name: "Session", desc: "Plat, ducats, relics and trades since the game started", c: SessionWidget },
  { id: "notices", cat: "You", name: "Notifications", desc: "Cycles, fissures, undercuts, Baro, trades: a line when it happens", c: NoticesWidget },
];

const DEFAULTS: WidgetCfg[] = [
  { id: "cycles", pinned: false, dashboard: true, anchor: 0, x: 1.5, y: 12, scale: 1, opacity: 0.88, size: "full", show: "always" },
  { id: "fissures", pinned: false, dashboard: true, anchor: 2, x: 1.5, y: 12, scale: 1, opacity: 0.88, size: "full", show: "orbiter",
    filter: { tiers: [], missions: [], steelPath: "all", storms: true } },
  { id: "rewards", pinned: true, dashboard: false, anchor: 1, x: 0, y: 8, scale: 1, opacity: 0.92, size: "full", show: "always" },
  { id: "session", pinned: false, dashboard: true, anchor: 6, x: 1.5, y: 6, scale: 1, opacity: 0.88, size: "full", show: "always" },
  { id: "baro", pinned: false, dashboard: true, anchor: 7, x: 0, y: 6, scale: 1, opacity: 0.88, size: "compact", show: "orbiter" },
  { id: "notices", pinned: true, dashboard: false, anchor: 1, x: 0, y: 1.5, scale: 1, opacity: 0.92, size: "full", show: "always" },
];

export async function loadWidgets(): Promise<WidgetCfg[]> {
  let saved: Partial<WidgetCfg>[] = [];
  try { saved = JSON.parse((await api.getSetting("widgets")) ?? "[]"); } catch {}
  return DEFAULTS.map((d) => ({ ...d, ...(saved.find((s) => s.id === d.id) ?? {}) }));
}

/** Save and tell the other window (overlay ↔ main) to reload. */
export async function saveWidgets(list: WidgetCfg[]) {
  await api.setSetting("widgets", JSON.stringify(list));
  await emit("widgets-changed");
}

/** CSS for a widget placed by anchor + offset, scaled from the anchored edge. */
export function placeStyle(w: WidgetCfg): Record<string, string> {
  const col = w.anchor % 3, row = Math.floor(w.anchor / 3);
  const s: Record<string, string> = {};
  const tx = col === 1 ? "-50%" : "0", ty = row === 1 ? "-50%" : "0";
  if (col === 0) s.left = `${w.x}%`; else if (col === 1) s.left = `calc(50% + ${w.x}%)`; else s.right = `${w.x}%`;
  if (row === 0) s.top = `${w.y}%`; else if (row === 1) s.top = `calc(50% + ${w.y}%)`; else s.bottom = `${w.y}%`;
  s.transform = `translate(${tx}, ${ty}) scale(${w.scale})`;
  s.transformOrigin = `${["left", "center", "right"][col]} ${["top", "center", "bottom"][row]}`;
  return s;
}

/** Anchor + offsets from where a widget was dropped (rects in the same frame). */
export function placeFromRect(r: DOMRect, layer: DOMRect): { anchor: number; x: number; y: number } {
  const W = layer.width, H = layer.height;
  const cx = r.left + r.width / 2 - layer.left, cy = r.top + r.height / 2 - layer.top;
  const col = cx < W / 3 ? 0 : cx > (2 * W) / 3 ? 2 : 1;
  const row = cy < H / 3 ? 0 : cy > (2 * H) / 3 ? 2 : 1;
  const left = r.left - layer.left, top = r.top - layer.top;
  const x = col === 0 ? left / W : col === 1 ? (cx - W / 2) / W : (W - (left + r.width)) / W;
  const y = row === 0 ? top / H : row === 1 ? (cy - H / 2) / H : (H - (top + r.height)) / H;
  // Edge anchors measure inwards, so a negative offset would be off screen.
  const round = (v: number, centred: boolean) => Math.round((centred ? v : Math.max(0, v)) * 1000) / 10;
  return { anchor: row * 3 + col, x: round(x, col === 1), y: round(y, row === 1) };
}
