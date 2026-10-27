<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { api } from "../api";
import { live, startLive } from "../widgets/live";
import { WIDGETS, loadWidgets, saveWidgets, type FissureFilter, type WidgetCfg } from "../widgets/registry";
import WidgetLayer from "../widgets/WidgetLayer.vue";

const cfgs = ref<WidgetCfg[]>([]);
const sel = ref<string | null>(null);
const hotkey = ref("CommandOrControl+Shift+W");
const capturing = ref(false);
const msg = ref("");
const TIERS = ["Lith", "Meso", "Neo", "Axi", "Requiem", "Omnia"];

const byId = (id: string) => cfgs.value.find((w) => w.id === id)!;
const pinned = computed(() => cfgs.value.filter((w) => w.pinned));
const dashboard = computed(() => cfgs.value.filter((w) => w.dashboard));
const missions = computed(() => [...new Set((live.world?.fissures ?? []).map((f) => f.missionType))].sort());
const screenW = window.screen.width, screenH = window.screen.height;
const pretty = (k: string) => k.replace("CommandOrControl", "Ctrl");

async function update(id: string, patch: Partial<WidgetCfg>) {
  cfgs.value = cfgs.value.map((w) => (w.id === id ? { ...w, ...patch } : w));
  if (id === "rewards" && "pinned" in patch) await api.setSetting("overlay_enabled", patch.pinned ? "1" : "0");
  await saveWidgets(cfgs.value);
}
function filterOf(w: WidgetCfg): FissureFilter {
  return w.filter ?? { tiers: [], missions: [], steelPath: "all", storms: true };
}
function toggleIn(w: WidgetCfg, key: "tiers" | "missions", v: string) {
  const f = filterOf(w);
  const list = f[key].includes(v) ? f[key].filter((x) => x !== v) : [...f[key], v];
  update(w.id, { filter: { ...f, [key]: list } });
}

// Hotkey: press the new combination; needs at least one modifier.
function capture(e: KeyboardEvent) {
  if (!capturing.value) return;
  e.preventDefault();
  if (["Control", "Shift", "Alt", "Meta"].includes(e.key)) return;
  if (e.key === "Escape") { capturing.value = false; return; }
  const mods = [e.ctrlKey && "CommandOrControl", e.altKey && "Alt", e.shiftKey && "Shift"].filter(Boolean) as string[];
  if (!mods.length) { msg.value = "Use Ctrl, Alt or Shift with the key."; return; }
  const key = e.code.startsWith("Key") ? e.code.slice(3) : e.code.startsWith("Digit") ? e.code.slice(5) : e.code;
  const combo = [...mods, key].join("+");
  capturing.value = false;
  api.setOverlayHotkey(combo).then(() => { hotkey.value = combo; msg.value = `Hotkey is now ${pretty(combo)}.`; }).catch((err) => (msg.value = String(err)));
}

onMounted(async () => {
  startLive();
  cfgs.value = await loadWidgets();
  hotkey.value = (await api.getSetting("overlay_hotkey")) ?? hotkey.value;
  api.on("widgets-changed", async () => (cfgs.value = await loadWidgets()));
  window.addEventListener("keydown", capture);
});
onUnmounted(() => window.removeEventListener("keydown", capture));
</script>

