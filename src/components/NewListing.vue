<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { api, listingPrompt, type InvRow, type MarketItem, type PriceSummary } from "../api";
import ItemIcon from "./ItemIcon.vue";

// Sell: your inventory with the cheapest online sell price, one click lists at
// that price. Buy: search any item and post a buy order.
const mode = ref<"sell" | "buy">(listingPrompt.value?.mode ?? "sell");
const inv = ref<InvRow[]>([]);
const prices = ref<Record<string, PriceSummary>>({});
const listed = ref<Set<string>>(new Set()); // slugs with an open sell order
const filter = ref("");
const q = ref(""); const results = ref<MarketItem[]>([]);
const item = ref<MarketItem | null>(null);
const price = ref<PriceSummary | null>(null);
const plat = ref(0); const qty = ref(1); const rank = ref<number | null>(null);
const busy = ref(false); const err = ref(""); const done = ref("");
let alive = true;

const rows = computed(() => {
  const f = filter.value.toLowerCase();
  return inv.value.filter((r) => r.quantity > 0 && (!f || r.name.toLowerCase().includes(f)));
});
const ranked = computed(() => !!item.value?.tags.some((t) => t === "mod" || t === "arcane_enhancement"));

// warframe.market allows ~3 requests/s; prices come in one after another.
async function loadPrices() {
  for (const r of rows.value) {
    if (!alive) return;
    if (prices.value[r.slug]) continue;
    try { prices.value[r.slug] = await api.marketPrice(r.slug); } catch {}
    await new Promise((res) => setTimeout(res, 350));
  }
}
async function choose(slug: string) {
  const it = await api.marketItem(slug);
  if (!it) { err.value = "Item is not on warframe.market."; return; }
  item.value = it; q.value = it.name; results.value = []; err.value = ""; done.value = "";
  price.value = prices.value[slug] ?? await api.marketPrice(slug).catch(() => null);
  if (price.value) prices.value[slug] = price.value;
  rank.value = null;
  qty.value = mode.value === "sell" ? inv.value.find((r) => r.slug === slug)?.quantity ?? 1 : 1;
  plat.value = mode.value === "sell" ? price.value?.sell ?? price.value?.buy ?? 1 : price.value?.buy ?? price.value?.sell ?? 1;
}
async function search() { results.value = q.value.length > 1 ? await api.marketSearch(q.value, 8) : []; }
function setMode(m: "sell" | "buy") { mode.value = m; item.value = null; price.value = null; q.value = ""; done.value = ""; }

async function post(slug = item.value?.slug, p = plat.value, n = qty.value) {
  if (!slug || p < 1 || n < 1) { err.value = "Pick an item, price and quantity."; return; }
  const it = item.value?.slug === slug ? item.value : await api.marketItem(slug);
  if (!it) return;
  busy.value = true; err.value = "";
  try {
    await api.wfmCreateOrder(it.id, mode.value === "sell", p, n, ranked.value && rank.value != null ? rank.value : undefined);
    if (mode.value === "sell") listed.value.add(slug);
    done.value = `${mode.value === "sell" ? "Selling" : "Buying"} ${n}× ${it.name} for ${p}p`;
    item.value = null; price.value = null; q.value = "";
    listingPrompt.value?.onDone?.();
  } catch (e) { err.value = String(e); } finally { busy.value = false; }
}
function close() { listingPrompt.value = null; }
function key(e: KeyboardEvent) { if (e.key === "Escape") close(); }

onMounted(async () => {
  window.addEventListener("keydown", key);
  const [rowsIn, mine] = await Promise.all([api.inventory(), api.wfmMyOrders().catch(() => [])]);
  inv.value = rowsIn.sort((a, b) => a.name.localeCompare(b.name));
  const ids = [...new Set(mine.filter((o) => o.type === "sell").map((o) => o.itemId))];
  if (ids.length) listed.value = new Set(Object.values(await api.marketItemsById(ids)).map((i) => i.slug));
  if (listingPrompt.value?.slug) await choose(listingPrompt.value.slug);
  loadPrices();
});
onUnmounted(() => { alive = false; window.removeEventListener("keydown", key); });
</script>

