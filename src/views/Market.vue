<script setup lang="ts">
import { onMounted, ref } from "vue";
import { api, type MarketItem, type Order, type PriceSummary, type RichStatus, type Status, type WsEvent } from "../api";
import ItemLink from "../components/ItemLink.vue";

const wfmStatus = ref<RichStatus | null>(null);
const wsState = ref("connecting");
const autoActivity = ref(true);
async function setStatus(s: string) {
  try { await api.wfmSetStatus(s, wfmStatus.value?.activity?.type, wfmStatus.value?.activity?.details ?? undefined); } catch (e) { err.value = String(e); }
}
async function toggleAuto() {
  autoActivity.value = !autoActivity.value;
  await api.setSetting("wfm_auto_activity", autoActivity.value ? "1" : "0");
}

const props = defineProps<{ status: Status | null }>();
const emit = defineEmits(["refresh"]);
const email = ref(""); const password = ref(""); const who = ref(""); const err = ref("");
const mine = ref<Order[]>([]);
const q = ref(""); const results = ref<MarketItem[]>([]); const price = ref<PriceSummary | null>(null); const book = ref<Order[]>([]);
const itemsById = ref<Record<string, MarketItem>>({});

async function login() {
  err.value = "";
  try { who.value = await api.wfmLogin(email.value, password.value); password.value = ""; emit("refresh"); await loadMine(); } catch (e) { err.value = String(e); }
}
async function logout() { await api.wfmLogout(); who.value = ""; mine.value = []; emit("refresh"); }
async function loadMine() {
  if (!props.status?.wfm_logged_in && !who.value) return;
  try { mine.value = await api.wfmMyOrders(); } catch (e) { err.value = String(e); return; }
  const missing = [...new Set(mine.value.map((o) => o.itemId))].filter((id) => !itemsById.value[id]);
  if (missing.length) Object.assign(itemsById.value, await api.marketItemsById(missing));
}
const itemName = (o: Order) => itemsById.value[o.itemId]?.name ?? o.itemId;
async function search() { results.value = q.value.length > 1 ? await api.marketSearch(q.value, 10) : []; }
async function show(i: MarketItem) {
  results.value = []; q.value = i.name;
  [price.value, book.value] = await Promise.all([api.marketPrice(i.slug), api.marketOrders(i.slug)]);
  book.value = book.value.filter((o) => o.visible && o.user && (o.user.status === "ingame" || o.user.status === "online")).sort((a, b) => a.platinum - b.platinum);
}
async function upd(o: Order, patch: Partial<Order>) {
  try { await api.wfmUpdateOrder(o.id, patch.platinum ?? o.platinum, patch.quantity ?? o.quantity, patch.visible ?? o.visible); await loadMine(); } catch (e) { err.value = String(e); }
}
async function del(o: Order) { try { await api.wfmDeleteOrder(o.id); await loadMine(); } catch (e) { err.value = String(e); } }
onMounted(async () => {
  who.value = (await api.getSetting("wfm_name")) ?? "";
  autoActivity.value = (await api.getSetting("wfm_auto_activity")) !== "0";
  wfmStatus.value = await api.wfmStatus();
  if (wfmStatus.value) wsState.value = "connected";
  api.on<WsEvent>("wfm-ws", (e) => {
    if (e.kind === "status") wfmStatus.value = { status: e.status!, statusSetAt: e.statusSetAt, statusUntil: e.statusUntil, activity: e.activity };
    if (e.kind === "connected" || e.kind === "signed_in") wsState.value = "connected";
    if (e.kind === "disconnected") wsState.value = `offline: ${e.reason}`;
  });
  await loadMine();
});
</script>