<template>
  <h1>Widgets</h1>
  <p class="muted">Pin widgets to the in-game overlay or keep them here on the dashboard. Pinned widgets show while Warframe is in front.</p>

  <h2>Dashboard</h2>
  <div class="dash">
    <div v-for="w in dashboard" :key="w.id" style="--wo: 1">
      <component :is="WIDGETS.find((x) => x.id === w.id)!.c" :cfg="w" />
    </div>
    <p v-if="!dashboard.length" class="muted">Nothing on the dashboard — tick “Dashboard” on a widget below.</p>
  </div>

  <h2>Overlay layout</h2>
  <div class="row" style="margin-bottom:10px">
    <button class="btn primary" @click="api.overlayEdit(true)">Edit in game</button>
    <span class="muted">or press</span>
    <button class="btn" :class="{ primary: capturing }" @click="capturing = !capturing; msg = ''">{{ capturing ? "press keys…" : pretty(hotkey) }}</button>
    <span class="muted">{{ msg || "click the hotkey to change it" }}</span>
  </div>
  <WidgetLayer :items="pinned" editable :selected="sel" :virtual-w="screenW" :virtual-h="screenH" @select="sel = $event" @move="(id, pos) => update(id, pos)" />
  <p class="muted" style="margin-top:6px">Drag to place. Widgets snap to the nearest corner, edge or centre, so the layout fits any resolution.</p>

  <h2>All widgets</h2>
  <div v-if="cfgs.length" class="gallery">
    <div v-for="m in WIDGETS" :key="m.id" class="card cfg" :class="{ sel: sel === m.id }" @click="sel = m.id">
      <div class="head"><b>{{ m.name }}</b></div>
      <span class="muted">{{ m.desc }}</span>
      <div class="ctl">
        <label class="row"><input type="checkbox" :checked="byId(m.id).pinned" @change="update(m.id, { pinned: !byId(m.id).pinned })" /> Pin to overlay</label>
        <label class="row"><input type="checkbox" :checked="byId(m.id).dashboard" @change="update(m.id, { dashboard: !byId(m.id).dashboard })" /> Dashboard</label>
      </div>
      <div class="ctl">
        <label>Size <select :value="byId(m.id).size" @change="update(m.id, { size: ($event.target as HTMLSelectElement).value as WidgetCfg['size'] })"><option value="compact">compact</option><option value="full">full</option></select></label>
        <label v-if="m.id !== 'rewards'">Show <select :value="byId(m.id).show" @change="update(m.id, { show: ($event.target as HTMLSelectElement).value as WidgetCfg['show'] })"><option value="always">always</option><option value="mission">in missions</option><option value="orbiter">in orbiter / relays</option></select></label>
      </div>
      <div class="ctl">
        <label>Opacity <input type="range" min="0.4" max="1" step="0.02" :value="byId(m.id).opacity" @change="update(m.id, { opacity: Number(($event.target as HTMLInputElement).value) })" /></label>
        <label>Scale <input type="range" min="0.7" max="1.6" step="0.05" :value="byId(m.id).scale" @change="update(m.id, { scale: Number(($event.target as HTMLInputElement).value) })" /></label>
      </div>
      <template v-if="m.id === 'fissures'">
        <div class="chips"><span class="muted">Tiers</span>
          <span v-for="t in TIERS" :key="t" class="chip" :class="{ on: filterOf(byId(m.id)).tiers.includes(t) }" @click.stop="toggleIn(byId(m.id), 'tiers', t)">{{ t }}</span>
        </div>
        <div class="chips"><span class="muted">Missions</span>
          <span v-for="t in missions" :key="t" class="chip" :class="{ on: filterOf(byId(m.id)).missions.includes(t) }" @click.stop="toggleIn(byId(m.id), 'missions', t)">{{ t }}</span>
        </div>
        <div class="ctl">
          <label>Steel Path <select :value="filterOf(byId(m.id)).steelPath" @change="update(m.id, { filter: { ...filterOf(byId(m.id)), steelPath: ($event.target as HTMLSelectElement).value as FissureFilter['steelPath'] } })"><option value="all">include</option><option value="only">only</option><option value="hide">hide</option></select></label>
          <label class="row"><input type="checkbox" :checked="filterOf(byId(m.id)).storms" @change="update(m.id, { filter: { ...filterOf(byId(m.id)), storms: !filterOf(byId(m.id)).storms } })" /> Void storms</label>
        </div>
      </template>
    </div>
  </div>
</template>

<style scoped>
.dash { display: flex; flex-wrap: wrap; gap: 12px; align-items: flex-start; }
.gallery { display: grid; grid-template-columns: repeat(auto-fill, minmax(320px, 1fr)); gap: 12px; }
.cfg { display: flex; flex-direction: column; gap: 8px; cursor: default; }
.cfg.sel { border-color: var(--accent); }
.head b { color: var(--accent2); letter-spacing: .1em; text-transform: uppercase; font-weight: 500; }
.ctl { display: flex; flex-wrap: wrap; gap: 14px; align-items: center; }
.ctl label { display: flex; gap: 6px; align-items: center; color: var(--fg2); }
.chips { display: flex; flex-wrap: wrap; gap: 6px; align-items: center; }
.chip { border: 1px solid var(--dim); padding: 1px 8px; font-size: 10.5px; cursor: pointer; color: var(--muted); }
.chip.on { border-color: var(--accent); color: var(--accent2); }
</style>