<template>
  <div class="backdrop" @click.self="close">
    <div class="card dlg">
      <h1>New listing</h1>
      <div class="row">
        <div class="modes">
          <button :class="{ on: mode === 'sell' }" @click="setMode('sell')">Sell</button>
          <button :class="{ on: mode === 'buy' }" @click="setMode('buy')">Buy</button>
        </div>
        <span class="muted" v-if="mode === 'sell'">From your inventory, priced at the cheapest online seller.</span>
        <span class="muted" v-else>Search any item and post a buy order.</span>
      </div>

      <div class="row" style="margin-top:10px">
        <input v-if="mode === 'sell'" v-model="filter" placeholder="Filter inventory" style="width:260px" />
        <input v-else v-model="q" @input="search" placeholder="Search item" style="width:260px" />
      </div>
      <div v-if="mode === 'buy' && results.length && !item" class="hits">
        <div v-for="r in results" :key="r.id" class="hit" @click="choose(r.slug)">
          <ItemIcon :icon="r.icon" :size="32" /><span>{{ r.name }}</span>
        </div>
      </div>

      <div v-if="item" class="card row pick">
        <ItemIcon :icon="item.icon" :size="48" />
        <b>{{ item.name }}</b>
        <span class="muted" v-if="price">sell {{ price.sell ?? "–" }}p · buy {{ price.buy ?? "–" }}p · {{ price.sell_count }} online</span>
        <label>Plat <input type="number" min="1" v-model.number="plat" style="width:70px" /></label>
        <label>Qty <input type="number" min="1" v-model.number="qty" style="width:60px" /></label>
        <label v-if="ranked">Rank <input type="number" min="0" v-model.number="rank" style="width:50px" /></label>
        <button class="btn primary" :disabled="busy" @click="post()">{{ mode === "sell" ? "Post sell order" : "Post buy order" }}</button>
        <button class="btn" @click="item = null">Cancel</button>
      </div>

      <table v-if="mode === 'sell' && !item" class="inv">
        <thead><tr><th>Item</th><th>Qty</th><th>Lowest</th><th></th></tr></thead>
        <tbody>
          <tr v-for="r in rows" :key="r.slug">
            <td class="pickable" @click="choose(r.slug)">
              <span class="row" style="gap:10px"><ItemIcon :slug="r.slug" /><span>{{ r.name }} <span v-if="listed.has(r.slug)" class="muted">· listed</span></span></span>
            </td>
            <td>{{ r.quantity }}</td>
            <td class="plat">{{ prices[r.slug] ? prices[r.slug].sell ?? "–" : "…" }}</td>
            <td class="row" style="justify-content:flex-end">
              <button class="btn" :disabled="busy || !prices[r.slug]?.sell" @click="post(r.slug, prices[r.slug]!.sell!, r.quantity)">
                sell @ {{ prices[r.slug]?.sell ?? "?" }}p
              </button>
              <button class="btn" @click="choose(r.slug)">edit</button>
            </td>
          </tr>
        </tbody>
      </table>
      <p v-if="mode === 'sell' && !rows.length" class="muted">Inventory is empty — sync it on the Inventory tab or add items there.</p>

      <p v-if="done" class="ok">{{ done }}</p>
      <p v-if="err" class="error">{{ err }}</p>
      <div class="row" style="margin-top:12px"><button class="btn" @click="close">Close</button></div>
    </div>
  </div>
</template>

<style scoped>
.backdrop { position: fixed; inset: 0; background: rgba(0, 0, 0, .7); display: flex; align-items: center; justify-content: center; z-index: 50; }
.dlg { width: min(720px, calc(100vw - 32px)); max-height: 82vh; overflow-y: auto; overflow-x: hidden; }
.pick { margin-top: 10px; flex-wrap: wrap; }
.inv { margin-top: 10px; }
.hits { margin-top: 6px; border: 1px solid var(--border); max-height: 280px; overflow-y: auto; }
.hit { display: flex; align-items: center; gap: 10px; padding: 6px 8px; cursor: pointer; }
.hit:hover { background: var(--bg3); color: var(--accent2); }
.pickable { cursor: pointer; }
.pickable:hover { color: var(--accent2); }
</style>
