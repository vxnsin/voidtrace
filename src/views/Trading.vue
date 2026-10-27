<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { api, fmtTs, tradePrompt, type Datapoint, type Status } from "../api";
import ItemIcon from "../components/ItemIcon.vue";

// One ledger from two sources: closed orders on warframe.market (public profile
// statistics) and trades stored here (closed through VoidTrace or typed in).
// A trade VoidTrace closed exists on both sides and is counted once.
type Row = { key: string; ts: number; kind: "sell" | "buy"; name: string; slug: string | null; quantity: number; total: number; source: string; localId?: number };

const props = defineProps<{ status: Status | null }>();
const rows = ref<Row[]>([]);
const points = ref<Datapoint[]>([]);
const range = ref<7 | 30 | 90 | 0>(30);
const note = ref("");
const syncedAt = ref<number | null>(null);
const busy = ref(false);
const adding = ref(false);
const nt = ref({ item: "", quantity: 1, platinum: 0, direction: "sell" as "sell" | "buy", partner: "" });

const norm = (s: string) => s.toLowerCase().replace(/[^a-z0-9]/g, "");

async function load() {
  busy.value = true; note.value = "";
  const [closed, trades, dps] = await Promise.all([
    props.status?.wfm_logged_in ? api.wfmClosedOrders().catch((e) => { note.value = `warframe.market: ${e}`; return []; }) : Promise.resolve([]),
    api.trades().catch(() => []),
    api.datapoints(5000).catch(() => []),
  ]);
  const out: Row[] = closed.map((o) => ({
    key: "w" + o.id, ts: Math.floor(Date.parse(o.closed) / 1000), kind: o.kind, name: o.name, slug: o.slug,
    quantity: o.quantity, total: Math.round(o.platinum * o.quantity), source: "warframe.market",
  }));
  for (const t of trades) {
    const twin = out.find((r) => r.source === "warframe.market" && r.kind === t.direction && norm(r.name) === norm(t.item) && Math.abs(r.ts - t.ts) < 900);
    if (twin) { twin.source = "VoidTrace"; twin.localId = t.id; continue; }
    out.push({ key: "l" + t.id, ts: t.ts, kind: t.direction, name: t.item, slug: null, quantity: t.quantity, total: t.platinum, source: "manual", localId: t.id });
  }
  out.sort((a, b) => b.ts - a.ts);
  // Manual rows have no slug; look it up for the picture.
  await Promise.all(out.filter((r) => !r.slug).map(async (r) => {
    r.slug = (await api.marketSearch(r.name, 5)).find((i) => norm(i.name) === norm(r.name))?.slug ?? null;
  }));
  rows.value = out;
  points.value = [...dps].sort((a, b) => a.ts - b.ts);
  syncedAt.value = Date.now() / 1000;
  busy.value = false;
}

const now = () => Date.now() / 1000;
const from = computed(() => (range.value ? now() - range.value * 86400 : Math.min(...rows.value.map((r) => r.ts), now())));
const inRange = computed(() => rows.value.filter((r) => r.ts >= from.value));
const prevRange = computed(() => range.value ? rows.value.filter((r) => r.ts < from.value && r.ts >= from.value - range.value * 86400) : []);
const sum = (list: Row[], kind: "sell" | "buy") => list.filter((r) => r.kind === kind).reduce((s, r) => s + r.total, 0);
const count = (list: Row[], kind: "sell" | "buy") => list.filter((r) => r.kind === kind).length;
const sold = computed(() => sum(inRange.value, "sell"));
const bought = computed(() => sum(inRange.value, "buy"));
const net = computed(() => sold.value - bought.value);
const prevNet = computed(() => sum(prevRange.value, "sell") - sum(prevRange.value, "buy"));
const lastPoint = computed(() => points.value[points.value.length - 1] ?? null);

const top = computed(() => {
  const m = new Map<string, { name: string; slug: string | null; total: number }>();
  for (const r of inRange.value.filter((r) => r.kind === "sell")) {
    const e = m.get(norm(r.name)) ?? { name: r.name, slug: r.slug, total: 0 };
    e.total += r.total; m.set(norm(r.name), e);
  }
  return [...m.values()].sort((a, b) => b.total - a.total).slice(0, 5);
});

