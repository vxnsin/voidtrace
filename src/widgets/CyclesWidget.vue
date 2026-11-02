<script setup lang="ts">
import { computed } from "vue";
import type { Cycle } from "../api";
import { left, live, progress } from "./live";
import type { WidgetCfg } from "./registry";
import WidgetFrame from "./WidgetFrame.vue";

const props = defineProps<{ cfg: WidgetCfg }>();
const ROWS: [string, keyof NonNullable<typeof live.world>][] = [
  ["Cetus", "cetusCycle"], ["Orb Vallis", "vallisCycle"], ["Cambion", "cambionCycle"],
  ["Zariman", "zarimanCycle"], ["Duviri", "duviriCycle"], ["Earth", "earthCycle"],
];
const COLOR: Record<string, string> = {
  day: "#d8c27a", night: "#7b9fd6", warm: "#e09060", cold: "#8ec6e0", fass: "#c9a35b", vome: "#7fb0c9",
  grineer: "var(--accent)", corpus: "#6fa8dc", joy: "#d8c27a", anger: "#e06040", envy: "#7fc97f", sorrow: "#7b9fd6", fear: "#a080c0",
};
// Phase lengths (s) in order, to roll a cycle forward locally when it expires
// before the next world-state fetch.
const PHASES: Record<string, [string, number][]> = {
  cetusCycle: [["day", 6000], ["night", 3000]],
  cambionCycle: [["fass", 6000], ["vome", 3000]],
  vallisCycle: [["warm", 400], ["cold", 1200]],
  zarimanCycle: [["grineer", 9000], ["corpus", 9000]],
  earthCycle: [["day", 14400], ["night", 14400]],
  duviriCycle: [["sorrow", 7200], ["fear", 7200], ["joy", 7200], ["anger", 7200], ["envy", 7200]],
};
function roll(key: string, c: Cycle | undefined): Cycle | undefined {
  const ph = PHASES[key];
  if (!c || !ph) return c;
  let state = c.state, a = Date.parse(c.activation), e = Date.parse(c.expiry);
  for (let i = 0; e <= live.now && i < 50; i++) {
    const [next, len] = ph[(ph.findIndex((p) => p[0] === state) + 1) % ph.length];
    state = next; a = e; e += len * 1000;
  }
  return { state, activation: new Date(a).toISOString(), expiry: new Date(e).toISOString() };
}

const rows = computed(() =>
  ROWS.slice(0, props.cfg.size === "compact" ? 3 : 6).map(([name, key]) => {
    const c = roll(key, live.world?.[key] as Cycle | undefined);
    return { name, state: c?.state ?? "…", left: left(c?.expiry), p: progress(c?.activation, c?.expiry) };
  }),
);
</script>

<template>
  <WidgetFrame title="World cycles" :meta="live.world ? 'live' : 'loading'">
    <div class="cyc">
      <template v-for="r in rows" :key="r.name">
        <span class="dim">{{ r.name }}</span>
        <span class="s"><b :style="{ color: COLOR[r.state] ?? 'var(--accent2)' }">{{ r.state }}</b> · {{ r.left }}</span>
        <div class="bar"><i :style="{ width: r.p * 100 + '%' }"></i></div>
      </template>
    </div>
  </WidgetFrame>
</template>

<style scoped>
.cyc { display: grid; grid-template-columns: 1fr auto; gap: 1px 16px; align-items: center; }
.s { color: var(--accent2); text-align: right; }
.s b { font-weight: 500; text-transform: capitalize; }
.bar { grid-column: 1 / -1; height: 2px; background: var(--border); margin-bottom: 4px; }
.bar i { display: block; height: 100%; background: var(--accent2); }
</style>
