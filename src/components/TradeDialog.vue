<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { api, tradePrompt, type MarketItem, type Order } from "../api";
import ItemIcon from "./ItemIcon.vue";

// The log only says *that* a trade finished, not what changed hands, so the
// user picks the listing (or types the item) and we do the bookkeeping.
const direction = ref<"sell" | "buy">(tradePrompt.value?.order?.type ?? "sell");
const orders = ref<Order[]>([]);
const itemsById = ref<Record<string, MarketItem>>({});
const picked = ref<Order | null>(tradePrompt.value?.order ?? null);
const closeOrder = ref(true);
const item = ref(""); const slug = ref<string | null>(null);
const quantity = ref(1); const unit = ref(0); const partner = ref("");
const q = ref(""); const results = ref<MarketItem[]>([]);
const busy = ref(false); const err = ref("");

const visible = computed(() => orders.value.filter((o) => o.type === direction.value));
const name = (o: Order) => itemsById.value[o.itemId]?.name ?? o.itemId;

function pick(o: Order) {
  picked.value = o;
  direction.value = o.type;
  item.value = name(o);
  slug.value = itemsById.value[o.itemId]?.slug ?? null;
  unit.value = o.platinum;
  quantity.value = 1;
  closeOrder.value = true;
}
function setDirection(d: "sell" | "buy") { direction.value = d; picked.value = null; }
async function search() { results.value = q.value.length > 1 ? await api.marketSearch(q.value, 8) : []; }
function choose(i: MarketItem) { item.value = i.name; slug.value = i.slug; q.value = i.name; results.value = []; }

async function save() {
  if (!item.value.trim() || quantity.value < 1) { err.value = "Item and quantity are required."; return; }
  busy.value = true; err.value = "";
  try {
    await api.recordTrade({
      order_id: picked.value && closeOrder.value ? picked.value.id : null,
      slug: slug.value,
      item: item.value.trim(),
      quantity: quantity.value,
      platinum: unit.value * quantity.value,
      direction: direction.value,
      partner: partner.value.trim(),
    });
    tradePrompt.value?.onDone?.();
    tradePrompt.value = null;
  } catch (e) { err.value = String(e); } finally { busy.value = false; }
}
function skip() { tradePrompt.value = null; }
function key(e: KeyboardEvent) { if (e.key === "Escape") skip(); }

onMounted(async () => {
  window.addEventListener("keydown", key);
  const s = await api.status();
  if (s.wfm_logged_in) {
    try { orders.value = await api.wfmMyOrders(); } catch (e) { err.value = String(e); }
    const ids = [...new Set(orders.value.map((o) => o.itemId))];
    if (ids.length) itemsById.value = await api.marketItemsById(ids);
  }
  if (picked.value) pick(picked.value);
});
onUnmounted(() => window.removeEventListener("keydown", key));
</script>

<template>
  <div class="backdrop" @click.self="skip">
    <div class="card dlg">
      <h1>Trade complete</h1>
      <div class="row">
        <div class="modes">
          <button :class="{ on: direction === 'sell' }" @click="setDirection('sell')">Sold</button>
          <button :class="{ on: direction === 'buy' }" @click="setDirection('buy')">Bought</button>
        </div>
        <span class="muted">Pick the listing it was, or enter the item.</span>
      </div>

      <table v-if="visible.length" style="margin-top:10px">
        <tbody>
          <tr v-for="o in visible" :key="o.id" class="pickable" :class="{ sel: picked?.id === o.id }" @click="pick(o)">
            <td>{{ picked?.id === o.id ? ">" : "" }}</td>
            <td><span class="row" style="gap:10px"><ItemIcon :icon="itemsById[o.itemId]?.icon" /><span>{{ name(o) }}<span v-if="o.rank != null" class="muted"> · r{{ o.rank }}</span></span></span></td>
            <td class="plat">{{ o.platinum }}p</td>
            <td class="muted">× {{ o.quantity }}</td>
          </tr>
        </tbody>
      </table>

      <div class="row" style="margin-top:10px" v-if="!picked">
        <ItemIcon v-if="slug" :slug="slug" :size="32" />
        <input v-model="q" @input="search" placeholder="Item" style="width:260px" />
      </div>
      <div v-if="!picked && results.length" class="hits">
        <div v-for="r in results" :key="r.id" class="hit" @click="choose(r)">
          <ItemIcon :icon="r.icon" :size="32" /><span>{{ r.name }}</span>
        </div>
      </div>

      <div class="row" style="margin-top:10px">
        <label>Qty <input type="number" min="1" :max="picked?.quantity" v-model.number="quantity" style="width:60px" /></label>
        <label>Plat each <input type="number" min="0" v-model.number="unit" style="width:80px" /></label>
        <span class="muted">= <span class="plat">{{ unit * quantity }}p</span></span>
        <input v-model="partner" placeholder="Partner" style="width:150px" />
      </div>
      <label class="row" v-if="picked" style="margin-top:8px">
        <input type="checkbox" v-model="closeOrder" /> Close {{ quantity }} on warframe.market (records the sale, lowers the listing)
      </label>

      <p v-if="err" class="error">{{ err }}</p>
      <div class="row" style="margin-top:12px">
        <button class="btn primary" :disabled="busy" @click="save">{{ busy ? "Saving…" : "Record" }}</button>
        <button class="btn" @click="skip">Skip</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.backdrop { position: fixed; inset: 0; background: rgba(0, 0, 0, .7); display: flex; align-items: center; justify-content: center; z-index: 50; }
.dlg { width: min(560px, calc(100vw - 32px)); max-height: 80vh; overflow-y: auto; overflow-x: hidden; }
.hits { margin-top: 6px; border: 1px solid var(--border); max-height: 240px; overflow-y: auto; }
.hit { display: flex; align-items: center; gap: 10px; padding: 6px 8px; cursor: pointer; }
.hit:hover { background: var(--bg3); color: var(--accent2); }
.pickable { cursor: pointer; }
.pickable.sel td { background: var(--bg3); color: var(--accent2); }
</style>
