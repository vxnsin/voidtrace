<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, reactive, ref } from "vue";
import { WIDGETS, placeFromRect, placeStyle, type WidgetCfg } from "./registry";

// Places widgets on a screen-sized layer. In the overlay the layer is the screen;
// in the layout editor (`virtualW/H` set) it is a screen-sized canvas scaled into
// its box, so positions match the game 1:1.
const props = defineProps<{ items: WidgetCfg[]; editable?: boolean; selected?: string | null; virtualW?: number; virtualH?: number }>();
const emit = defineEmits<{
  select: [id: string];
  move: [id: string, pos: { anchor: number; x: number; y: number }, pointer: { x: number; y: number }];
  drag: [id: string | null, pointer: { x: number; y: number }];
}>();

// Holding Alt snaps to this grid (columns x rows of the screen).
const GRID_X = 48, GRID_Y = 27;

const box = ref<HTMLElement | null>(null);
const layer = ref<HTMLElement | null>(null);
const k = ref(1);
const alt = ref(false);
const offsets = reactive<Record<string, { dx: number; dy: number }>>({});
let drag: { id: string; el: HTMLElement; sx: number; sy: number; r0: DOMRect; L: DOMRect } | null = null;
let ro: ResizeObserver | undefined;
const onKey = (e: KeyboardEvent) => (alt.value = e.altKey);

const comp = (id: string) => WIDGETS.find((w) => w.id === id)!.c;
const style = (w: WidgetCfg) => {
  const s = placeStyle(w);
  const o = offsets[w.id];
  if (o) s.transform = `translate(${o.dx}px, ${o.dy}px) ${s.transform}`;
  return { ...s, "--wo": String(w.opacity) };
};
const inner = computed(() =>
  props.virtualW ? { width: `${props.virtualW}px`, height: `${props.virtualH}px`, transform: `scale(${k.value})` } : {},
);

function down(e: PointerEvent, w: WidgetCfg) {
  if (!props.editable) return;
  emit("select", w.id);
  const el = e.currentTarget as HTMLElement;
  drag = { id: w.id, el, sx: e.clientX, sy: e.clientY, r0: el.getBoundingClientRect(), L: layer.value!.getBoundingClientRect() };
  el.setPointerCapture(e.pointerId);
}
// Keep the widget fully on screen; with Alt, snap its corner to the grid.
function move(e: PointerEvent) {
  if (!drag) return;
  alt.value = e.altKey;
  const { r0, L } = drag;
  let left = r0.left + e.clientX - drag.sx, top = r0.top + e.clientY - drag.sy;
  if (e.altKey) {
    const gx = L.width / GRID_X, gy = L.height / GRID_Y;
    left = L.left + Math.round((left - L.left) / gx) * gx;
    top = L.top + Math.round((top - L.top) / gy) * gy;
  }
  left = Math.min(Math.max(left, L.left), L.right - r0.width);
  top = Math.min(Math.max(top, L.top), L.bottom - r0.height);
  offsets[drag.id] = { dx: (left - r0.left) / k.value, dy: (top - r0.top) / k.value };
  emit("drag", drag.id, { x: e.clientX, y: e.clientY });
}
function up(e: PointerEvent) {
  if (!drag) return;
  const d = drag;
  drag = null;
  emit("move", d.id, placeFromRect(d.el.getBoundingClientRect(), layer.value!.getBoundingClientRect()), { x: e.clientX, y: e.clientY });
  emit("drag", null, { x: e.clientX, y: e.clientY });
  delete offsets[d.id];
}

onMounted(() => {
  window.addEventListener("keydown", onKey);
  window.addEventListener("keyup", onKey);
  if (!props.virtualW || !box.value) return;
  ro = new ResizeObserver(() => (k.value = box.value!.clientWidth / props.virtualW!));
  ro.observe(box.value);
});
onBeforeUnmount(() => {
  ro?.disconnect();
  window.removeEventListener("keydown", onKey);
  window.removeEventListener("keyup", onKey);
});
</script>

<template>
  <div ref="box" class="box" :class="{ virtual: !!virtualW }" :style="virtualW ? { aspectRatio: `${virtualW} / ${virtualH}` } : {}">
    <div ref="layer" class="layer" :class="{ edit: editable, snap: editable && alt }" :style="inner" @pointermove="move" @pointerup="up">
      <div v-for="w in items" :key="w.id" class="slot" :class="{ sel: editable && selected === w.id }" :style="style(w)" @pointerdown="down($event, w)">
        <component :is="comp(w.id)" :cfg="w" />
      </div>
      <slot />
    </div>
  </div>
</template>

<style scoped>
.box { position: relative; width: 100%; height: 100%; overflow: hidden; }
.box.virtual { height: auto; border: 1px solid var(--border); background: linear-gradient(180deg, #1a1f27 0%, #10141a 45%, #0b0d10 100%); }
.layer { position: absolute; inset: 0; transform-origin: top left; }
.box.virtual .layer { inset: auto; left: 0; top: 0; }
.slot { position: absolute; }
.layer.edit .slot { cursor: move; outline: 1px dashed color-mix(in srgb, var(--accent) 70%, transparent); }
.layer.edit .slot.sel { outline: 2px solid var(--accent); }
.layer.edit { background-image: linear-gradient(color-mix(in srgb, var(--accent) 07%, transparent) 1px, transparent 1px), linear-gradient(90deg, color-mix(in srgb, var(--accent) 07%, transparent) 1px, transparent 1px); background-size: 33.333% 33.333%; }
/* Alt held: the snap grid (48 x 27, see GRID_X / GRID_Y). */
.layer.edit.snap { background-image: linear-gradient(color-mix(in srgb, var(--accent2) 09%, transparent) 1px, transparent 1px), linear-gradient(90deg, color-mix(in srgb, var(--accent2) 09%, transparent) 1px, transparent 1px); background-size: calc(100% / 48) calc(100% / 27); }
</style>
