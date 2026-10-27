<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import { api } from "../api";
import { startLive } from "../widgets/live";
import { CATEGORIES, WIDGETS, loadWidgets, saveWidgets, type WidgetCfg } from "../widgets/registry";
import WidgetSettings from "../widgets/WidgetSettings.vue";
import NotifyCard from "../components/NotifyCard.vue";

// Quick setup: each widget live with its settings next to it. Placing and
// fine-tuning happens in game, in the overlay's edit mode.
const cfgs = ref<WidgetCfg[]>([]);
type Action = "edit" | "hud" | "riven";
const HOTKEYS: { action: Action; label: string; hint: string }[] = [
  { action: "edit", label: "Edit widgets", hint: "opens the editor over the game" },
  { action: "hud", label: "Hide / show widgets", hint: "hides every widget until pressed again" },
  { action: "riven", label: "Scan riven on screen", hint: "reads the riven card in front of you: grades, listings, rolls" },
];
const keys = ref<Record<string, string>>({});
const capturing = ref<Action | null>(null);
const msg = ref("");

const byId = (id: string) => cfgs.value.find((w) => w.id === id)!;
const pretty = (k: string) => k.replace("CommandOrControl", "Ctrl");

async function update(id: string, patch: Partial<WidgetCfg>) {
  cfgs.value = cfgs.value.map((w) => (w.id === id ? { ...w, ...patch } : w));
  await saveWidgets(cfgs.value);
}

// Hotkeys: click one, press the new combination (needs Ctrl, Alt or Shift).
function capture(e: KeyboardEvent) {
  const action = capturing.value;
  if (!action) return;
  e.preventDefault();
  if (["Control", "Shift", "Alt", "Meta"].includes(e.key)) return;
  if (e.key === "Escape") { capturing.value = null; return; }
  const mods = [e.ctrlKey && "CommandOrControl", e.altKey && "Alt", e.shiftKey && "Shift"].filter(Boolean) as string[];
  if (!mods.length) { msg.value = "Use Ctrl, Alt or Shift with the key."; return; }
  const key = e.code.startsWith("Key") ? e.code.slice(3) : e.code.startsWith("Digit") ? e.code.slice(5) : e.code;
  const combo = [...mods, key].join("+");
  capturing.value = null;
  api.setOverlayHotkey(action, combo)
    .then(async () => { keys.value = await api.overlayHotkeys(); msg.value = `${pretty(combo)} saved.`; })
    .catch((err) => (msg.value = String(err)));
}

onMounted(async () => {
  startLive();
  cfgs.value = await loadWidgets();
  keys.value = await api.overlayHotkeys();
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
      <span class="muted">or press {{ pretty(keys.edit ?? "") }}</span>
    </div>
  </div>

  <h2>Hotkeys</h2>
  <div class="card hk">
    <div v-for="h in HOTKEYS" :key="h.action" class="hkrow">
      <span><b>{{ h.label }}</b><span class="muted"> · {{ h.hint }}</span></span>
      <button class="btn" :class="{ primary: capturing === h.action }" @click="capturing = capturing === h.action ? null : h.action; msg = ''">{{ capturing === h.action ? "press keys… (Esc cancels)" : pretty(keys[h.action] ?? "–") }}</button>
    </div>
    <span class="muted">{{ msg || "Click a hotkey and press the new combination. They work system-wide while VoidTrace runs, so pick combinations other programs do not need." }}</span>
  </div>

  <h2>Notifications</h2>
  <NotifyCard />

  <template v-if="cfgs.length">
    <template v-for="c in CATEGORIES" :key="c">
      <h2>{{ c }}</h2>
      <div v-for="m in WIDGETS.filter((w) => w.cat === c)" :key="m.id" class="card item">
        <div class="preview" style="--wo: 1"><component :is="m.c" :cfg="byId(m.id)" /></div>
        <div class="side">
          <div class="name"><b>{{ m.name }}</b><span class="muted">{{ m.desc }}</span></div>
          <span class="state" :class="{ on: byId(m.id).pinned }">{{ byId(m.id).pinned ? "placed in game" : "not placed — add it in the editor" }}</span>
          <WidgetSettings :cfg="byId(m.id)" compact @update="(p) => update(m.id, p)" />
        </div>
      </div>
    </template>
  </template>
</template>

<style scoped>
.edit { display: flex; gap: 20px; align-items: center; justify-content: space-between; flex-wrap: wrap; margin-bottom: 8px; }
.edit b { color: var(--accent2); letter-spacing: .1em; text-transform: uppercase; font-weight: 500; }
.edit p { margin: 4px 0 0; }
.hk { display: grid; gap: 8px; }
.hkrow { display: flex; justify-content: space-between; align-items: center; gap: 16px; flex-wrap: wrap; }
.hkrow b { color: var(--accent2); font-weight: 500; }
.state { font-size: 10.5px; letter-spacing: .08em; text-transform: uppercase; color: var(--muted); }
.state.on { color: var(--warn); }
.item { display: grid; grid-template-columns: auto minmax(280px, 1fr); gap: 24px; align-items: start; margin-bottom: 10px; }
.preview { min-width: 240px; }
.side { display: grid; gap: 10px; align-content: start; }
.name { display: grid; gap: 2px; }
.name b { color: var(--accent2); font-weight: 500; letter-spacing: .06em; }
@media (max-width: 1000px) { .item { grid-template-columns: 1fr; } }
</style>
