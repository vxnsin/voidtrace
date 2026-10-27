<script setup lang="ts">
import { computed, ref } from "vue";
import { PRESETS, applyTheme, exportTheme, parseTheme, saveTheme, theme, type Theme } from "../theme";

// Theme picker: presets, a few colours to make it your own, and sharing as JSON.
const t = computed(() => theme.current);
const shareMsg = ref("");
const importText = ref("");
const importOpen = ref(false);

let timer: number | undefined;
function set(patch: Partial<Theme>) {
  const next: Theme = { ...t.value, ...patch, tokens: undefined };
  if (!next.name.endsWith("(custom)") && next.name !== "Custom") next.name = `${t.value.name.replace(/ \(custom\)$/, "")} (custom)`;
  if (patch.bg) next.light = lum(patch.bg) > 0.5;
  // Colour pickers fire on every move: apply at once, store when it settles.
  clearTimeout(timer);
  applyTheme(next);
  timer = window.setTimeout(() => saveTheme(next), 250);
}
function pick(p: Theme) { clearTimeout(timer); saveTheme(p); }
const lum = (c: string) => { const n = parseInt(c.slice(1), 16); return (0.2126 * (n >> 16) + 0.7152 * ((n >> 8) & 255) + 0.0722 * (n & 255)) / 255; };
const val = (e: Event) => (e.target as HTMLInputElement).value;

const COLOURS: [keyof Theme, string][] = [
  ["accent", "Accent"], ["bg", "Background"], ["fg", "Text"], ["hi", "Bright text"], ["widget", "Widget background"],
];

async function copy() {
  try { await navigator.clipboard.writeText(exportTheme(t.value)); shareMsg.value = "Copied. Paste it anywhere to share."; }
  catch { shareMsg.value = "Clipboard not available."; }
}
function doImport() {
  try { pick(parseTheme(importText.value)); importOpen.value = false; importText.value = ""; shareMsg.value = "Theme applied."; }
  catch (e) { shareMsg.value = `That doesn't look like a theme (${e instanceof Error ? e.message : e}).`; }
}
</script>

<template>
  <div class="card">
    <h2 style="margin-top:0">Theme</h2>
    <div class="presets">
      <button v-for="p in PRESETS" :key="p.name" class="preset" :class="{ on: t.name === p.name }" @click="pick(p)">
        <span class="sw" :style="{ background: p.bg, borderColor: p.accent }"><i :style="{ background: p.accent }"></i><i :style="{ background: p.fg }"></i></span>
        {{ p.name }}
      </button>
    </div>
    <div class="custom">
      <label v-for="[k, l] in COLOURS" :key="k" class="col">
        <input type="color" :value="t[k] as string" @input="set({ [k]: val($event) } as Partial<Theme>)" />
        <span>{{ l }}</span>
      </label>
    </div>
    <div class="row" style="margin-top:10px">
      <label class="rng"><span class="muted">Widget opacity</span>
        <input type="range" min="0.3" max="1" step="0.02" :value="t.widgetOpacity" @input="set({ widgetOpacity: Number(val($event)) })" />
        <span>{{ Math.round(t.widgetOpacity * 100) }}%</span>
      </label>
      <label class="row"><input type="checkbox" :checked="t.mono" @change="set({ mono: !t.mono })" /> Item pictures in greyscale</label>
    </div>
    <div class="row" style="margin-top:10px">
      <span class="muted">{{ t.name }}</span>
      <button class="btn" @click="copy">Copy theme</button>
      <button class="btn" @click="importOpen = !importOpen">Import…</button>
      <span class="muted">{{ shareMsg }}</span>
    </div>
    <div v-if="importOpen" style="margin-top:8px">
      <textarea v-model="importText" rows="3" placeholder='{"voidtrace_theme":1, "name": "…", "accent": "#…", …}' class="imp"></textarea>
      <button class="btn primary" style="margin-top:6px" @click="doImport">Apply</button>
    </div>
    <p class="muted" style="margin:8px 0 0">Applies to the app and the in-game widgets right away.</p>
  </div>
</template>

<style scoped>
.presets { display: flex; flex-wrap: wrap; gap: 8px; }
.preset { all: unset; cursor: pointer; display: flex; align-items: center; gap: 8px; padding: 5px 10px 5px 6px; border: 1px solid var(--dim); color: var(--fg2); font-size: 11px; letter-spacing: .06em; text-transform: uppercase; }
.preset:hover { color: var(--accent2); border-color: var(--fg2); }
.preset.on { border-color: var(--accent); color: var(--accent2); }
.sw { display: flex; gap: 3px; padding: 3px; border: 1px solid; }
.sw i { width: 8px; height: 12px; display: block; }
.custom { display: flex; flex-wrap: wrap; gap: 14px; margin-top: 12px; }
.col { display: flex; align-items: center; gap: 6px; color: var(--fg2); cursor: pointer; }
.col input { width: 26px; height: 22px; padding: 0; border: 1px solid var(--dim); background: none; cursor: pointer; }
.rng { display: flex; align-items: center; gap: 8px; }
.rng input { width: 140px; accent-color: var(--accent); }
.imp { width: 100%; background: var(--bg3); border: 1px solid var(--border); color: inherit; padding: 8px; font: inherit; resize: vertical; }
</style>
