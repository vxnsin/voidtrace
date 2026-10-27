<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, reactive, ref } from "vue";
import { WIDGETS, placeFromRect, placeStyle, type WidgetCfg } from "./registry";

// Places widgets on a screen-sized layer. In the overlay the layer is the screen;
// in the layout editor (`virtualW/H` set) it is a screen-sized canvas scaled into
// its box, so positions match the game 1:1.
const props = defineProps<{ items: WidgetCfg[]; editable?: boolean; selected?: string | null; virtualW?: number; virtualH?: number }>();
const emit = defineEmits<{ select: [id: string]; move: [id: string, pos: { anchor: number; x: number; y: number }] }>();

const box = ref<HTMLElement | null>(null);
const layer = ref<HTMLElement | null>(null);
const k = ref(1);
const offsets = reactive<Record<string, { dx: number; dy: number }>>({});
let drag: { id: string; el: HTMLElement; sx: number; sy: number } | null = null;
let ro: ResizeObserver | undefined;

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
  drag = { id: w.id, el: e.currentTarget as HTMLElement, sx: e.clientX, sy: e.clientY };
  drag.el.setPointerCapture(e.pointerId);
}
function move(e: PointerEvent) {
  if (drag) offsets[drag.id] = { dx: (e.clientX - drag.sx) / k.value, dy: (e.clientY - drag.sy) / k.value };
}
function up() {
  if (!drag) return;
  const d = drag;
  drag = null;
  emit("move", d.id, placeFromRect(d.el.getBoundingClientRect(), layer.value!.getBoundingClientRect()));
  delete offsets[d.id];
}

onMounted(() => {
  if (!props.virtualW || !box.value) return;
  ro = new ResizeObserver(() => (k.value = box.value!.clientWidth / props.virtualW!));
  ro.observe(box.value);
});
onBeforeUnmount(() => ro?.disconnect());
</script>

<template>
  <div ref="box" class="box" :class="{ virtual: !!virtualW }" :style="virtualW ? { aspectRatio: `${virtualW} / ${virtualH}` } : {}">
    <div ref="layer" class="layer" :class="{ edit: editable }" :style="inner" @pointermove="move" @pointerup="up">
      <div v-for="w in items" :key="w.id" class="slot" :class="{ sel: editable && selected === w.id }" :style="style(w)" @pointerdown="down($event, w)">
        <component :is="comp(w.id)" :cfg="w" />
      </div>
      <slot />
    </div>
  </div>
</template>

<style scoped>
.box { position: relative; width: 100%; height: 100%; overflow: hidden; }
.box.virtual { height: auto; border: 1px solid var(--border, #1d1d21); background: linear-gradient(180deg, #1a1f27 0%, #10141a 45%, #0b0d10 100%); }
.layer { position: absolute; inset: 0; transform-origin: top left; }
.box.virtual .layer { inset: auto; left: 0; top: 0; }
.slot { position: absolute; }
.layer.edit .slot { cursor: move; outline: 1px dashed rgba(200, 16, 46, .7); }
.layer.edit .slot.sel { outline: 2px solid #c8102e; }
.layer.edit { background-image: linear-gradient(rgba(200, 16, 46, .07) 1px, transparent 1px), linear-gradient(90deg, rgba(200, 16, 46, .07) 1px, transparent 1px); background-size: 33.333% 33.333%; }
</style>
