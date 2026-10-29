<script setup lang="ts">
import { computed } from "vue";
import { live } from "./live";
import type { FissureFilter, WidgetCfg } from "./registry";

// Settings of one widget; used in the in-game editor panel and the Widgets tab.
const props = defineProps<{ cfg: WidgetCfg; compact?: boolean }>();
const emit = defineEmits<{ update: [patch: Partial<WidgetCfg>] }>();
const TIERS = ["Lith", "Meso", "Neo", "Axi", "Requiem", "Omnia"];
const missions = computed(() => [...new Set((live.world?.fissures ?? []).map((f) => f.missionType))].sort());
const f = computed<FissureFilter>(() => props.cfg.filter ?? { tiers: [], missions: [], steelPath: "all", storms: true });
const val = (e: Event) => (e.target as HTMLInputElement).value;
function toggle(key: "tiers" | "missions", v: string) {
  const list = f.value[key].includes(v) ? f.value[key].filter((x) => x !== v) : [...f.value[key], v];
  emit("update", { filter: { ...f.value, [key]: list } });
}
</script>

<template>
  <div class="ws">
    <div class="seg">
      <span class="lbl">Size</span>
      <button v-for="s in (['compact', 'full'] as const)" :key="s" :class="{ on: cfg.size === s }" @click="emit('update', { size: s })">{{ s }}</button>
    </div>
    <div v-if="cfg.id !== 'rewards'" class="seg">
      <span class="lbl">Show</span>
      <button v-for="[k, l] in ([['always', 'always'], ['mission', 'missions'], ['orbiter', 'orbiter']] as const)" :key="k" :class="{ on: cfg.show === k }" @click="emit('update', { show: k })">{{ l }}</button>
    </div>
    <template v-if="!compact">
      <label class="rng"><span class="lbl">Opacity</span><input type="range" min="0.4" max="1" step="0.02" :value="cfg.opacity" @input="emit('update', { opacity: Number(val($event)) })" /><span>{{ Math.round(cfg.opacity * 100) }}%</span></label>
      <label class="rng"><span class="lbl">Scale</span><input type="range" min="0.7" max="1.6" step="0.05" :value="cfg.scale" @input="emit('update', { scale: Number(val($event)) })" /><span>{{ Math.round(cfg.scale * 100) }}%</span></label>
    </template>
    <div v-if="cfg.id === 'relics'" class="seg">
      <span class="lbl">Rank by</span>
      <button v-for="s in (['plat', 'ducats'] as const)" :key="s" :class="{ on: (cfg.relicSort ?? 'plat') === s }" @click="emit('update', { relicSort: s })">{{ s }}</button>
    </div>
    <template v-if="cfg.id === 'fissures'">
      <div class="chips"><span class="lbl">Tiers</span><span v-for="t in TIERS" :key="t" class="chip" :class="{ on: f.tiers.includes(t) }" @click="toggle('tiers', t)">{{ t }}</span></div>
      <div class="chips"><span class="lbl">Missions</span><span v-for="t in missions" :key="t" class="chip" :class="{ on: f.missions.includes(t) }" @click="toggle('missions', t)">{{ t }}</span></div>
      <div class="seg">
        <span class="lbl">Steel Path</span>
        <button v-for="[k, l] in ([['all', 'include'], ['only', 'only'], ['hide', 'hide']] as const)" :key="k" :class="{ on: f.steelPath === k }" @click="emit('update', { filter: { ...f, steelPath: k } })">{{ l }}</button>
      </div>
      <div class="seg">
        <span class="lbl">Void storms</span>
        <button :class="{ on: f.storms }" @click="emit('update', { filter: { ...f, storms: true } })">show</button>
        <button :class="{ on: !f.storms }" @click="emit('update', { filter: { ...f, storms: false } })">hide</button>
      </div>
    </template>
  </div>
</template>

<style scoped>
.ws { display: grid; gap: 8px; font: 11px var(--mono); color: var(--fg); }
.lbl { color: var(--muted); text-transform: uppercase; letter-spacing: .1em; font-size: 10px; min-width: 76px; }
.seg { display: flex; align-items: center; gap: 0; flex-wrap: wrap; }
.seg .lbl { margin-right: 8px; }
.seg button { all: unset; cursor: pointer; padding: 3px 9px; border: 1px solid var(--dim); margin-left: -1px; color: var(--muted); }
.seg button.on { color: var(--accent2); border-color: var(--accent); position: relative; }
.rng { display: flex; align-items: center; gap: 8px; }
.rng input { flex: 1; accent-color: var(--accent); }
.rng span:last-child { width: 38px; text-align: right; color: var(--accent2); }
.chips { display: flex; flex-wrap: wrap; gap: 5px; align-items: center; }
.chip { border: 1px solid var(--dim); padding: 1px 7px; cursor: pointer; color: var(--muted); font-size: 10.5px; }
.chip.on { border-color: var(--accent); color: var(--accent2); }
</style>
