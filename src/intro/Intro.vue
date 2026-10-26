<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from "vue";

const emit = defineEmits<{ done: [] }>();
const props = defineProps<{ backendOk: boolean }>();

type Mode = "full" | "quick" | "off";
const DURATION: Record<Mode, number> = { full: 8000, quick: 2000, off: 0 };
const mode = ref<Mode>("full");
try { mode.value = (localStorage.getItem("intro_mode") as Mode) || "full"; } catch {}
const finished = ref(false);
const canvas = ref<HTMLCanvasElement | null>(null);

let raf = 0;
let start = 0;
let ctx: CanvasRenderingContext2D;
let cols = 0, rows = 0;
const CW = 8, CH = 14;
const RAMP = " .,:;!?jJ$8@#";

function setMode(m: Mode) {
  mode.value = m;
  try { localStorage.setItem("intro_mode", m); } catch {}
  if (m === "off") finish();
}
function finish() {
  cancelAnimationFrame(raf);
  emit("done");
}
function onKey(e: KeyboardEvent) {
  if (e.key === "Escape") return finish();
  if (finished.value) finish();
}

// Deterministic per-cell noise.
function hash(x: number, y: number, s = 0) {
  let h = (x * 374761393 + y * 668265263 + s * 2246822519) | 0;
  h = Math.imul(h ^ (h >>> 13), 1274126177);
  return ((h ^ (h >>> 16)) >>> 0) / 4294967296;
}

// Lotus emblem: three petals rising from a common base. u = along petal, v = across.
function petal(px: number, py: number, angle: number, len: number, width: number) {
  const c = Math.cos(angle), s = Math.sin(angle);
  const u = -px * s + -py * c; // rotate so the petal points "up" (negative y)
  const v = px * c - py * s;
  if (u < 0 || u > len) return 0;
  const w = width * Math.sin(Math.PI * (u / len)) ** 0.7;
  const d = Math.abs(v) / Math.max(w, 1e-3);
  if (d > 1) return 0;
  // Line drawing: bright outline, faint spine, mostly empty inside.
  if (d > 0.8) return 0.85 + 0.15 * (d - 0.8) / 0.2;
  if (Math.abs(v) < 0.55) return 0.45;
  return hash(Math.round(px), Math.round(py), 5) > 0.82 ? 0.18 : 0;
}

function lotus(px: number, py: number) {
  // px, py in cell units relative to emblem base (y up is negative)
  const a = petal(px, py, 0, 30, 6.5);
  const b = petal(px, py, 0.78, 24, 5);
  const c = petal(px, py, -0.78, 24, 5);
  const d = petal(px, py, 1.35, 15, 3.5);
  const e = petal(px, py, -1.35, 15, 3.5);
  return Math.max(a, b, c, d, e);
}

function frame(now: number) {
  const t = (now - start) / 1000;
  const total = DURATION[mode.value] / 1000;
  const p = total > 0 ? Math.min(1, t / total) : 1;
  const w = canvas.value!.width, h = canvas.value!.height;
  ctx.fillStyle = "#070708";
  ctx.fillRect(0, 0, w, h);
  ctx.font = `${CH - 2}px ${getComputedStyle(document.documentElement).getPropertyValue("--mono")}`;
  ctx.textBaseline = "top";

  const cx = cols / 2, cy = rows * 0.62;
  const reveal = Math.min(1, p * 1.6);         // emblem draws in over the first ~60%
  const rays = Math.min(1, Math.max(0, (p - 0.1) / 0.7));
  const maxR = Math.hypot(cols, rows);

  for (let y = 0; y < rows; y++) {
    for (let x = 0; x < cols; x++) {
      const dx = x - cx, dy = (y - cy) * (CH / CW);
      const r = Math.hypot(dx, dy);
      const ang = Math.atan2(dy, dx);

      // Emblem
      let v = lotus(dx, dy);
      let crimson = false;
      if (v > 0) {
        // reveal from the base upwards
        const rowFrac = (cy - y) / 32;
        if (rowFrac > reveal) v = 0;
        crimson = r < 3.5 || (Math.abs(dx) < 0.6 && dy < -3 && dy > -14);
      }

      // Radial web: 12 spokes + concentric rings, drawn outward over time
      let web = 0;
      const spoke = Math.abs(((ang / (Math.PI / 6)) % 1 + 1) % 1 - 0.5) * 2; // 0 at spoke
      const ring = Math.abs(((r / 9) % 1) - 0.5) * 2;
      if (r / maxR < rays && r > 8) {
        if (spoke > 0.96) web = 0.28;
        if (ring > 0.97 && hash(x, y, 3) > 0.35) web = 0.22;
      }

      // Drifting rain
      const colSeed = hash(x, 0, 7);
      const speed = 6 + colSeed * 10;
      const head = ((t * speed + colSeed * rows * 3) % (rows * 1.6));
      const dist = head - y;
      let rain = 0;
      if (dist > 0 && dist < 12 && hash(x, 1, 9) > 0.55) rain = (1 - dist / 12) * 0.35;

      const n = hash(x, y, Math.floor(t * 8)) ;
      let val = Math.max(v, web, rain);
      if (val <= 0) { if (n > 0.995) val = 0.08; else continue; }
      if (v === 0 && n < 0.08) val *= 0.3; // flicker

      const idx = Math.min(RAMP.length - 1, Math.floor(val * (RAMP.length - 1)));
      const ch = v > 0 && n < 0.03 ? RAMP[Math.floor(hash(x, y, 11) * RAMP.length)] : RAMP[idx];
      if (ch === " ") continue;
      const g = Math.floor(40 + val * 150);
      ctx.fillStyle = crimson && v > 0 ? "#c8102e" : `rgb(${g},${g},${g + 2})`;
      ctx.fillText(ch, x * CW, y * CH);
    }
  }

  if (p >= 1 && !finished.value) finished.value = true;
  raf = requestAnimationFrame(frame);
}

