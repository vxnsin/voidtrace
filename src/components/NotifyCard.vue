<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { NOTICE_KINDS, notifyApi, type NotifyConfig } from "../api";
import { live } from "../widgets/live";

// Which notices show in game, which make a sound, and the rules for cycles and fissures.
const cfg = ref<NotifyConfig | null>(null);
const TIERS = ["Lith", "Meso", "Neo", "Axi", "Requiem", "Omnia"];
const CYCLES = [["cetus", "Cetus"], ["vallis", "Orb Vallis"], ["cambion", "Cambion"], ["zariman", "Zariman"], ["duviri", "Duviri"], ["earth", "Earth"]];
const SP = [["all", "any"], ["only", "Steel Path only"], ["hide", "no Steel Path"]] as const;
const missions = computed(() => [...new Set([...(live.world?.fissures ?? []).map((f) => f.missionType), ...(cfg.value?.fissure.missions ?? [])])].sort());

onMounted(async () => (cfg.value = await notifyApi.config()));
async function save(patch: Partial<NotifyConfig>) {
  cfg.value = { ...cfg.value!, ...patch };
  await notifyApi.saveConfig(cfg.value);
}
const flip = (list: string[], v: string) => (list.includes(v) ? list.filter((x) => x !== v) : [...list, v]);
const fissureChip = (key: "tiers" | "missions", v: string) => save({ fissure: { ...cfg.value!.fissure, [key]: flip(cfg.value!.fissure[key], v) } });
const cycleChip = (id: string) => save({ cycles: flip(cfg.value!.cycles.length ? cfg.value!.cycles : CYCLES.map((c) => c[0]), id) });
const setLead = (e: Event) => save({ cycle_lead: Math.max(1, Number((e.target as HTMLInputElement).value) || 5) });
</script>

<template>
  <div v-if="cfg" class="card nc">
    <div v-for="k in NOTICE_KINDS" :key="k.kind" class="kind">
      <span><b>{{ k.label }}</b><span class="muted"> · {{ k.hint }}</span></span>
      <span class="row">
        <label class="row"><input type="checkbox" :checked="!cfg.off.includes(k.kind)" @change="save({ off: flip(cfg.off, k.kind) })" /> in game</label>
        <label class="row"><input type="checkbox" :checked="cfg.sound.includes(k.kind)" @change="save({ sound: flip(cfg.sound, k.kind) })" /> sound</label>
        <button class="btn" @click="notifyApi.test(k.kind)">Test</button>
      </span>
    </div>

    <div class="rule">
      <span class="lbl">Cycles</span>
      <span v-for="[id, l] in CYCLES" :key="id" class="chip" :class="{ on: !cfg.cycles.length || cfg.cycles.includes(id) }" @click="cycleChip(id)">{{ l }}</span>
      <label class="row muted">warn <input type="number" min="1" max="60" :value="cfg.cycle_lead" style="width:56px" @change="setLead" /> min before</label>
    </div>
    <div class="rule">
      <span class="lbl">Fissures</span>
      <span v-for="t in TIERS" :key="t" class="chip" :class="{ on: cfg.fissure.tiers.includes(t) }" @click="fissureChip('tiers', t)">{{ t }}</span>
    </div>
    <div class="rule">
      <span class="lbl"></span>
      <span v-for="m in missions" :key="m" class="chip" :class="{ on: cfg.fissure.missions.includes(m) }" @click="fissureChip('missions', m)">{{ m }}</span>
    </div>
    <div class="rule">
      <span class="lbl"></span>
      <div class="modes">
        <button v-for="[k, l] in SP" :key="k" :class="{ on: cfg.fissure.steelPath === k }" @click="save({ fissure: { ...cfg.fissure, steelPath: k } })">{{ l }}</button>
      </div>
      <label class="row"><input type="checkbox" :checked="cfg.fissure.storms" @change="save({ fissure: { ...cfg.fissure, storms: !cfg.fissure.storms } })" /> Void Storms</label>
    </div>
    <p class="muted" style="margin:0">Fissures are announced only once you pick a tier or a mission type. Discord webhooks are set up in Settings.</p>
  </div>
</template>

<style scoped>
.nc { display: grid; gap: 8px; }
.kind { display: flex; justify-content: space-between; align-items: center; gap: 12px; flex-wrap: wrap; }
.kind b { color: var(--accent2); font-weight: 500; }
.rule { display: flex; flex-wrap: wrap; gap: 6px; align-items: center; margin-top: 4px; }
.lbl { color: var(--muted); text-transform: uppercase; letter-spacing: .1em; font-size: 10px; min-width: 76px; }
.chip { border: 1px solid var(--dim); padding: 1px 7px; cursor: pointer; color: var(--muted); font-size: 10.5px; }
.chip.on { border-color: var(--accent); color: var(--accent2); }
</style>
