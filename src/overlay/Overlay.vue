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
const hudHidden = ref(false);
const hudKey = ref("");
const toast = ref("");
let toastTimer: number | undefined;
const pretty = (k?: string) => (k ?? "").replace("CommandOrControl", "Ctrl");

async function reload() {
  cfgs.value = await loadWidgets();
  const keys = await api.overlayHotkeys();
  hotkey.value = pretty(keys.edit);
  hudKey.value = pretty(keys.hud);
}
const shown = computed(() =>
  cfgs.value.filter((w) => {
    if (!w.pinned) return false;
    if (edit.value) return true;
    if (hudHidden.value) return false;
    if (w.id === "rewards") return live.showRewards;
    return live.game.focused && (w.show === "always" || w.show === live.game.where);
  }),
);
const byId = (id: WidgetId) => cfgs.value.find((w) => w.id === id)!;
const meta = (id: WidgetId) => WIDGETS.find((w) => w.id === id)!;
const selected = computed(() => (sel.value ? byId(sel.value) : null));

async function update(id: WidgetId, patch: Partial<WidgetCfg>) {
  cfgs.value = cfgs.value.map((w) => (w.id === id ? { ...w, ...patch } : w));
  // The reward OCR only runs while its widget is on screen.
  if (id === "rewards" && "pinned" in patch) await api.setSetting("overlay_enabled", patch.pinned ? "1" : "0");
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

// Dragging a widget back up into the palette removes it.
const palette = ref<HTMLElement | null>(null);
const overPalette = ref(false);
function inPalette(p: { x: number; y: number }) {
  const r = palette.value?.getBoundingClientRect();
  return !!r && p.x >= r.left && p.x <= r.right && p.y >= r.top && p.y <= r.bottom;
}
function dragging(id: string | null, p: { x: number; y: number }) {
  overPalette.value = !!id && inPalette(p);
}
async function moved(id: string, pos: { anchor: number; x: number; y: number }, p: { x: number; y: number }) {
  overPalette.value = false;
  if (inPalette(p)) return remove(id as WidgetId);
  await update(id as WidgetId, pos);
}
const done = () => { sel.value = null; api.overlayEdit(false); };

onMounted(async () => {
  document.body.classList.add("overlay");
  startLive();
  await reload();
  api.on("widgets-changed", reload);
  hudHidden.value = await api.hudHidden();
  api.on<boolean>("overlay-hud", (h) => {
    hudHidden.value = h;
    toast.value = h ? `Widgets hidden · ${hudKey.value} to show` : "Widgets back";
    clearTimeout(toastTimer);
    toastTimer = window.setTimeout(() => (toast.value = ""), 2500);
  });
  api.on<boolean>("overlay-edit", (on) => { edit.value = on; live.editing = on; if (on) reload(); else sel.value = null; });
  api.on<string>("overlay-error", (e) => { error.value = e; setTimeout(() => (error.value = ""), 6000); });
  api.on("rewards", () => (error.value = ""));
  window.addEventListener("keydown", (e) => { if (e.key === "Escape" && edit.value) done(); });
});
</script>

<template>
  <WidgetLayer :items="shown" :editable="edit" :selected="sel" @select="sel = $event as WidgetId" @move="moved" @drag="dragging">
    <template v-if="edit">
      <!-- palette -->
      <div ref="palette" class="palette" :class="{ drop: overPalette }" @pointerdown.stop>
        <div v-if="overPalette" class="dropmsg">Drop here to remove</div>
        <div class="head">
          <b>WIDGETS</b>
          <div class="cats"><button v-for="c in CATEGORIES" :key="c" :class="{ on: cat === c }" @click="cat = c">{{ c }}</button></div>
          <span class="hint">drag in to add · drag back up to remove · hold Alt to snap · {{ hotkey }} / Esc to finish</span>
          <button class="act" :class="{ on: sel === null }" @click="sel = null">All widgets</button>
          <button class="act done" @click="done">Done</button>
        </div>
        <div class="tiles">
          <div v-for="m in WIDGETS.filter((w) => w.cat === cat)" :key="m.id" class="tile" :class="{ pinned: byId(m.id)?.pinned, sel: sel === m.id }" @pointerdown.prevent="tileDown($event, m.id)">
            <b>{{ m.name }}</b><span>{{ m.desc }}</span><i>{{ byId(m.id)?.pinned ? "on screen" : "drag in" }}</i>
            <button v-if="byId(m.id)?.pinned" class="x" title="Remove from screen" @pointerdown.stop @click.stop="remove(m.id)">×</button>
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
    <div v-if="toast && live.game.focused" class="toast">{{ toast }}</div>
  </WidgetLayer>
</template>

<style scoped>
.palette { position: absolute; top: 0; left: 50%; transform: translateX(-50%); width: min(980px, 92vw); background: color-mix(in srgb, var(--wbg) 95%, transparent); border: 1px solid var(--accent); border-top: 0; font: 11px var(--mono); color: var(--fg2); z-index: 20; }
.head { display: flex; align-items: center; gap: 14px; padding: 8px 12px; border-bottom: 1px solid var(--border); flex-wrap: wrap; }
.head b { color: var(--accent2); letter-spacing: .18em; font-weight: 500; }
.cats { display: flex; }
.cats button, .act { all: unset; cursor: pointer; padding: 3px 10px; border: 1px solid var(--dim); margin-left: -1px; color: var(--muted); text-transform: uppercase; letter-spacing: .08em; font-size: 10.5px; }
.cats button.on, .act.on { color: var(--accent2); border-color: var(--accent); }
.act.done { border-color: var(--accent); color: var(--accent2); }
.act.done:hover { background: var(--accent); color: var(--on-accent); }
.hint { flex: 1; color: var(--muted); min-width: 200px; }
.tiles { display: flex; gap: 8px; padding: 10px 12px; overflow-x: auto; }
.tile { flex: 0 0 210px; border: 1px dashed var(--dim); padding: 8px 10px; cursor: grab; display: grid; gap: 3px; user-select: none; }
.tile b { color: var(--accent2); font-weight: 500; letter-spacing: .06em; }
.tile span { color: var(--muted); font-size: 10.5px; }
.tile i { font-style: normal; color: var(--muted); font-size: 10px; text-transform: uppercase; letter-spacing: .1em; }
.tile.pinned { border-style: solid; } .tile.pinned i { color: var(--warn); }
.tile.sel { border-color: var(--accent); }
.tile { position: relative; }
.tile .x { all: unset; position: absolute; top: 4px; right: 6px; cursor: pointer; color: var(--muted); font-size: 14px; line-height: 1; padding: 2px 4px; }
.tile .x:hover { color: var(--accent); }
.palette.drop { background: color-mix(in srgb, var(--accent) 30%, var(--wbg)); }
.dropmsg { position: absolute; inset: 0; display: flex; align-items: center; justify-content: center; color: var(--accent2); letter-spacing: .2em; text-transform: uppercase; font-size: 12px; background: color-mix(in srgb, var(--accent) 25%, transparent); z-index: 2; pointer-events: none; }
.panel { position: absolute; right: 16px; top: 50%; transform: translateY(-50%); width: 320px; background: color-mix(in srgb, var(--wbg) 95%, transparent); border: 1px solid var(--accent); padding: 12px; z-index: 20; }
.ph { display: flex; justify-content: space-between; align-items: center; margin-bottom: 10px; }
.ph b { color: var(--accent2); letter-spacing: .12em; text-transform: uppercase; font: 500 11px var(--mono); }
.ws-all { display: grid; gap: 10px; font: 11px var(--mono); color: var(--fg2); }
.ws-all label { display: flex; align-items: center; gap: 10px; }
.ws-all label span { width: 70px; text-transform: uppercase; font-size: 10px; letter-spacing: .1em; color: var(--muted); }
.ws-all input { flex: 1; accent-color: var(--accent); }
.ws-all p { margin: 0; color: var(--muted); }
.ghost { position: fixed; transform: translate(-50%, -50%); pointer-events: none; opacity: .9; z-index: 30; }
.toast { position: absolute; top: 14px; left: 50%; transform: translateX(-50%); background: color-mix(in srgb, var(--wbg) 90%, transparent); border: 1px solid var(--dim); color: var(--fg); padding: 5px 12px; font: 11px var(--mono); letter-spacing: .06em; }
.err { position: absolute; top: 60px; left: 50%; transform: translateX(-50%); background: color-mix(in srgb, var(--wbg) 92%, transparent); border: 1px solid var(--accent); color: var(--accent); padding: 6px 12px; font: 11px var(--mono); }
</style>
