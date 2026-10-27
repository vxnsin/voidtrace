<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { api } from "../api";
import { startLive } from "../widgets/live";
import { CATEGORIES, WIDGETS, loadWidgets, saveWidgets, type WidgetCfg } from "../widgets/registry";
import WidgetSettings from "../widgets/WidgetSettings.vue";

// Quick setup only: what is on screen / on the dashboard. Placing and fine-tuning
// happens in game, in the overlay's edit mode.
const cfgs = ref<WidgetCfg[]>([]);
const hotkey = ref("CommandOrControl+Shift+W");
const capturing = ref(false);
const msg = ref("");

const byId = (id: string) => cfgs.value.find((w) => w.id === id)!;
const dashboard = computed(() => cfgs.value.filter((w) => w.dashboard));
const pretty = (k: string) => k.replace("CommandOrControl", "Ctrl");

async function update(id: string, patch: Partial<WidgetCfg>) {
  cfgs.value = cfgs.value.map((w) => (w.id === id ? { ...w, ...patch } : w));
  if (id === "rewards" && "pinned" in patch) await api.setSetting("overlay_enabled", patch.pinned ? "1" : "0");
  await saveWidgets(cfgs.value);
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

  <div class="card edit">
    <div>
      <b>Arrange in game</b>
      <p class="muted">Open the editor over the game: drag widgets in from the top bar, move them, click one to change it.</p>
    </div>
    <div class="row">
      <button class="btn primary" @click="api.overlayEdit(true)">Edit in game</button>
      <span class="muted">or press</span>
      <button class="btn" :class="{ primary: capturing }" @click="capturing = !capturing; msg = ''">{{ capturing ? "press keys…" : pretty(hotkey) }}</button>
    </div>
    <span v-if="msg" class="muted">{{ msg }}</span>
  </div>

  <h2>Dashboard</h2>
  <div class="dash">
    <div v-for="w in dashboard" :key="w.id" style="--wo: 1">
      <component :is="WIDGETS.find((x) => x.id === w.id)!.c" :cfg="w" />
    </div>
    <p v-if="!dashboard.length" class="muted">Nothing here yet — switch on “Dashboard” for a widget below.</p>
  </div>

  <template v-if="cfgs.length">
    <template v-for="c in CATEGORIES" :key="c">
      <h2>{{ c }}</h2>
      <div v-for="m in WIDGETS.filter((w) => w.cat === c)" :key="m.id" class="card item">
        <div class="name"><b>{{ m.name }}</b><span class="muted">{{ m.desc }}</span></div>
        <div class="toggles">
          <label class="row"><input type="checkbox" :checked="byId(m.id).pinned" @change="update(m.id, { pinned: !byId(m.id).pinned })" /> On screen</label>
          <label class="row"><input type="checkbox" :checked="byId(m.id).dashboard" @change="update(m.id, { dashboard: !byId(m.id).dashboard })" /> Dashboard</label>
        </div>
        <WidgetSettings :cfg="byId(m.id)" compact @update="(p) => update(m.id, p)" />
      </div>
    </template>
  </template>
</template>

<style scoped>
.edit { display: flex; gap: 20px; align-items: center; justify-content: space-between; flex-wrap: wrap; margin-bottom: 8px; }
.edit b { color: var(--accent2); letter-spacing: .1em; text-transform: uppercase; font-weight: 500; }
.edit p { margin: 4px 0 0; }
.dash { display: flex; flex-wrap: wrap; gap: 12px; align-items: flex-start; }
.item { display: grid; grid-template-columns: minmax(200px, 1fr) auto minmax(260px, 1.4fr); gap: 16px; align-items: start; margin-bottom: 8px; }
.name { display: grid; gap: 2px; }
.name b { color: var(--accent2); font-weight: 500; letter-spacing: .06em; }
.toggles { display: grid; gap: 4px; }
@media (max-width: 1000px) { .item { grid-template-columns: 1fr; } }
</style>
