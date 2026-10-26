<script setup lang="ts">
import { onMounted, ref } from "vue";
import { api, fmtTs, type InvRow, type MarketItem, type PriceSummary, type SyncResult } from "../api";
import ItemLink from "../components/ItemLink.vue";

const syncing = ref(false);
const syncedAt = ref<number | null>(null);
async function sync() {
  syncing.value = true; msg.value = "";
  try {
    const r = await api.inventorySync();
    msg.value = `Synced ${r.items_tradeable} tradeable of ${r.items_total} items in ${r.seconds.toFixed(1)}s · ${r.platinum}p · ${r.ducats} ducats`;
    syncedAt.value = Date.now() / 1000;
    await load();
  } catch (e) { msg.value = String(e); } finally { syncing.value = false; }
}

const rows = ref<InvRow[]>([]);
const prices = ref<Record<string, PriceSummary>>({});
const q = ref("");
const results = ref<MarketItem[]>([]);
const msg = ref("");
const listing = ref<{ slug: string; name: string; id: string; plat: number; qty: number } | null>(null);
const items = ref<MarketItem[]>([]);

async function load() {
  rows.value = await api.inventory();
  for (const r of rows.value) api.marketPrice(r.slug).then((p) => (prices.value[r.slug] = p)).catch(() => {});
}
async function search() {
  results.value = q.value.length > 1 ? await api.marketSearch(q.value, 12) : [];
}
async function add(i: MarketItem, d = 1) {
  await api.inventoryAdd(i.slug, i.name, d);
  results.value = [];
  q.value = "";
  await load();
}
async function set(r: InvRow, n: number) {
  await api.inventorySet(r.slug, r.name, n);
  await load();
}
async function startListing(r: InvRow) {
  const hits = await api.marketSearch(r.name, 5);
  const it = hits.find((h) => h.slug === r.slug);
  if (!it) return (msg.value = "Item not on warframe.market");
  const p = prices.value[r.slug];
  listing.value = { slug: r.slug, name: r.name, id: it.id, plat: p?.sell ?? 10, qty: r.quantity };
}
async function postListing() {
  if (!listing.value) return;
  try {
    await api.wfmCreateOrder(listing.value.id, true, listing.value.plat, listing.value.qty);
    msg.value = `Listed ${listing.value.qty}× ${listing.value.name} for ${listing.value.plat}p`;
    listing.value = null;
  } catch (e) { msg.value = String(e); }
}
const total = () => rows.value.reduce((s, r) => s + (prices.value[r.slug]?.sell ?? 0) * r.quantity, 0);
const ducats = () => rows.value.reduce((s, r) => s + (prices.value[r.slug]?.ducats ?? 0) * r.quantity, 0);

onMounted(async () => {
  await load();
  const s = await api.getSetting("inventory_synced_at");
  if (s) syncedAt.value = Number(s);
  api.on<SyncResult>("inventory-synced", (r) => { syncedAt.value = Date.now() / 1000; msg.value = `Auto-synced ${r.items_tradeable} tradeable items`; load(); });
  api.on<string>("inventory-sync-error", (e) => (msg.value = e));
});
</script>

<template>
  <h1>Inventory</h1>
  <div class="row" style="margin-bottom:10px">
    <button class="btn primary" :disabled="syncing" @click="sync">{{ syncing ? "Scanning…" : "Sync from game" }}</button>
    <span class="muted" v-if="syncedAt">last sync {{ fmtTs(syncedAt) }}</span>
    <span class="muted" v-else>never synced — reads the inventory from the running game (enable in Settings for auto-sync)</span>
  </div>
  <div class="grid cols-3" style="margin-bottom:16px">
    <div class="card"><div class="stat">{{ rows.length }}<small>distinct items</small></div></div>
    <div class="card"><div class="stat plat">{{ total() }}p<small>lowest-sell value</small></div></div>
    <div class="card"><div class="stat ducat">{{ ducats() }}<small>ducats</small></div></div>
  </div>
  <div class="row" style="position:relative">
    <input v-model="q" @input="search" placeholder="Add item (e.g. Wisp Prime Systems)" style="width:360px" />
    <div v-if="results.length" class="card" style="position:absolute;top:40px;left:0;z-index:5;width:360px;padding:4px">
      <div v-for="r in results" :key="r.id" class="row" style="padding:6px 8px;cursor:pointer;justify-content:space-between" @click="add(r)">
        <span>{{ r.name }}</span><span class="ducat">{{ r.ducats || "" }}</span>
      </div>
    </div>
  </div>
  <p v-if="msg" :class="/^(Listed|Synced|Auto-synced)/.test(msg) ? 'ok' : 'error'">{{ msg }}</p>

  <div v-if="listing" class="card row" style="margin:12px 0">
    <b>{{ listing.name }}</b>
    <label>Plat <input type="number" v-model.number="listing.plat" style="width:80px" /></label>
    <label>Qty <input type="number" v-model.number="listing.qty" style="width:70px" /></label>
    <button class="btn primary" @click="postListing">Post sell order</button>
    <button class="btn" @click="listing = null">Cancel</button>
  </div>

  <table style="margin-top:12px">
    <thead><tr><th>Item</th><th>Qty</th><th>Sell</th><th>Buy</th><th>Ducats</th><th></th></tr></thead>
    <tbody>
      <tr v-for="r in rows" :key="r.slug">
        <td><ItemLink :slug="r.slug" :name="r.name" :ducats="prices[r.slug]?.ducats" /></td>
        <td><input type="number" :value="r.quantity" style="width:70px" @change="set(r, Number(($event.target as HTMLInputElement).value))" /></td>
        <td class="plat">{{ prices[r.slug]?.sell ?? "–" }}</td>
        <td>{{ prices[r.slug]?.buy ?? "–" }}</td>
        <td class="ducat">{{ prices[r.slug]?.ducats || "–" }}</td>
        <td class="row">
          <button class="btn" @click="startListing(r)">List on WFM</button>
          <button class="btn danger" @click="set(r, 0)">✕</button>
        </td>
      </tr>
    </tbody>
  </table>
</template>
