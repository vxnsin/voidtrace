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
  grineer: "#c8102e", corpus: "#6fa8dc", joy: "#d8c27a", anger: "#e06040", envy: "#7fc97f", sorrow: "#7b9fd6", fear: "#a080c0",
};
const rows = computed(() =>
  ROWS.slice(0, props.cfg.size === "compact" ? 3 : 6).map(([name, key]) => {
    const c = live.world?.[key] as Cycle | undefined;
    return { name, state: c?.state ?? "…", left: left(c?.expiry), p: progress(c?.activation, c?.expiry) };
  }),
);
</script>

<template>
  <WidgetFrame title="World cycles" :meta="live.world ? 'live' : 'loading'">
    <div class="cyc">
      <template v-for="r in rows" :key="r.name">
        <span class="dim">{{ r.name }}</span>
        <span class="s"><b :style="{ color: COLOR[r.state] ?? '#e8e8e8' }">{{ r.state }}</b> · {{ r.left }}</span>
        <div class="bar"><i :style="{ width: r.p * 100 + '%' }"></i></div>
      </template>
    </div>
  </WidgetFrame>
</template>

<style scoped>
.cyc { display: grid; grid-template-columns: 1fr auto; gap: 1px 16px; align-items: center; }
.s { color: #e8e8e8; text-align: right; }
.s b { font-weight: 500; text-transform: capitalize; }
.bar { grid-column: 1 / -1; height: 2px; background: #1d1d21; margin-bottom: 4px; }
.bar i { display: block; height: 100%; background: #e8e8e8; }
</style>
