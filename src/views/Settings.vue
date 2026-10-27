<script setup lang="ts">
import { onMounted, ref } from "vue";
import { api, type Status } from "../api";
import { getVersion } from "@tauri-apps/api/app";
import { checkForUpdate, installUpdate, updates } from "../updates";
import ThemeCard from "../components/ThemeCard.vue";
import DiscordCard from "../components/DiscordCard.vue";

defineProps<{ status: Status | null }>();
const emit = defineEmits(["refresh"]);
const memorySync = ref(false);
const introMode = ref<"full" | "quick" | "off">("full");
function setIntro(m: "full" | "quick" | "off") {
  introMode.value = m;
  try { localStorage.setItem("intro_mode", m); } catch {}
}
const launchWithGame = ref(false);
async function rerunSetup() { await api.setSetting("setup_done", "0"); window.location.reload(); }
const version = ref("");

// Bug report: description + logs + settings (tokens removed) as a zip on the desktop.
const bugText = ref("");
const bugGame = ref(true);
const bugPath = ref("");
const bugErr = ref("");
const bugBusy = ref(false);
async function saveReport() {
  bugBusy.value = true; bugErr.value = ""; bugPath.value = "";
  try { bugPath.value = await api.exportBugReport(bugText.value, bugGame.value); }
  catch (e) { bugErr.value = String(e); }
  finally { bugBusy.value = false; }
}
function reportOnGitHub() {
  const first = bugText.value.trim().split(/\r?\n/)[0].slice(0, 70);
  const body = [
    "**What happened**",
    bugText.value.trim() || "…",
    "",
    `**Version** ${version.value}`,
    "",
    "_Attach the zip from Settings → Problems & logs (drag it into this box)._",
  ].join("\n");
  api.openUrl(`https://github.com/vxnsin/voidtrace/issues/new?labels=bug&title=${encodeURIComponent(first || "Bug: ")}&body=${encodeURIComponent(body)}`);
}
const autoUpdate = ref(true);
const checked = ref(false);
async function toggleAutoUpdate() { autoUpdate.value = !autoUpdate.value; await api.setSetting("update_auto", autoUpdate.value ? "1" : "0"); }
async function checkNow() { await checkForUpdate(); checked.value = true; }
onMounted(async () => { version.value = await getVersion(); autoUpdate.value = (await api.getSetting("update_auto")) !== "0"; });
async function toggleLaunch() {
  try {
    await api.setLaunchWithGame(!launchWithGame.value);
    launchWithGame.value = !launchWithGame.value;
  } catch (e) { msg.value = String(e); }
}
const rawLog = ref(false);
const crop = ref({ x: 0.18, y: 0.4, w: 0.64, h: 0.14 });
const patterns = ref("");
const msg = ref("");
const busy = ref(false);

async function load() {
  try { introMode.value = (localStorage.getItem("intro_mode") as typeof introMode.value) || "full"; } catch {}
  memorySync.value = (await api.getSetting("memory_sync")) === "1";
  launchWithGame.value = (await api.getSetting("launch_with_game")) === "1";
  rawLog.value = (await api.getSetting("log_raw")) === "1";
  const c = await api.getSetting("ocr_crop"); if (c) crop.value = JSON.parse(c);
  patterns.value = (await api.getSetting("log_patterns")) ?? "";
}
async function save() {
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
  <ThemeCard style="margin-bottom:12px" />
  <DiscordCard style="margin-bottom:12px" />
  <div class="card" style="margin-bottom:12px">
    <h2 style="margin-top:0">Startup</h2>
    <div class="row">
      <span class="muted">Intro sequence:</span>
      <div class="modes">
        <button :class="{ on: introMode === 'full' }" @click="setIntro('full')">Full 5s</button>
        <button :class="{ on: introMode === 'quick' }" @click="setIntro('quick')">Quick 2s</button>
        <button :class="{ on: introMode === 'off' }" @click="setIntro('off')">Off</button>
      </div>
      <span class="muted">Esc skips it any time. Applies on next start.</span>
    </div>
    <div class="row" style="margin-top:10px"><button class="btn" @click="rerunSetup">Run setup again</button></div>
    <label class="row" style="margin-top:10px"><input type="checkbox" :checked="launchWithGame" @change="toggleLaunch" /> Launch with Warframe</label>
    <p class="muted" style="margin:6px 0 0">Starts hidden in the tray when you sign in to Windows and opens as soon as Warframe starts. Closing the window keeps it in the tray; quit from the tray icon.</p>
  </div>
  <div class="card" style="margin-bottom:12px">
    <h2 style="margin-top:0">Problems &amp; logs</h2>
    <p class="muted" style="margin-top:0">Something broke? Write what you did and what happened, save the report and send the zip — or attach it to a GitHub issue.</p>
    <textarea v-model="bugText" rows="4" placeholder="What did you do, what did you expect, what happened instead?" class="bugtext"></textarea>
    <label class="row" style="margin-top:8px"><input type="checkbox" v-model="bugGame" /> Include the end of Warframe's EE.log (game events; contains your in-game name)</label>
    <div class="row" style="margin-top:10px">
      <button class="btn primary" :disabled="bugBusy" @click="saveReport">{{ bugBusy ? "Saving…" : "Save bug report" }}</button>
      <button class="btn" @click="reportOnGitHub">Report on GitHub</button>
      <button class="btn" @click="api.openLogFolder()">Open log folder</button>
    </div>
    <p v-if="bugPath" class="ok">Saved: {{ bugPath }}</p>
    <p v-if="bugErr" class="error">{{ bugErr }}</p>
  </div>
  <div class="card" style="margin-bottom:12px">
    <h2 style="margin-top:0">Updates</h2>
    <div class="row">
      <span class="muted">Installed v{{ version }}</span>
      <button class="btn" :disabled="updates.checking" @click="checkNow">{{ updates.checking ? "Checking…" : "Check now" }}</button>
      <button v-if="updates.available" class="btn primary" :disabled="updates.installing" @click="installUpdate">{{ updates.installing ? `Installing ${Math.round(updates.progress * 100)}%` : `Install v${updates.available.version} & restart` }}</button>
      <span v-else-if="checked && !updates.error" class="ok">Up to date.</span>
      <span v-if="updates.error" class="error">{{ updates.error }}</span>
    </div>
    <pre v-if="updates.available?.body" class="notes">{{ updates.available.body }}</pre>
    <label class="row" style="margin-top:10px"><input type="checkbox" :checked="autoUpdate" @change="toggleAutoUpdate" /> Check GitHub for new releases on start and every few hours</label>
  </div>
  <div class="card">
    <h2 style="margin-top:0">Overlay</h2>
    <p class="muted" style="margin-top:0">The relic reward widget is added or removed in the in-game editor; reading the reward screen runs only while it is on screen.</p>
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

<style scoped>
.bugtext { width: 100%; background: var(--bg3); border: 1px solid var(--border); color: inherit; padding: 8px; font: inherit; resize: vertical; }
.notes { white-space: pre-wrap; margin: 10px 0 0; color: var(--fg2); font: inherit; max-height: 200px; overflow: auto; }
</style>
