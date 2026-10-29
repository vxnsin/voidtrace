<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { api, notifyApi, type MarketItem, type PriceAlert, type Status, type WatchRow, type WatchSnapshot } from "../api";
import ItemLink from "../components/ItemLink.vue";
import ItemIcon from "../components/ItemIcon.vue";

// Market watch: my listings against the cheapest online seller / best buyer,
// with a manual "match", and price alerts fed by the new-orders socket.
const props = defineProps<{ status: Status | null }>();
const snap = ref<WatchSnapshot>({ checked: 0, rows: [] });
const busy = ref(false);
const err = ref("");
const watchOn = ref(true);
const minutes = ref(5);
const alerts = ref<PriceAlert[]>([]);
const q = ref("");
const results = ref<MarketItem[]>([]);
const pick = ref<MarketItem | null>(null);
const max = ref<number | null>(null);

const beaten = computed(() => snap.value.rows.filter((r) => r.beaten).length);
const ago = computed(() => {
  if (!snap.value.checked) return "not checked yet";
  const m = Math.round((Date.now() / 1000 - snap.value.checked) / 60);
  return m < 1 ? "checked just now" : `checked ${m} min ago`;
});

async function refresh() {
  busy.value = true; err.value = "";
  try { snap.value = await notifyApi.watchRefresh(); } catch (e) { err.value = String(e); } finally { busy.value = false; }
}
async function match(r: WatchRow) {
  if (r.best == null) return;
  try {
    await api.wfmUpdateOrder(r.order_id, r.best, r.quantity, true);
    snap.value.rows = snap.value.rows.map((x) => (x.order_id === r.order_id ? { ...x, mine: r.best!, beaten: false } : x));
  } catch (e) { err.value = String(e); }
}
async function saveWatch() {
  await api.setSetting("watch_listings", watchOn.value ? "1" : "0");
  await api.setSetting("watch_minutes", String(Math.max(2, minutes.value || 5)));
}

async function search() { pick.value = null; results.value = q.value.length > 1 ? await api.marketSearch(q.value, 8) : []; }
async function choose(i: MarketItem) {
  pick.value = i; q.value = i.name; results.value = [];
  const p = await api.marketPrice(i.slug).catch(() => null);
  if (p?.sell && max.value == null) max.value = Math.max(1, Math.round(p.sell * 0.85));
}
// Saved first; on an error the list goes back to what is stored.
async function persist(next: PriceAlert[]) {
  const prev = alerts.value;
  alerts.value = next;
  try { await notifyApi.setAlerts(next); err.value = ""; return true; } catch (e) { alerts.value = prev; err.value = String(e); return false; }
}
async function addAlert() {
  if (!pick.value || !max.value) return;
  const a = { slug: pick.value.slug, name: pick.value.name, item_id: pick.value.id, max: Math.max(1, Math.round(max.value)) };
  if (await persist([...alerts.value.filter((x) => x.item_id !== a.item_id), a])) { pick.value = null; q.value = ""; max.value = null; }
}
async function removeAlert(a: PriceAlert) {
  await persist(alerts.value.filter((x) => x.item_id !== a.item_id));
}

onMounted(async () => {
  snap.value = await notifyApi.watch();
  alerts.value = await notifyApi.alerts();
  watchOn.value = (await api.getSetting("watch_listings")) !== "0";
  minutes.value = Number(await api.getSetting("watch_minutes")) || 5;
  if (!snap.value.checked && props.status?.wfm_logged_in) refresh();
});
</script>

<template>
  <h1>Market watch</h1>

  <h2>My listings</h2>
  <div class="card">
    <div class="row" style="margin-bottom:8px">
      <button class="btn primary" :disabled="busy || !status?.wfm_logged_in" @click="refresh">{{ busy ? "Checking…" : "Check now" }}</button>
      <span class="muted">{{ ago }}<template v-if="snap.rows.length"> · {{ beaten }} of {{ snap.rows.length }} beaten</template></span>
      <label class="row" style="margin-left:auto"><input type="checkbox" v-model="watchOn" @change="saveWatch" /> check every</label>
      <input type="number" min="2" max="60" v-model.number="minutes" style="width:56px" @change="saveWatch" /><span class="muted">min</span>
    </div>
    <p v-if="!status?.wfm_logged_in" class="muted">Log in on the Warframe.Market tab to watch your listings.</p>
    <p v-if="err" class="error">{{ err }}</p>
    <table v-if="snap.rows.length">
      <thead><tr><th>Item</th><th></th><th>Mine</th><th>Best other</th><th></th></tr></thead>
      <tbody>
        <tr v-for="r in snap.rows" :key="r.order_id" :class="{ hidden: !r.visible }">
          <td><ItemLink :slug="r.slug" :name="r.name" /><span v-if="r.rank != null" class="muted"> r{{ r.rank }}</span></td>
          <td><span class="tag">{{ r.kind }}</span></td>
          <td class="plat">{{ r.mine }}p</td>
          <td>
            <template v-if="!r.visible"><span class="muted">hidden</span></template>
            <template v-else-if="r.best != null"><span :class="r.beaten ? 'error' : 'ok'">{{ r.best }}p</span> <span class="muted">{{ r.best_user }}</span></template>
            <span v-else class="muted">nobody online</span>
          </td>
          <td><button v-if="r.beaten" class="btn" @click="match(r)">Match {{ r.best }}p</button></td>
        </tr>
      </tbody>
    </table>
    <p class="muted" style="margin:8px 0 0">Only players online or in game count. Matching changes your price once; nothing is changed automatically.</p>
  </div>

  <h2>Price alerts</h2>
  <div class="card">
    <div class="row add">
      <div class="search">
        <input v-model="q" placeholder="Item…" @input="search" style="width:260px" />
        <div v-if="results.length" class="drop">
          <div v-for="i in results" :key="i.id" class="opt" @click="choose(i)"><ItemIcon :icon="i.icon" :size="20" /> {{ i.name }}</div>
        </div>
      </div>
      <span class="muted">at or below</span>
      <input type="number" min="1" v-model.number="max" style="width:70px" /><span class="muted">p</span>
      <button class="btn primary" :disabled="!pick || !max" @click="addAlert">Add alert</button>
    </div>
    <table v-if="alerts.length" style="margin-top:8px">
      <tbody>
        <tr v-for="a in alerts" :key="a.item_id">
          <td><ItemLink :slug="a.slug" :name="a.name" /></td>
          <td class="plat">≤ {{ a.max }}p</td>
          <td style="text-align:right"><button class="btn danger" @click="removeAlert(a)">Remove</button></td>
        </tr>
      </tbody>
    </table>
    <p class="muted" style="margin:8px 0 0">Listens to warframe.market's live feed of new orders while VoidTrace runs (no polling, no login needed). Alerts show in game and go to Discord if a webhook takes them.</p>
  </div>
</template>

<style scoped>
tr.hidden td { opacity: .5; }
.search { position: relative; }
.drop { position: absolute; z-index: 5; top: 100%; left: 0; right: 0; background: var(--bg2); border: 1px solid var(--dim); }
.opt { padding: 4px 8px; cursor: pointer; display: flex; gap: 8px; align-items: center; }
.opt:hover { background: var(--bg3); color: var(--accent2); }
</style>
