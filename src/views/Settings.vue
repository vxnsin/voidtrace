<script setup lang="ts">
import { onMounted, ref } from "vue";
import { api, type Status } from "../api";

defineProps<{ status: Status | null }>();
const emit = defineEmits(["refresh"]);
const overlay = ref(true);
const memorySync = ref(false);
const introMode = ref<"full" | "quick" | "off">("full");
function setIntro(m: "full" | "quick" | "off") {
  introMode.value = m;
  try { localStorage.setItem("intro_mode", m); } catch {}
}
const rawLog = ref(false);
const crop = ref({ x: 0.18, y: 0.4, w: 0.64, h: 0.14 });
const patterns = ref("");
const msg = ref("");
const busy = ref(false);

async function load() {
  try { introMode.value = (localStorage.getItem("intro_mode") as typeof introMode.value) || "full"; } catch {}
  overlay.value = (await api.getSetting("overlay_enabled")) !== "0";
  memorySync.value = (await api.getSetting("memory_sync")) === "1";
  rawLog.value = (await api.getSetting("log_raw")) === "1";
  const c = await api.getSetting("ocr_crop"); if (c) crop.value = JSON.parse(c);
  patterns.value = (await api.getSetting("log_patterns")) ?? "";
}
async function save() {
  await api.setSetting("overlay_enabled", overlay.value ? "1" : "0");
  await api.setSetting("memory_sync", memorySync.value ? "1" : "0");
  await api.setSetting("log_raw", rawLog.value ? "1" : "0");
  await api.setSetting("ocr_crop", JSON.stringify(crop.value));
  if (patterns.value.trim()) await api.setSetting("log_patterns", patterns.value);
  msg.value = "Saved. Log pattern changes apply after restart.";
}
async function refresh() {
  busy.value = true; msg.value = "";
  try { await api.refreshData(); msg.value = "Game data and market catalogue refreshed."; emit("refresh"); } catch (e) { msg.value = String(e); } finally { busy.value = false; }
}
async function testOcr() { msg.value = "Capturing in 3s, tab to the reward screen…"; setTimeout(async () => { await api.ocrNow(); msg.value = "OCR done — check the overlay."; }, 3000); }
onMounted(load);
</script>

<template>
  <h1>Settings</h1>
  <div class="card" style="margin-bottom:12px">
    <h2 style="margin-top:0">Startup</h2>
    <div class="row">
      <span class="muted">Intro sequence:</span>
      <div class="modes">
        <button :class="{ on: introMode === 'full' }" @click="setIntro('full')">Full 8s</button>
        <button :class="{ on: introMode === 'quick' }" @click="setIntro('quick')">Quick 2s</button>
        <button :class="{ on: introMode === 'off' }" @click="setIntro('off')">Off</button>
      </div>
      <span class="muted">Esc skips it any time. Applies on next start.</span>
    </div>
  </div>
  <div class="card">
    <h2 style="margin-top:0">Overlay</h2>
    <label class="row"><input type="checkbox" v-model="overlay" /> Show relic reward overlay automatically</label>
    <p class="muted">Warframe must run in <b>Borderless Fullscreen</b> for any overlay to be visible. Game language must be English for OCR matching.</p>
    <div class="row">
      <span class="muted">Reward row crop (fractions of screen):</span>
      <label>x <input type="number" step="0.01" v-model.number="crop.x" style="width:70px" /></label>
      <label>y <input type="number" step="0.01" v-model.number="crop.y" style="width:70px" /></label>
      <label>w <input type="number" step="0.01" v-model.number="crop.w" style="width:70px" /></label>
      <label>h <input type="number" step="0.01" v-model.number="crop.h" style="width:70px" /></label>
      <button class="btn" @click="testOcr">Test OCR now</button>
      <button class="btn" @click="api.overlayPreview()">Preview overlay</button>
      <button class="btn" @click="api.overlayToggle()">Toggle overlay</button>
    </div>
  </div>

  <div class="card" style="margin-top:12px">
    <h2 style="margin-top:0">Inventory sync</h2>
    <label class="row"><input type="checkbox" v-model="memorySync" /> Read the inventory from the game process automatically (after login and after each mission)</label>
    <p class="muted" style="margin:6px 0 0">
      Read-only scan of Warframe's memory for the inventory document the game already holds — no injection, no writes, no offsets.
      This is what Overwolf-based companions do too, but outside Overwolf it is not covered by DE's third-party policy. Your call.
      Warframe and VoidTrace must run at the same privilege level (don't start either as administrator).
    </p>
  </div>

  <div class="card" style="margin-top:12px">
    <h2 style="margin-top:0">Game log</h2>
    <p class="muted">Reading <code>{{ status?.ee_log }}</code> — {{ status?.ee_log_exists ? "found" : "not found (start Warframe once)" }}</p>
    <label class="row"><input type="checkbox" v-model="rawLog" /> Forward every log line as an event (debugging)</label>
    <p class="muted">Regex overrides (JSON with keys logged_in, mission_start, mission_end, reward_screen, reward_done, trade_complete, relic_opened). Leave empty for defaults.</p>
    <textarea v-model="patterns" rows="4" style="width:100%;background:var(--bg3);border:1px solid var(--border);border-radius:6px;color:inherit;padding:8px;font-family:monospace"></textarea>
  </div>

  <div class="card row" style="margin-top:12px">
    <button class="btn primary" @click="save">Save</button>
    <button class="btn" :disabled="busy" @click="refresh">{{ busy ? "Refreshing…" : "Refresh game data" }}</button>
    <span :class="msg.startsWith('Saved') || msg.includes('refreshed') ? 'ok' : 'muted'">{{ msg }}</span>
  </div>
</template>
