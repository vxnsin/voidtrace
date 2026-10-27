<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { api } from "../api";
import { live, startLive } from "../widgets/live";
import { loadWidgets, saveWidgets, type WidgetCfg } from "../widgets/registry";
import WidgetLayer from "../widgets/WidgetLayer.vue";

// The overlay window is one transparent layer over the screen. Pinned widgets
// show while the game is in front and their rule matches; the relic rewards
// widget shows itself on the reward screen. Edit mode (hotkey) shows them all.
const cfgs = ref<WidgetCfg[]>([]);
const edit = ref(false);
const sel = ref<string | null>(null);
const error = ref("");
const hotkey = ref("");

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
async function moved(id: string, pos: { anchor: number; x: number; y: number }) {
  cfgs.value = cfgs.value.map((w) => (w.id === id ? { ...w, ...pos } : w));
  await saveWidgets(cfgs.value);
}
const done = () => api.overlayEdit(false);

onMounted(async () => {
  document.body.classList.add("overlay");
  startLive();
  await reload();
  api.on("widgets-changed", reload);
  api.on<boolean>("overlay-edit", (on) => { edit.value = on; if (on) reload(); });
  api.on<string>("overlay-error", (e) => { error.value = e; setTimeout(() => (error.value = ""), 6000); });
  api.on("rewards", () => (error.value = ""));
  window.addEventListener("keydown", (e) => { if (e.key === "Escape" && edit.value) done(); });
});
</script>

<template>
  <WidgetLayer :items="shown" :editable="edit" :selected="sel" @select="sel = $event" @move="moved">
    <div v-if="edit" class="bar">
      <b>EDIT OVERLAY</b><span>drag widgets · {{ hotkey }} or Esc to finish</span>
      <button @click="done">Done</button>
    </div>
    <div v-if="error" class="err">{{ error }}</div>
  </WidgetLayer>
</template>

<style scoped>
.bar { position: absolute; top: 12px; left: 50%; transform: translateX(-50%); display: flex; gap: 14px; align-items: center; background: rgba(7, 7, 8, .94); border: 1px solid #c8102e; padding: 8px 14px; font: 11px var(--mono); color: #8a8a90; letter-spacing: .06em; z-index: 10; }
.bar b { color: #e8e8e8; letter-spacing: .16em; font-weight: 500; }
.bar button { all: unset; cursor: pointer; border: 1px solid #c8102e; color: #e8e8e8; padding: 3px 10px; text-transform: uppercase; letter-spacing: .1em; }
.bar button:hover { background: #c8102e; color: #000; }
.err { position: absolute; top: 60px; left: 50%; transform: translateX(-50%); background: rgba(7, 7, 8, .92); border: 1px solid #c8102e; color: #c8102e; padding: 6px 12px; font: 11px var(--mono); }
</style>