function resize() {
  const c = canvas.value!;
  c.width = window.innerWidth;
  c.height = window.innerHeight;
  cols = Math.ceil(c.width / CW);
  rows = Math.ceil(c.height / CH);
}

onMounted(() => {
  if (mode.value === "off") return finish();
  ctx = canvas.value!.getContext("2d")!;
  resize();
  window.addEventListener("resize", resize);
  window.addEventListener("keydown", onKey);
  start = performance.now();
  raf = requestAnimationFrame(frame);
});
onBeforeUnmount(() => {
  cancelAnimationFrame(raf);
  window.removeEventListener("resize", resize);
  window.removeEventListener("keydown", onKey);
});
</script>

<template>
  <div class="intro" @click="finished && finish()">
    <canvas ref="canvas"></canvas>
    <pre class="meta">VoidTrace/INTRO · sequence
mode ........... {{ mode.toUpperCase() }} · decorative only
emblem ......... lotus · procedural
web ............ 12 spokes · radial
marker ......... core · crimson</pre>
    <div class="cta" :class="{ show: finished }">
      <div class="hint">PRESS ANY KEY TO CONTINUE</div>
      <button class="btn" @click.stop="finish">Open dashboard</button>
    </div>
    <div class="status">BACKEND / {{ props.backendOk ? "AVAILABLE" : "STARTING" }}</div>
    <div class="controls">
      <div class="modes">
        <button :class="{ on: mode === 'full' }" @click.stop="setMode('full')">Full 8s</button>
        <button :class="{ on: mode === 'quick' }" @click.stop="setMode('quick')">Quick 2s</button>
        <button :class="{ on: mode === 'off' }" @click.stop="setMode('off')">Off</button>
      </div>
      <button class="skip" @click.stop="finish">Skip <kbd>Esc</kbd></button>
    </div>
  </div>
</template>

<style scoped>
.intro { position: fixed; inset: 0; background: #070708; z-index: 50; cursor: default; }
canvas { display: block; width: 100%; height: 100%; }
.meta { position: absolute; top: 14px; left: 16px; margin: 0; color: var(--dim); font-size: 10px; line-height: 1.5; letter-spacing: .04em; }
.cta { position: absolute; left: 0; right: 0; bottom: 60px; text-align: center; opacity: 0; transition: opacity .6s; pointer-events: none; }
.cta.show { opacity: 1; pointer-events: auto; }
.hint { color: var(--fg2); font-size: 10px; letter-spacing: .22em; margin-bottom: 10px; }
.status { position: absolute; left: 16px; bottom: 14px; color: var(--muted); font-size: 10px; letter-spacing: .18em; }
.controls { position: absolute; right: 16px; bottom: 10px; display: flex; gap: 8px; align-items: center; }
.modes { display: flex; border: 1px solid var(--dim); }
.modes button { all: unset; cursor: pointer; padding: 4px 10px; font-size: 10px; letter-spacing: .08em; color: var(--muted); }
.modes button.on { background: var(--bg3); color: var(--accent2); }
.skip { all: unset; cursor: pointer; border: 1px solid var(--accent); padding: 4px 10px; font-size: 10px; letter-spacing: .1em; color: var(--accent2); display: flex; gap: 8px; }
.skip:hover { background: var(--accent); color: #000; }
kbd { color: var(--muted); font: inherit; }
.skip:hover kbd { color: #000; }
</style>