// Per-day bars (sold / bought) and the in-game plat line from inventory syncs.
const chart = computed(() => {
  const days = Math.max(1, Math.min(180, Math.ceil((now() - from.value) / 86400)));
  const day0 = Math.floor(now() / 86400) - days + 1;
  const s = new Array(days).fill(0), b = new Array(days).fill(0);
  for (const r of inRange.value) {
    const i = Math.floor(r.ts / 86400) - day0;
    if (i >= 0 && i < days) (r.kind === "sell" ? s : b)[i] += r.total;
  }
  const max = Math.max(1, ...s, ...b);
  const bars = s.map((v, i) => ({ s: (v / max) * 100, b: (b[i] / max) * 100, tip: `${label(i)} · sold ${v}p · bought ${b[i]}p` }));
  // In-game plat as a line, only once there are values from two different days.
  const pts = points.value.filter((p) => p.ts >= day0 * 86400);
  let line = "";
  if (new Set(pts.map((p) => Math.floor(p.ts / 86400))).size >= 2) {
    const lo = Math.min(...pts.map((p) => p.platinum)), hi = Math.max(...pts.map((p) => p.platinum));
    line = pts.map((p) => `${((p.ts / 86400 - day0) / days) * 100},${100 - ((p.platinum - lo) / Math.max(1, hi - lo)) * 90 - 5}`).join(" ");
  }
  function label(i: number) { return new Date((day0 + i) * 86400000).toLocaleDateString("en-GB", { day: "2-digit", month: "short" }); }
  return { bars, line, max, labels: [label(0), label(Math.floor(days / 2)), label(days - 1)], empty: !inRange.value.length };
});

async function addManual() {
  if (!nt.value.item.trim()) return;
  await api.addTrade({ ...nt.value, item: nt.value.item.trim() });
  nt.value = { item: "", quantity: 1, platinum: 0, direction: "sell", partner: "" };
  adding.value = false;
  await load();
}
async function del(r: Row) { if (r.localId != null) { await api.deleteTrade(r.localId); await load(); } }
function fromListing() { tradePrompt.value = { onDone: load }; }
const ago = computed(() => (syncedAt.value ? `synced ${new Date(syncedAt.value * 1000).toLocaleTimeString("en-GB", { hour12: false })}` : ""));

onMounted(load);
</script>

