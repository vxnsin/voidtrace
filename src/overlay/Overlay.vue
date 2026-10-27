<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { api } from "../api";
import { live, startLive } from "../widgets/live";
import { CATEGORIES, WIDGETS, loadWidgets, placeFromRect, saveWidgets, type WidgetCfg, type WidgetId } from "../widgets/registry";
import WidgetLayer from "../widgets/WidgetLayer.vue";
import WidgetSettings from "../widgets/WidgetSettings.vue";

// The overlay window is one transparent layer over the screen. Outside edit mode
// pinned widgets show while the game is in front and their rule matches (relic
// rewards show themselves on the reward screen). Edit mode (hotkey) is the widget
// editor: a palette at the top to drag widgets in, a panel for the selected one.
const cfgs = ref<WidgetCfg[]>([]);
const edit = ref(false);
const sel = ref<WidgetId | null>(null);
const cat = ref<(typeof CATEGORIES)[number]>("World");
const error = ref("");
const hotkey = ref("");
const ghost = ref<{ id: WidgetId; x: number; y: number; moved: boolean } | null>(null);
const ghostEl = ref<HTMLElement | null>(null);

async function reload() {
  cfgs.value = await loadWidgets();
  hotkey.value = ((await api.getSetting("overlay_hotkey")) ?? "CommandOrControl+Shift+W").replace("CommandOrControl", "Ctrl");
}
const shown = computed(() =>
  cfgs.value.filter((w) => {
    if (!w.pinned) return false;
    if (edit.value) return true;
    if (w.id === "rewards") return live.showRewards;
    return live.game.focused && (w.show === "always" || w.show === live.game.where);
  }),
);
const byId = (id: WidgetId) => cfgs.value.find((w) => w.id === id)!;
const meta = (id: WidgetId) => WIDGETS.find((w) => w.id === id)!;
const selected = computed(() => (sel.value ? byId(sel.value) : null));

async function update(id: WidgetId, patch: Partial<WidgetCfg>) {
  cfgs.value = cfgs.value.map((w) => (w.id === id ? { ...w, ...patch } : w));
  await saveWidgets(cfgs.value);
}
async function updateAll(patch: Partial<WidgetCfg>) {
  cfgs.value = cfgs.value.map((w) => ({ ...w, ...patch }));
  await saveWidgets(cfgs.value);
}
function remove(id: WidgetId) { update(id, { pinned: false }); sel.value = null; }

// Palette: drag a tile onto the screen to pin it there; a click selects or pins in place.
function tileDown(e: PointerEvent, id: WidgetId) {
  ghost.value = { id, x: e.clientX, y: e.clientY, moved: false };
  const sx = e.clientX, sy = e.clientY;
  const mv = (ev: PointerEvent) => {
    if (!ghost.value) return;
    ghost.value = { ...ghost.value, x: ev.clientX, y: ev.clientY, moved: ghost.value.moved || Math.hypot(ev.clientX - sx, ev.clientY - sy) > 6 };
  };
  const upEv = async (ev: PointerEvent) => {
    window.removeEventListener("pointermove", mv);
    const g = ghost.value;
    if (g?.moved && ev.clientY > 120 && ghostEl.value) {
      const pos = placeFromRect(ghostEl.value.getBoundingClientRect(), new DOMRect(0, 0, window.innerWidth, window.innerHeight));
      await update(g.id, { pinned: true, ...pos });
      sel.value = g.id;
    } else if (g && !g.moved) {
      if (!byId(g.id).pinned) await update(g.id, { pinned: true });
      sel.value = g.id;
    }
    ghost.value = null;
  };
  window.addEventListener("pointermove", mv);
  window.addEventListener("pointerup", upEv, { once: true });
}

async function moved(id: string, pos: { anchor: number; x: number; y: number }) {
  await update(id as WidgetId, pos);
}
const done = () => { sel.value = null; api.overlayEdit(false); };

onMounted(async () => {
  document.body.classList.add("overlay");
  startLive();
  await reload();
  api.on("widgets-changed", reload);
  api.on<boolean>("overlay-edit", (on) => { edit.value = on; if (on) reload(); else sel.value = null; });
  api.on<string>("overlay-error", (e) => { error.value = e; setTimeout(() => (error.value = ""), 6000); });
  api.on("rewards", () => (error.value = ""));
  window.addEventListener("keydown", (e) => { if (e.key === "Escape" && edit.value) done(); });
});
</script>

