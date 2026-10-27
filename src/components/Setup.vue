<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { api, type Status, type SyncResult } from "../api";

// First start: walk through what the app needs, test the parts that tend to
// break (log path, privilege level, memory scan) right here instead of later.
const emit = defineEmits<{ done: [] }>();
const steps = ["Welcome", "Game", "warframe.market", "Inventory", "Extras"] as const;
const step = ref(0);
const status = ref<Status | null>(null);
const running = ref(false);
let poll: number | undefined;

const email = ref(""); const password = ref(""); const who = ref(""); const loginErr = ref(""); const busy = ref(false);
const memorySync = ref(false); const sync = ref<SyncResult | null>(null); const syncErr = ref("");
const overlay = ref(true); const launch = ref(false);

async function refresh() {
  status.value = await api.status();
  running.value = await api.gameRunning();
}
async function login() {
  busy.value = true; loginErr.value = "";
  try { who.value = await api.wfmLogin(email.value, password.value); password.value = ""; await refresh(); }
  catch (e) { loginErr.value = String(e); } finally { busy.value = false; }
}
async function setMemorySync(on: boolean) {
  memorySync.value = on;
  await api.setSetting("memory_sync", on ? "1" : "0");
}
async function testSync() {
  busy.value = true; syncErr.value = ""; sync.value = null;
  try { sync.value = await api.inventorySync(); }
  catch (e) {
    const m = String(e);
    syncErr.value = /administrator|cannot open/.test(m)
      ? "Can't read the game. Start Warframe and VoidTrace the same way — neither as administrator — then try again."
      : /not running/.test(m) ? "Warframe isn't running yet. Start it, log in, then test again."
      : /not loaded/.test(m) ? "Game data is still downloading — give it a few seconds."
      : `${m} — log in fully (past the loading screen) and try again; the inventory is only in memory for a while after login.`;
  } finally { busy.value = false; }
}
async function finish() {
  await api.setSetting("overlay_enabled", overlay.value ? "1" : "0");
  if (launch.value) await api.setLaunchWithGame(true).catch(() => {});
  await api.setSetting("setup_done", "1");
  emit("done");
}
async function skip() { await api.setSetting("setup_done", "1"); emit("done"); }

const canNext = computed(() => step.value < steps.length - 1);
onMounted(async () => {
  who.value = (await api.getSetting("wfm_name")) ?? "";
  memorySync.value = (await api.getSetting("memory_sync")) === "1";
  overlay.value = (await api.getSetting("overlay_enabled")) !== "0";
  launch.value = (await api.getSetting("launch_with_game")) === "1";
  await refresh();
  poll = window.setInterval(refresh, 2000);
});
onUnmounted(() => clearInterval(poll));
</script>

<template>
  <div class="setup">
    <div class="box card">
      <div class="top">
        <h1>Setup <span class="muted">{{ step + 1 }} / {{ steps.length }} · {{ steps[step] }}</span></h1>
        <button class="btn" @click="skip">Skip setup</button>
      </div>
      <div class="dots"><i v-for="(s, i) in steps" :key="s" :class="{ on: i <= step }"></i></div>

      <section v-if="step === 0">
        <p>VoidTrace reads Warframe's log, shows relic prices in an overlay, tracks mastery and trades, and manages your warframe.market listings.</p>
        <p class="muted">A minute of setup here saves you chasing sync problems later. Every step can be skipped and changed in Settings.</p>
      </section>

      <section v-if="step === 1">
        <p>Start Warframe and log in. VoidTrace picks up the game on its own.</p>
        <ul class="check">
          <li :class="{ ok: status?.ee_log_exists }">Game log found <span class="muted">{{ status?.ee_log }}</span></li>
          <li :class="{ ok: running }">Warframe is running</li>
          <li class="info">Display mode <b>Borderless Fullscreen</b> — exclusive fullscreen hides every overlay</li>
          <li class="info">Game language <b>English</b> — the reward overlay matches English item names</li>
        </ul>
        <p class="muted" v-if="!status?.ee_log_exists">No log yet: Warframe writes it on first launch. This updates by itself.</p>
      </section>

      <section v-if="step === 2">
        <p v-if="status?.wfm_logged_in">Logged in as <b>{{ who }}</b>. Listings, status and chat are ready.</p>
        <template v-else>
          <p>Log in to manage listings, close orders after trades and show your in-game status.</p>
          <div class="row">
            <input v-model="email" placeholder="warframe.market email" style="width:240px" />
            <input v-model="password" type="password" placeholder="password" style="width:200px" @keyup.enter="login" />
            <button class="btn primary" :disabled="busy" @click="login">{{ busy ? "…" : "Log in" }}</button>
          </div>
          <p class="muted">Sent only to api.warframe.market; the session token stays on this PC.</p>
          <p v-if="loginErr" class="error">{{ loginErr }}</p>
        </template>
      </section>

      <section v-if="step === 3">
        <label class="row"><input type="checkbox" :checked="memorySync" @change="setMemorySync(!memorySync)" /> Read inventory and mastery from the game</label>
        <p class="muted">
          Read-only scan of the inventory the game already holds in memory — no injection, no writes. Overwolf companions read the same data,
          but outside Overwolf it is not covered by DE's third-party policy. Your call. Gives you the inventory, plat/ducat history and mastery
          without an account id.
        </p>
        <div class="row" v-if="memorySync">
          <button class="btn primary" :disabled="busy || !running" @click="testSync">{{ busy ? "Scanning…" : "Test sync now" }}</button>
          <span class="muted" v-if="!running">start Warframe first</span>
        </div>
        <p v-if="sync" class="ok">Found {{ sync.items_tradeable }} tradeable of {{ sync.items_total }} items · {{ sync.platinum }}p · {{ sync.ducats }} ducats in {{ sync.seconds.toFixed(1) }}s</p>
        <p v-if="syncErr" class="error">{{ syncErr }}</p>
      </section>

      <section v-if="step === 4">
        <label class="row"><input type="checkbox" v-model="overlay" /> Relic reward overlay</label>
        <label class="row" style="margin-top:8px"><input type="checkbox" v-model="launch" /> Launch with Warframe <span class="muted">— waits in the tray, opens when the game starts</span></label>
      </section>

      <div class="row steps-nav">
        <button class="btn" v-if="step > 0" @click="step--">Back</button>
        <button class="btn primary" v-if="canNext" @click="step++">{{ step === 0 ? "Start" : "Next" }}</button>
        <button class="btn primary" v-else @click="finish">Done</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.setup { position: fixed; inset: 0; z-index: 60; background: var(--bg); display: flex; align-items: center; justify-content: center; padding: 16px; }
.box { width: min(640px, 100%); }
.top { display: flex; justify-content: space-between; align-items: baseline; gap: 12px; }
.dots { display: flex; gap: 6px; margin: 0 0 18px; }
.dots i { flex: 1; height: 2px; background: var(--border); }
.dots i.on { background: var(--accent); }
section { min-height: 150px; }
.check { list-style: none; padding: 0; margin: 10px 0; display: grid; gap: 8px; }
.check li::before { content: "[ ]"; color: var(--dim); margin-right: 10px; }
.check li.ok::before { content: "[x]"; color: var(--accent2); }
.check li.info::before { content: " · "; }
.steps-nav { justify-content: flex-end; margin-top: 18px; }
</style>