<template>
  <div class="row" style="justify-content:space-between;margin-bottom:14px">
    <h1 style="margin:0">Trading <span class="muted" v-if="ago">· {{ ago }}</span></h1>
    <div class="row">
      <div class="modes">
        <button v-for="r in ([7, 30, 90, 0] as const)" :key="r" :class="{ on: range === r }" @click="range = r">{{ r ? r + "d" : "all" }}</button>
      </div>
      <button class="btn" :disabled="busy" @click="load">{{ busy ? "Syncing…" : "Sync" }}</button>
    </div>
  </div>
  <p v-if="!status?.wfm_logged_in" class="muted">Log in on the Warframe.Market tab and your closed orders show up here by themselves.</p>
  <p v-if="note" class="error">{{ note }}</p>

  <div class="grid cols-4">
    <div class="card"><div class="stat plat">{{ sold.toLocaleString() }}p<small>sold · {{ count(inRange, "sell") }} trades{{ count(inRange, "sell") ? ` · avg ${Math.round(sold / count(inRange, "sell"))}p` : "" }}</small></div></div>
    <div class="card"><div class="stat">{{ bought.toLocaleString() }}p<small>bought · {{ count(inRange, "buy") }} trades</small></div></div>
    <div class="card"><div class="stat" :class="net >= 0 ? 'ok' : 'error'">{{ net >= 0 ? "+" : "" }}{{ net.toLocaleString() }}p<small>net<template v-if="range && prevRange.length"> · {{ net - prevNet >= 0 ? "+" : "" }}{{ (net - prevNet).toLocaleString() }}p vs previous {{ range }}d</template></small></div></div>
    <div class="card"><div class="stat">{{ lastPoint ? lastPoint.platinum.toLocaleString() + "p" : "–" }}<small>{{ lastPoint ? "plat in game · " + fmtTs(lastPoint.ts) : "plat in game · needs inventory sync" }}</small></div></div>
  </div>

  <h2>Plat per day</h2>
  <div class="card">
    <p v-if="chart.empty" class="muted" style="margin:0">No trades in this range yet.</p>
    <template v-else>
      <div class="plot">
        <span class="ymax muted">{{ chart.max }}p</span>
        <div class="cols">
          <div v-for="(b, i) in chart.bars" :key="i" class="col" :title="b.tip">
            <i class="s" :style="{ height: b.s + '%' }"></i><i class="b" :style="{ height: b.b + '%' }"></i>
          </div>
        </div>
        <svg v-if="chart.line" class="line" viewBox="0 0 100 100" preserveAspectRatio="none"><polyline :points="chart.line" fill="none" stroke="var(--accent)" stroke-width="2" vector-effect="non-scaling-stroke" /></svg>
      </div>
      <div class="xl muted"><span v-for="(l, i) in chart.labels" :key="l + i">{{ l }}</span></div>
      <div class="row muted" style="gap:16px;margin-top:6px;font-size:11px">
        <span><i class="sw" style="background:var(--accent2)"></i>sold</span><span><i class="sw" style="background:var(--muted)"></i>bought</span>
        <span v-if="chart.line"><i class="sw" style="background:var(--accent);height:2px"></i>plat in game</span>
      </div>
    </template>
  </div>

  <div class="split">
    <div>
      <div class="row" style="justify-content:space-between">
        <h2>Ledger</h2>
        <div class="row">
          <button class="btn" @click="fromListing">From a listing…</button>
          <button class="btn" @click="adding = !adding">{{ adding ? "Cancel" : "Add manually" }}</button>
        </div>
      </div>
      <div v-if="adding" class="card row" style="margin-bottom:10px">
        <select v-model="nt.direction"><option value="sell">Sold</option><option value="buy">Bought</option></select>
        <input v-model="nt.item" placeholder="Item" style="width:220px" />
        <label>Qty <input type="number" min="1" v-model.number="nt.quantity" style="width:60px" /></label>
        <label>Total <input type="number" min="0" v-model.number="nt.platinum" style="width:80px" /></label>
        <input v-model="nt.partner" placeholder="Partner" style="width:140px" />
        <button class="btn primary" @click="addManual">Add</button>
      </div>
      <table v-if="inRange.length">
        <thead><tr><th>When</th><th></th><th>Item</th><th>Qty</th><th>Plat</th><th>Source</th><th></th></tr></thead>
        <tbody>
          <tr v-for="r in inRange" :key="r.key">
            <td class="muted">{{ fmtTs(r.ts) }}</td>
            <td><span class="tag" :class="r.kind === 'sell' ? 'sold' : ''">{{ r.kind === "sell" ? "sold" : "bought" }}</span></td>
            <td><span class="row" style="gap:10px;flex-wrap:nowrap"><ItemIcon :slug="r.slug" /><span>{{ r.name }}</span></span></td>
            <td>{{ r.quantity }}</td>
            <td :class="r.kind === 'sell' ? 'ok' : ''">{{ r.kind === "sell" ? "+" : "−" }}{{ r.total }}p</td>
            <td class="muted">{{ r.source }}</td>
            <td><button v-if="r.source === 'manual'" class="btn danger" @click="del(r)">✕</button></td>
          </tr>
        </tbody>
      </table>
      <p v-else class="muted">Nothing in this range.</p>
    </div>
    <div>
      <h2>Best sellers</h2>
      <div v-for="t in top" :key="t.name" class="best">
        <span class="row" style="gap:10px;flex-wrap:nowrap"><ItemIcon :slug="t.slug" /><span>{{ t.name }}</span></span>
        <span class="plat">{{ t.total }}p</span>
        <div class="bar"><i :style="{ width: (t.total / top[0].total) * 100 + '%' }"></i></div>
      </div>
      <p v-if="!top.length" class="muted">No sales in this range.</p>
    </div>
  </div>
</template>

<style scoped>
.split { display: grid; grid-template-columns: minmax(0, 1fr) 280px; gap: 24px; }
@media (max-width: 1100px) { .split { grid-template-columns: 1fr; } }
.best { display: grid; grid-template-columns: 1fr auto; gap: 6px 10px; align-items: center; margin-bottom: 12px; }
.best .bar { grid-column: 1 / -1; height: 3px; }
.best .bar i { display: block; height: 100%; background: var(--accent2); }
.sw { display: inline-block; width: 10px; height: 10px; margin-right: 6px; vertical-align: middle; }
.plot { position: relative; height: 160px; border-bottom: 1px solid var(--border); background-image: linear-gradient(var(--border) 1px, transparent 1px); background-size: 100% 33.333%; }
.ymax { position: absolute; top: -2px; left: 0; font-size: 10px; }
.cols { position: absolute; inset: 14px 0 0 0; display: flex; align-items: flex-end; gap: 2px; }
.col { flex: 1 1 0; height: 100%; display: flex; align-items: flex-end; justify-content: center; gap: 1px; min-width: 0; }
.col i { display: block; width: 40%; max-width: 12px; }
.col .s { background: var(--accent2); opacity: .85; } .col .b { background: var(--muted); }
.col:hover { background: var(--bg3); }
.line { position: absolute; inset: 14px 0 0 0; width: 100%; height: calc(100% - 14px); pointer-events: none; }
.xl { display: flex; justify-content: space-between; font-size: 10px; margin-top: 4px; }
.tag.sold { color: var(--ok); border-color: color-mix(in srgb, var(--ok) 35%, transparent); }
.stat.ok { color: var(--ok); } .stat.error { color: var(--accent); }
</style>