<template>
  <WidgetLayer :items="shown" :editable="edit" :selected="sel" @select="sel = $event as WidgetId" @move="moved">
    <template v-if="edit">
      <!-- palette -->
      <div class="palette" @pointerdown.stop>
        <div class="head">
          <b>WIDGETS</b>
          <div class="cats"><button v-for="c in CATEGORIES" :key="c" :class="{ on: cat === c }" @click="cat = c">{{ c }}</button></div>
          <span class="hint">drag onto the screen · click a widget to edit · {{ hotkey }} / Esc to finish</span>
          <button class="act" :class="{ on: sel === null }" @click="sel = null">All widgets</button>
          <button class="act done" @click="done">Done</button>
        </div>
        <div class="tiles">
          <div v-for="m in WIDGETS.filter((w) => w.cat === cat)" :key="m.id" class="tile" :class="{ pinned: byId(m.id)?.pinned, sel: sel === m.id }" @pointerdown.prevent="tileDown($event, m.id)">
            <b>{{ m.name }}</b><span>{{ m.desc }}</span><i>{{ byId(m.id)?.pinned ? "on screen" : "drag in" }}</i>
          </div>
        </div>
      </div>

      <!-- settings panel -->
      <div class="panel" @pointerdown.stop>
        <template v-if="selected">
          <div class="ph"><b>{{ meta(selected.id).name }}</b><button class="act" @click="remove(selected.id)">Remove</button></div>
          <WidgetSettings :cfg="selected" @update="(p) => update(selected!.id, p)" />
        </template>
        <template v-else>
          <div class="ph"><b>All widgets</b></div>
          <div class="ws-all">
            <label><span>Opacity</span><input type="range" min="0.4" max="1" step="0.02" :value="cfgs[0]?.opacity ?? 0.88" @change="updateAll({ opacity: Number(($event.target as HTMLInputElement).value) })" /></label>
            <label><span>Scale</span><input type="range" min="0.7" max="1.6" step="0.05" :value="cfgs[0]?.scale ?? 1" @change="updateAll({ scale: Number(($event.target as HTMLInputElement).value) })" /></label>
            <p>Sets every widget at once. Pick one on the screen to change just that one. The hotkey is changed on the Widgets tab in the app.</p>
          </div>
        </template>
      </div>

      <!-- drag ghost -->
      <div v-if="ghost?.moved" ref="ghostEl" class="ghost" :style="{ left: ghost.x + 'px', top: ghost.y + 'px', '--wo': String(byId(ghost.id).opacity) }">
        <component :is="meta(ghost.id).c" :cfg="byId(ghost.id)" />
      </div>
    </template>
    <div v-if="error" class="err">{{ error }}</div>
  </WidgetLayer>
</template>

<style scoped>
.palette { position: absolute; top: 0; left: 50%; transform: translateX(-50%); width: min(980px, 92vw); background: rgba(7, 7, 8, .95); border: 1px solid #c8102e; border-top: 0; font: 11px var(--mono); color: #8a8a90; z-index: 20; }
.head { display: flex; align-items: center; gap: 14px; padding: 8px 12px; border-bottom: 1px solid #1d1d21; flex-wrap: wrap; }
.head b { color: #e8e8e8; letter-spacing: .18em; font-weight: 500; }
.cats { display: flex; }
.cats button, .act { all: unset; cursor: pointer; padding: 3px 10px; border: 1px solid #2a2a2e; margin-left: -1px; color: #6a6a70; text-transform: uppercase; letter-spacing: .08em; font-size: 10.5px; }
.cats button.on, .act.on { color: #e8e8e8; border-color: #c8102e; }
.act.done { border-color: #c8102e; color: #e8e8e8; }
.act.done:hover { background: #c8102e; color: #000; }
.hint { flex: 1; color: #4e4e52; min-width: 200px; }
.tiles { display: flex; gap: 8px; padding: 10px 12px; overflow-x: auto; }
.tile { flex: 0 0 210px; border: 1px dashed #2a2a2e; padding: 8px 10px; cursor: grab; display: grid; gap: 3px; user-select: none; }
.tile b { color: #e8e8e8; font-weight: 500; letter-spacing: .06em; }
.tile span { color: #6a6a70; font-size: 10.5px; }
.tile i { font-style: normal; color: #4e4e52; font-size: 10px; text-transform: uppercase; letter-spacing: .1em; }
.tile.pinned { border-style: solid; } .tile.pinned i { color: #b89a5a; }
.tile.sel { border-color: #c8102e; }
.panel { position: absolute; right: 16px; top: 50%; transform: translateY(-50%); width: 320px; background: rgba(7, 7, 8, .95); border: 1px solid #c8102e; padding: 12px; z-index: 20; }
.ph { display: flex; justify-content: space-between; align-items: center; margin-bottom: 10px; }
.ph b { color: #e8e8e8; letter-spacing: .12em; text-transform: uppercase; font: 500 11px var(--mono); }
.ws-all { display: grid; gap: 10px; font: 11px var(--mono); color: #8a8a90; }
.ws-all label { display: flex; align-items: center; gap: 10px; }
.ws-all label span { width: 70px; text-transform: uppercase; font-size: 10px; letter-spacing: .1em; color: #6a6a70; }
.ws-all input { flex: 1; accent-color: #c8102e; }
.ws-all p { margin: 0; color: #4e4e52; }
.ghost { position: fixed; transform: translate(-50%, -50%); pointer-events: none; opacity: .9; z-index: 30; }
.err { position: absolute; top: 60px; left: 50%; transform: translateX(-50%); background: rgba(7, 7, 8, .92); border: 1px solid #c8102e; color: #c8102e; padding: 6px 12px; font: 11px var(--mono); }
</style>
