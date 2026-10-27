<script setup lang="ts">
import { computed } from "vue";
import { left, live } from "./live";
import type { WidgetCfg } from "./registry";
import WidgetFrame from "./WidgetFrame.vue";

const props = defineProps<{ cfg: WidgetCfg }>();
const list = computed(() => {
  const f = props.cfg.filter ?? { tiers: [], missions: [], steelPath: "all", storms: true };
  return (live.world?.fissures ?? [])
    .filter((x) => Date.parse(x.expiry) > live.now)
    .filter((x) => !f.tiers.length || f.tiers.includes(x.tier))
    .filter((x) => !f.missions.length || f.missions.includes(x.missionType))
    .filter((x) => (f.steelPath === "only" ? x.isHard : f.steelPath === "hide" ? !x.isHard : true))
    .filter((x) => f.storms || !x.isStorm)
    .sort((a, b) => a.tierNum - b.tierNum || Date.parse(a.expiry) - Date.parse(b.expiry))
    .slice(0, props.cfg.size === "compact" ? 4 : 10);
});
const meta = computed(() => {
  const f = props.cfg.filter;
  const parts = [...(f?.tiers ?? []), ...(f?.missions ?? [])];
  return parts.length ? parts.join(" · ").toLowerCase() : "all";
});
</script>

<template>
  <WidgetFrame title="Fissures" :meta="meta">
    <div v-if="list.length" class="fis">
      <template v-for="x in list" :key="x.id">
        <span :class="x.tierNum >= 4 ? 'red' : 'hi'">{{ x.tier }}</span>
        <span>{{ x.missionType }} <span class="dim">{{ x.node }}</span><span v-if="x.isHard" class="gold"> SP</span><span v-if="x.isStorm" class="gold"> storm</span></span>
        <span class="dim">{{ left(x.expiry) }}</span>
      </template>
    </div>
    <div v-else class="dim">{{ live.world ? "nothing matches the filter" : "loading…" }}</div>
  </WidgetFrame>
</template>

<style scoped>
.fis { display: grid; grid-template-columns: auto 1fr auto; gap: 1px 10px; }
.fis > :nth-child(3n) { text-align: right; }
</style>
