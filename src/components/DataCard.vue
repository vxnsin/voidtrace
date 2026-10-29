<script setup lang="ts">
import { ref } from "vue";
import { open, save } from "@tauri-apps/plugin-dialog";
import { dataApi } from "../api";

// Export everything VoidTrace recorded (JSON or CSV), and import a JSON export
// on another PC.
const msg = ref("");
const err = ref("");
const stamp = () => new Date().toISOString().slice(0, 10);

async function doExport(format: "json" | "csv") {
  msg.value = ""; err.value = "";
  const path = await save({
    defaultPath: `voidtrace-${stamp()}.${format}`,
    filters: [format === "json" ? { name: "JSON", extensions: ["json"] } : { name: "CSV", extensions: ["csv"] }],
  });
  if (!path) return;
  try {
    const files = await dataApi.exportData(path, format);
    msg.value = files.length > 1 ? `Saved ${files.length} files next to ${files[0]}` : `Saved ${files[0]}`;
  } catch (e) { err.value = String(e); }
}
async function doImport() {
  msg.value = ""; err.value = "";
  const path = await open({ multiple: false, filters: [{ name: "VoidTrace export", extensions: ["json"] }] });
  if (!path || Array.isArray(path)) return;
  try {
    const n = await dataApi.importData(path);
    msg.value = `Imported ${n.trades} trades, ${n.history} history points, ${n.relic_runs} relic runs` + (n.inventory ? `, ${n.inventory} inventory items` : "") + ". Already known rows were skipped.";
  } catch (e) { err.value = String(e); }
}
</script>

<template>
  <div class="card">
    <h2 style="margin-top:0">Your data</h2>
    <p class="muted" style="margin-top:0">Trades, plat / ducat / credit history, relic runs and the inventory. JSON keeps everything and can be imported again; CSV is one file per table for spreadsheets.</p>
    <div class="row">
      <button class="btn" @click="doExport('json')">Export JSON</button>
      <button class="btn" @click="doExport('csv')">Export CSV</button>
      <button class="btn" @click="doImport">Import JSON…</button>
    </div>
    <p v-if="msg" class="ok" style="margin:8px 0 0">{{ msg }}</p>
    <p v-if="err" class="error" style="margin:8px 0 0">{{ err }}</p>
  </div>
</template>
