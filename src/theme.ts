import { emit, listen } from "@tauri-apps/api/event";
import { reactive } from "vue";
import { api } from "./api";

/** A theme is a handful of base colours; the in-between greys (panels,
 *  borders, muted text) are mixed from background and text so custom themes
 *  stay consistent. Presets may pin exact values in `tokens`. */
export type Theme = {
  name: string;
  bg: string;
  fg: string;
  /** Bright text: headings, values. */
  hi: string;
  accent: string;
  warn: string;
  ok: string;
  /** Overlay widget background and its default opacity. */
  widget: string;
  widgetOpacity: number;
  /** Item pictures in greyscale (the Void look). */
  mono: boolean;
  light?: boolean;
  tokens?: Record<string, string>;
};

export const PRESETS: Theme[] = [
  {
    name: "Void", bg: "#070708", fg: "#b4b4b4", hi: "#e8e8e8", accent: "#c8102e", warn: "#b89a5a", ok: "#9aa89a",
    widget: "#070708", widgetOpacity: 0.88, mono: true,
    tokens: { bg2: "#0b0b0d", bg3: "#111114", fg2: "#8a8a8a", muted: "#4e4e52", dim: "#2a2a2e", border: "#1c1c20" },
  },
  {
    name: "Lua", bg: "#efede8", fg: "#3c3b38", hi: "#121110", accent: "#b00d28", warn: "#8a6420", ok: "#3f6b45",
    widget: "#f4f2ee", widgetOpacity: 0.92, mono: false, light: true,
  },
  {
    name: "Orokin", bg: "#0b0906", fg: "#c4b896", hi: "#f2e4bc", accent: "#d4af37", warn: "#e0a84a", ok: "#a3b07e",
    widget: "#0b0906", widgetOpacity: 0.9, mono: false,
  },
  {
    name: "Corpus", bg: "#05090f", fg: "#a3b3c6", hi: "#e4eefa", accent: "#3d8bd9", warn: "#c9a35b", ok: "#6fc69a",
    widget: "#05090f", widgetOpacity: 0.88, mono: false,
  },
  {
    name: "High contrast", bg: "#000000", fg: "#f2f2f2", hi: "#ffffff", accent: "#ffd400", warn: "#ffae00", ok: "#4cff7a",
    widget: "#000000", widgetOpacity: 0.96, mono: false, tokens: { muted: "#9a9a9a", dim: "#5a5a5a", border: "#3a3a3a" },
  },
];

export const theme = reactive<{ current: Theme }>({ current: PRESETS[0] });

function rgb(hex: string): [number, number, number] {
  let h = hex.replace("#", "");
  if (h.length === 3) h = [...h].map((c) => c + c).join("");
  const n = parseInt(h.slice(0, 6), 16) || 0;
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}
const hex = (c: number[]) => "#" + c.map((v) => Math.round(v).toString(16).padStart(2, "0")).join("");
/** `a` moved `t` of the way towards `b`. */
export const mix = (a: string, b: string, t: number) => {
  const x = rgb(a), y = rgb(b);
  return hex(x.map((v, i) => v + (y[i] - v) * t));
};
const luma = (c: string) => { const [r, g, b] = rgb(c); return (0.2126 * r + 0.7152 * g + 0.0722 * b) / 255; };

/** Every CSS variable a theme sets. */
export function tokens(t: Theme): Record<string, string> {
  const m = (k: number) => mix(t.bg, t.fg, k);
  return {
    bg: t.bg, bg2: m(0.03), bg3: m(0.06), border: m(0.1), dim: m(0.18), muted: m(0.38), fg2: m(0.75),
    fg: t.fg, accent: t.accent, accent2: t.hi, ok: t.ok, warn: t.warn, bad: t.accent,
    "on-accent": luma(t.accent) > 0.55 ? "#000000" : t.light ? "#ffffff" : "#000000",
    wbg: t.widget, two: String(t.widgetOpacity), "img-filter": t.mono ? "grayscale(1) contrast(1.1)" : "none",
    ...(t.tokens ?? {}),
  };
}

export function applyTheme(t: Theme) {
  theme.current = t;
  const s = document.documentElement.style;
  for (const [k, v] of Object.entries(tokens(t))) s.setProperty(`--${k}`, v);
  s.setProperty("color-scheme", t.light ? "light" : "dark");
}

/** Read the saved theme, apply it and follow changes from the other window. */
export async function initTheme() {
  try {
    const saved = await api.getSetting("theme");
    if (saved) applyTheme(parseTheme(saved));
  } catch {}
  listen<Theme>("theme-changed", (e) => applyTheme(e.payload));
}

export async function saveTheme(t: Theme) {
  applyTheme(t);
  await api.setSetting("theme", JSON.stringify(t));
  await emit("theme-changed", t);
}

/** Shareable JSON; `tokens` are left out so it stays short. */
export function exportTheme(t: Theme) {
  const { tokens: _, ...rest } = t;
  return JSON.stringify({ voidtrace_theme: 1, ...rest });
}

/** Accepts the export format (or a saved theme); throws on anything else. */
export function parseTheme(json: string): Theme {
  const o = JSON.parse(json);
  if (!o || typeof o !== "object" || typeof o.bg !== "string" || typeof o.accent !== "string") throw new Error("not a VoidTrace theme");
  const base = PRESETS.find((p) => p.name === o.name) ?? PRESETS[0];
  // Always 6-digit hex: the canvases append an alpha byte.
  const color = (k: keyof Theme) => mix(typeof o[k] === "string" && /^#[0-9a-f]{3}([0-9a-f]{3})?$/i.test(o[k]) ? o[k] : (base[k] as string), "#000", 0);
  return {
    name: typeof o.name === "string" ? o.name.slice(0, 40) : "Custom",
    bg: color("bg"), fg: color("fg"), hi: color("hi"), accent: color("accent"), warn: color("warn"), ok: color("ok"), widget: color("widget"),
    widgetOpacity: Math.min(1, Math.max(0.3, Number(o.widgetOpacity) || base.widgetOpacity)),
    mono: !!o.mono, light: !!o.light,
    // Only presets carry pinned tokens; a shared theme is mixed from its colours.
    tokens: PRESETS.find((p) => p.name === o.name && p.bg === o.bg && p.accent === o.accent)?.tokens,
  };
}
