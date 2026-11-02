<script setup lang="ts">
import { onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { api } from "../api";

// Phone / tablet dashboard: a read-only page on the local network. The QR
// code holds the address with a token that changes every start.
type DashInfo = { running: boolean; url: string | null; qr: string | null };
const info = ref<DashInfo>({ running: false, url: null, qr: null });
const err = ref("");
const busy = ref(false);
const port = ref(47811);

async function start() {
  busy.value = true; err.value = "";
  try {
    await api.setSetting("dashboard_port", String(port.value || 47811));
    info.value = await invoke<DashInfo>("dashboard_start");
  } catch (e) { err.value = String(e); } finally { busy.value = false; }
}
async function stop() { info.value = await invoke<DashInfo>("dashboard_stop"); }
const qrUrl = () => (info.value.qr ? `data:image/svg+xml;utf8,${encodeURIComponent(info.value.qr)}` : "");

onMounted(async () => {
  info.value = await invoke<DashInfo>("dashboard_status");
  port.value = Number(await api.getSetting("dashboard_port")) || 47811;
});
</script>

<template>
  <div class="card">
    <h2 style="margin-top:0">Phone dashboard</h2>
    <p class="muted" style="margin-top:0">A second screen: cycles, fissures, relic rewards while you pick, today's trades and your listings, live in the phone's browser. Phone and PC need the same network. Off until you start it; it stops when VoidTrace quits.</p>
    <div class="row">
      <button v-if="!info.running" class="btn primary" :disabled="busy" @click="start">{{ busy ? "Starting…" : "Start phone dashboard" }}</button>
      <button v-else class="btn" @click="stop">Stop</button>
      <label class="row muted">port <input type="number" min="1024" max="65535" v-model.number="port" style="width:80px" :disabled="info.running" /></label>
    </div>
    <div v-if="info.running" class="dash">
      <img :src="qrUrl()" width="200" height="200" alt="QR code" class="qr" />
      <div>
        <p style="margin-top:0">Scan it with the phone camera, or open:</p>
        <code class="url">{{ info.url }}</code>
        <p class="muted">The link changes every time you start the dashboard; old links stop working. Windows may ask once whether VoidTrace may accept connections on private networks; allow it for the phone to connect.</p>
      </div>
    </div>
    <p v-if="err" class="error">{{ err }}</p>
  </div>
</template>

<style scoped>
.dash { display: flex; gap: 20px; align-items: flex-start; margin-top: 12px; flex-wrap: wrap; }
.qr { filter: none; opacity: 1; background: #fff; padding: 8px; }
.url { word-break: break-all; color: var(--accent2); }
</style>