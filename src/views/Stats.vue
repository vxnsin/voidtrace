<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { api, fmtTs, tradePrompt, type Datapoint, type RelicRun, type Trade } from "../api";

const points = ref<Datapoint[]>([]);
const trades = ref<Trade[]>([]);
const runs = ref<RelicRun[]>([]);
const dp = ref({ platinum: 0, ducats: 0, credits: 0 });
const nt = ref({ item: "", quantity: 1, platinum: 0, direction: "sell" as "sell" | "buy", partner: "" });

async function load() {
  [points.value, trades.value, runs.value] = await Promise.all([api.datapoints(), api.trades(), api.relicRuns(50)]);
  const last = points.value[0];
  if (last) dp.value = { platinum: last.platinum, ducats: last.ducats, credits: last.credits };
}
async function saveDp() { await api.addDatapoint(dp.value.platinum, dp.value.ducats, dp.value.credits); await load(); }
async function saveTrade() { await api.addTrade(nt.value); nt.value = { item: "", quantity: 1, platinum: 0, direction: "sell", partner: "" }; await load(); }
function fromListing() { tradePrompt.value = { onDone: load }; }
async function del(id: number) { await api.deleteTrade(id); await load(); }

const earned = computed(() => trades.value.filter((t) => t.direction === "sell").reduce((s, t) => s + t.platinum, 0));
const spent = computed(() => trades.value.filter((t) => t.direction === "buy").reduce((s, t) => s + t.platinum, 0));
const sparkline = computed(() => {
  const pts = [...points.value].reverse().slice(-60);
  if (pts.length < 2) return "";
  const max = Math.max(...pts.map((p) => p.platinum), 1);
  return pts.map((p, i) => `${(i / (pts.length - 1)) * 300},${60 - (p.platinum / max) * 56}`).join(" ");
});
onMounted(load);
</script>

<template>
  <h1>Stats & Trading Analytics</h1>
  <div class="grid cols-4">
    <div class="card"><div class="stat plat">{{ earned }}p<small>earned from sells</small></div></div>
    <div class="card"><div class="stat">{{ spent }}p<small>spent on buys</small></div></div>
    <div class="card"><div class="stat">{{ trades.length }}<small>trades logged</small></div></div>
    <div class="card"><div class="stat">{{ runs.length }}<small>relic runs recorded</small></div></div>
  </div>

  <h2>Platinum over time</h2>
  <div class="card row" style="gap:20px">
    <svg width="300" height="60" v-if="sparkline"><polyline :points="sparkline" fill="none" stroke="var(--accent)" stroke-width="2" /></svg>
    <span v-else class="muted">Log a datapoint to start the graph.</span>
    <label>Plat <input type="number" v-model.number="dp.platinum" style="width:90px" /></label>
    <label>Ducats <input type="number" v-model.number="dp.ducats" style="width:90px" /></label>
    <label>Credits <input type="number" v-model.number="dp.credits" style="width:120px" /></label>
    <button class="btn primary" @click="saveDp">Log now</button>
  </div>

  <h2>Trades</h2>
  <div class="card row">
    <select v-model="nt.direction"><option value="sell">Sold</option><option value="buy">Bought</option></select>
    <input v-model="nt.item" placeholder="Item" style="width:220px" />
    <input type="number" v-model.number="nt.quantity" style="width:60px" />
    <input type="number" v-model.number="nt.platinum" placeholder="plat" style="width:80px" />
    <input v-model="nt.partner" placeholder="Partner" style="width:140px" />
    <button class="btn primary" @click="saveTrade">Add</button>
    <button class="btn" @click="fromListing">From a listing…</button>
    <span class="muted">A finished in-game trade opens a dialog: pick the listing and it is closed on warframe.market, logged here and taken off the inventory.</span>
  </div>
  <table style="margin-top:8px">
    <thead><tr><th>When</th><th></th><th>Item</th><th>Qty</th><th>Plat</th><th>Partner</th><th></th></tr></thead>
    <tbody>
      <tr v-for="t in trades" :key="t.id">
        <td class="muted">{{ fmtTs(t.ts) }}</td><td>{{ t.direction === "sell" ? "→" : "←" }}</td><td>{{ t.item }}</td><td>{{ t.quantity }}</td>
        <td class="plat">{{ t.platinum }}</td><td>{{ t.partner }}</td><td><button class="btn danger" @click="del(t.id)">✕</button></td>
      </tr>
    </tbody>
  </table>

  <h2>Relic runs</h2>
  <table>
    <thead><tr><th>When</th><th>Relic</th><th>Rewards seen</th></tr></thead>
    <tbody>
      <tr v-for="r in runs" :key="r.id"><td class="muted">{{ fmtTs(r.ts) }}</td><td>{{ r.relic || "?" }}</td><td>{{ r.rewards.join(" · ") }}</td></tr>
    </tbody>
  </table>
</template>
