<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { api, fmtTs, listingPrompt, type InvRow, type MarketItem, type PriceSummary, type Status, type SyncResult } from "../api";
import ItemLink from "../components/ItemLink.vue";
import ItemIcon from "../components/ItemIcon.vue";
import InventorySets from "../components/InventorySets.vue";
import ListingCheck from "../components/ListingCheck.vue";

defineProps<{ status: Status | null }>();

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
const onlyDup = ref(false);
const onlyVaulted = ref(false);
const listed = ref<Set<string>>(new Set());
const listedFilter = ref<"all" | "listed" | "unlisted">("all");
const minPlat = ref(0);
const shown = computed(() => rows.value.filter((r) => {
  const p = prices.value[r.slug];
  if (onlyDup.value && r.quantity < 2) return false;
  if (onlyVaulted.value && !p?.vaulted) return false;
  if (listedFilter.value === "listed" && !listed.value.has(r.slug)) return false;
  if (listedFilter.value === "unlisted" && listed.value.has(r.slug)) return false;
  if (minPlat.value && (p?.sell ?? 0) < minPlat.value) return false;
  return true;
}));

const setsRef = ref<InstanceType<typeof InventorySets> | null>(null);
async function load() {
  rows.value = await api.inventory();
  setsRef.value?.load();
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
function startListing(r?: InvRow, mode: "sell" | "buy" = "sell") {
  listingPrompt.value = { mode, slug: r?.slug };
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
    <template v-if="status?.wfm_logged_in">
      <button class="btn" style="margin-left:auto" @click="startListing()">+ Sell on WFM</button>
      <button class="btn" @click="startListing(undefined, 'buy')">+ Buy on WFM</button>
    </template>
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
        <span class="row" style="gap:10px"><ItemIcon :icon="r.icon" /><span>{{ r.name }}</span></span><span class="ducat">{{ r.ducats || "" }}</span>
      </div>
    </div>
  </div>
  <p v-if="msg" :class="/^(Listed|Synced|Auto-synced)/.test(msg) ? 'ok' : 'error'">{{ msg }}</p>


  <div class="row" style="margin-top:12px">
    <label class="row"><input type="checkbox" v-model="onlyDup" /> duplicates</label>
    <label class="row"><input type="checkbox" v-model="onlyVaulted" /> vaulted</label>
    <div class="modes"><button v-for="[k, l] in ([['all', 'all'], ['listed', 'listed'], ['unlisted', 'not listed']] as const)" :key="k" :class="{ on: listedFilter === k }" @click="listedFilter = k">{{ l }}</button></div>
    <label class="row muted">min <input type="number" min="0" v-model.number="minPlat" style="width:64px" />p</label>
    <span class="muted">{{ shown.length }} of {{ rows.length }}</span>
  </div>
  <table style="margin-top:12px">
    <thead><tr><th>Item</th><th>Qty</th><th>Sell</th><th>Buy</th><th>Ducats</th><th></th></tr></thead>
    <tbody>
      <tr v-for="r in shown" :key="r.slug">
        <td><ItemLink :slug="r.slug" :name="r.name" :ducats="prices[r.slug]?.ducats" /></td>
        <td><input type="number" :value="r.quantity" style="width:70px" @change="set(r, Number(($event.target as HTMLInputElement).value))" /></td>
        <td class="plat">{{ prices[r.slug]?.sell ?? "–" }}</td>
        <td>{{ prices[r.slug]?.buy ?? "–" }}</td>
        <td class="ducat">{{ prices[r.slug]?.ducats || "–" }}</td>
        <td class="row">
          <button class="btn" v-if="status?.wfm_logged_in" @click="startListing(r)">List on WFM</button>
          <button class="btn danger" @click="set(r, 0)">✕</button>
        </td>
      </tr>
    </tbody>
  </table>
  <h2>Sets</h2>
  <InventorySets ref="setsRef" :logged-in="!!status?.wfm_logged_in" />

  <template v-if="status?.wfm_logged_in">
    <h2>Listings</h2>
    <ListingCheck :rows="rows" @listed="(s) => (listed = s)" />
  </template>
</template>
