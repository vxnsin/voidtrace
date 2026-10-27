<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { api, type InvRow, type MarketItem, type Order } from "../api";

// My warframe.market listings against the inventory: sell orders for things
// I no longer have (or fewer of), and bulk show / hide / delete.
const props = defineProps<{ rows: InvRow[] }>();
const emit = defineEmits<{ listed: [slugs: Set<string>] }>();
const orders = ref<Order[]>([]);
const items = ref<Record<string, MarketItem>>({});
const msg = ref("");
const busy = ref(false);
const confirmDelete = ref(false);

async function load() {
  try {
    orders.value = await api.wfmMyOrders();
    const ids = [...new Set(orders.value.map((o) => o.itemId))].filter((id) => !items.value[id]);
    if (ids.length) Object.assign(items.value, await api.marketItemsById(ids));
    emit("listed", new Set(orders.value.filter((o) => o.type === "sell").map((o) => items.value[o.itemId]?.slug).filter(Boolean) as string[]));
  } catch (e) { msg.value = String(e); }
}
const have = (slug?: string) => props.rows.find((r) => r.slug === slug)?.quantity ?? 0;
const stale = computed(() => orders.value.filter((o) => o.type === "sell" && have(items.value[o.itemId]?.slug) < o.quantity));

async function fix(o: Order) {
  const n = have(items.value[o.itemId]?.slug);
  try {
    if (n <= 0) await api.wfmDeleteOrder(o.id);
    else await api.wfmUpdateOrder(o.id, o.platinum, n, o.visible);
    await load();
  } catch (e) { msg.value = String(e); }
}
async function bulk(what: "show" | "hide" | "delete") {
  busy.value = true; msg.value = "";
  let done = 0;
  for (const o of orders.value) {
    try {
      if (what === "delete") await api.wfmDeleteOrder(o.id);
      else if (o.visible !== (what === "show")) await api.wfmUpdateOrder(o.id, o.platinum, o.quantity, what === "show");
      else continue;
      done++;
      await new Promise((r) => setTimeout(r, 350)); // 3 requests a second at most
    } catch (e) { msg.value = String(e); break; }
  }
  confirmDelete.value = false;
  busy.value = false;
  msg.value ||= `${done} listings ${what === "delete" ? "deleted" : what === "show" ? "shown" : "hidden"}.`;
  await load();
}
onMounted(load);
</script>

<template>
  <div class="card">
    <div class="row">
      <span class="muted">{{ orders.length }} listings</span>
      <button class="btn" :disabled="busy || !orders.length" @click="bulk('show')">Show all</button>
      <button class="btn" :disabled="busy || !orders.length" @click="bulk('hide')">Hide all</button>
      <button v-if="!confirmDelete" class="btn danger" :disabled="busy || !orders.length" @click="confirmDelete = true">Delete all…</button>
      <template v-else>
        <span class="error">Delete all {{ orders.length }} listings?</span>
        <button class="btn danger" :disabled="busy" @click="bulk('delete')">Yes, delete</button>
        <button class="btn" @click="confirmDelete = false">No</button>
      </template>
      <span class="muted">{{ msg }}</span>
    </div>
    <table v-if="stale.length" style="margin-top:8px">
      <thead><tr><th>Listed but not in the inventory</th><th>Listed</th><th>Have</th><th></th></tr></thead>
      <tbody>
        <tr v-for="o in stale" :key="o.id">
          <td>{{ items[o.itemId]?.name ?? o.itemId }}</td>
          <td>{{ o.quantity }}</td>
          <td :class="have(items[o.itemId]?.slug) ? '' : 'error'">{{ have(items[o.itemId]?.slug) }}</td>
          <td><button class="btn" @click="fix(o)">{{ have(items[o.itemId]?.slug) ? `Set to ${have(items[o.itemId]?.slug)}` : "Delete listing" }}</button></td>
        </tr>
      </tbody>
    </table>
    <p v-else class="muted" style="margin:8px 0 0">Every sell listing is covered by the inventory.</p>
  </div>
</template>