<template>
  <h1>Warframe.Market</h1>
  <p v-if="err" class="error">{{ err }}</p>
  <div class="card" v-if="!status?.wfm_logged_in">
    <div class="row">
      <input v-model="email" placeholder="warframe.market email" style="width:240px" />
      <input v-model="password" type="password" placeholder="password" style="width:200px" @keyup.enter="login" />
      <button class="btn primary" @click="login">Log in</button>
      <span class="muted">Credentials are sent only to api.warframe.market; the session token is stored locally.</span>
    </div>
  </div>
  <div class="card row" v-else style="justify-content:space-between">
    <span>Logged in as <b>{{ who }}</b> <span class="muted">· socket {{ wsState }}</span></span>
    <div class="row">
      <span class="muted">status</span>
      <div class="modes">
        <button v-for="s in ['ingame', 'online', 'invisible']" :key="s" :class="{ on: wfmStatus?.status === s }" @click="setStatus(s)">{{ s }}</button>
      </div>
      <span class="muted" v-if="wfmStatus?.activity">{{ wfmStatus.activity.type.toLowerCase().replace('_', ' ') }}<template v-if="wfmStatus.activity.details"> · {{ wfmStatus.activity.details }}</template></span>
      <label class="pill" :title="'Mirror login / mission state from EE.log to your market status'"><input type="checkbox" :checked="autoActivity" @change="toggleAuto" /> auto from game</label>
      <button class="btn" @click="loadMine">Reload</button><button class="btn" @click="logout">Log out</button>
    </div>
  </div>

  <h2>My listings</h2>
  <table v-if="mine.length">
    <thead><tr><th>Type</th><th>Item</th><th>Plat</th><th>Qty</th><th>Visible</th><th></th></tr></thead>
    <tbody>
      <tr v-for="o in mine" :key="o.id">
        <td>{{ o.type }}</td>
        <td>
          <ItemLink v-if="itemsById[o.itemId]" :slug="itemsById[o.itemId].slug" :name="itemsById[o.itemId].name" :icon="itemsById[o.itemId].icon" :ducats="itemsById[o.itemId].ducats" />
          <template v-else>{{ itemName(o) }}</template>
          <span v-if="o.rank != null" class="muted"> · r{{ o.rank }}</span>
        </td>
        <td><input type="number" :value="o.platinum" style="width:80px" @change="upd(o, { platinum: Number(($event.target as HTMLInputElement).value) })" /></td>
        <td><input type="number" :value="o.quantity" style="width:70px" @change="upd(o, { quantity: Number(($event.target as HTMLInputElement).value) })" /></td>
        <td><input type="checkbox" :checked="o.visible" @change="upd(o, { visible: ($event.target as HTMLInputElement).checked })" /></td>
        <td><button class="btn danger" @click="del(o)">✕</button></td>
      </tr>
    </tbody>
  </table>
  <p v-else class="muted">No listings (or not logged in).</p>

  <h2>Price check</h2>
  <div class="row" style="position:relative">
    <input v-model="q" @input="search" placeholder="Item name" style="width:360px" />
    <div v-if="results.length" class="card" style="position:absolute;top:40px;left:0;z-index:5;width:360px;padding:4px">
      <div v-for="r in results" :key="r.id" style="padding:6px 8px;cursor:pointer" @click="show(r)">{{ r.name }}</div>
    </div>
  </div>
  <div v-if="price" class="grid cols-4" style="margin-top:12px">
    <div class="card"><div class="stat plat">{{ price.sell ?? "–" }}p<small>lowest sell (online)</small></div></div>
    <div class="card"><div class="stat">{{ price.buy ?? "–" }}p<small>highest buy</small></div></div>
    <div class="card"><div class="stat ducat">{{ price.ducats || "–" }}<small>ducats</small></div></div>
    <div class="card"><div class="stat">{{ price.sell_count }}<small>sellers online</small></div></div>
  </div>
  <table v-if="book.length" style="margin-top:12px">
    <thead><tr><th>Type</th><th>Plat</th><th>Qty</th><th>Seller</th><th>Status</th><th>Rep</th></tr></thead>
    <tbody>
      <tr v-for="o in book.slice(0, 30)" :key="o.id"><td>{{ o.type }}</td><td class="plat">{{ o.platinum }}</td><td>{{ o.quantity }}</td><td>{{ o.user?.ingameName }}</td><td class="muted">{{ o.user?.status }}</td><td>{{ o.user?.reputation }}</td></tr>
    </tbody>
  </table>
</template>
