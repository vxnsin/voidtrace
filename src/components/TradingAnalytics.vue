<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { dataApi, type Datapoint, type ItemStats } from "../api";
import { live, startLive } from "../widgets/live";
import ItemIcon from "./ItemIcon.vue";

// What the ledger means: margin per item (buy cost against sales, sets from
// bought parts), who I trade with, credits and ducats over time, and the
// market's own history for one item.
export type LedgerRow = { ts: number; kind: "sell" | "buy"; name: string; slug: string | null; quantity: number; total: number; partner?: string };
const props = defineProps<{ rows: LedgerRow[]; all: LedgerRow[]; points: Datapoint[] }>();

const norm = (s: string) => s.toLowerCase().replace(/[^a-z0-9]/g, "");
const setBase = (name: string) => (/ set$/i.test(name) ? name.replace(/ set$/i, "") : null);

// Average buy price per unit, over all time (a purchase last month still
// counts against a sale today).
const buyAvg = computed(() => {
  const m = new Map<string, { qty: number; cost: number }>();
  for (const r of props.all.filter((r) => r.kind === "buy")) {
    const e = m.get(norm(r.name)) ?? { qty: 0, cost: 0 };
    e.qty += r.quantity; e.cost += r.total; m.set(norm(r.name), e);
  }
  return m;
});
/** Cost of one set from the parts I bought: one of each part name seen. */
function setCost(set: string): number | null {
  const base = norm(setBase(set) ?? "");
  if (!base) return null;
  const parts = [...buyAvg.value.entries()].filter(([k]) => k.startsWith(base) && k !== norm(set));
  if (!parts.length) return null;
  return parts.reduce((s, [, v]) => s + v.cost / v.qty, 0);
}

type ProfitRow = { name: string; slug: string | null; qty: number; revenue: number; cost: number | null; assembled: boolean };
const profit = computed(() => {
  const m = new Map<string, ProfitRow>();
  for (const r of props.rows.filter((r) => r.kind === "sell")) {
    const k = norm(r.name);
    const e = m.get(k) ?? { name: r.name, slug: r.slug, qty: 0, revenue: 0, cost: null, assembled: false };
    e.qty += r.quantity; e.revenue += r.total; m.set(k, e);
  }
  for (const [k, e] of m) {
    const b = buyAvg.value.get(k);
    if (b) e.cost = Math.round((b.cost / b.qty) * e.qty);
    else {
      const unit = setCost(e.name);
      if (unit != null) { e.cost = Math.round(unit * e.qty); e.assembled = true; }
    }
  }
  return [...m.values()]
    .map((e) => ({ ...e, margin: e.cost == null ? null : e.revenue - e.cost }))
    .sort((a, b) => (b.margin ?? -1e9) - (a.margin ?? -1e9) || b.revenue - a.revenue)
    .slice(0, 12);
});

type Partner = { name: string; trades: number; sold: number; bought: number; best: { name: string; total: number } | null };
const partners = computed(() => {
  const m = new Map<string, Partner>();
  for (const r of props.rows.filter((r) => r.partner)) {
    const k = r.partner!.toLowerCase();
    const e = m.get(k) ?? { name: r.partner!, trades: 0, sold: 0, bought: 0, best: null };
    e.trades++;
    if (r.kind === "sell") { e.sold += r.total; if (!e.best || r.total > e.best.total) e.best = { name: r.name, total: r.total }; }
    else e.bought += r.total;
    m.set(k, e);
  }
  return [...m.values()].sort((a, b) => b.trades - a.trades || b.sold - a.sold).slice(0, 8);
});

// Credits and ducats from the inventory syncs, each scaled to its own range.
const lines = computed(() => {
  const pts = props.points;
  if (pts.length < 2) return null;
  const t0 = pts[0].ts, t1 = pts[pts.length - 1].ts;
  const line = (v: (p: Datapoint) => number) => {
    const lo = Math.min(...pts.map(v)), hi = Math.max(...pts.map(v));
    return pts.map((p) => `${((p.ts - t0) / Math.max(1, t1 - t0)) * 100},${95 - ((v(p) - lo) / Math.max(1, hi - lo)) * 90}`).join(" ");
  };
  const last = pts[pts.length - 1], first = pts[0];
  return { credits: line((p) => p.credits), ducats: line((p) => p.ducats), last, dc: last.credits - first.credits, dd: last.ducats - first.ducats };
});

// Market history for one item.
const traded = computed(() => {
  const seen = new Map<string, { name: string; slug: string }>();
  for (const r of props.all) if (r.slug && !seen.has(r.slug)) seen.set(r.slug, { name: r.name, slug: r.slug });
  return [...seen.values()].slice(0, 40);
});
const pick = ref("");
const stats = ref<ItemStats | null>(null);
const statsErr = ref("");
async function loadStats() {
  stats.value = null; statsErr.value = "";
  if (!pick.value) return;
  try { stats.value = await dataApi.marketStats(pick.value); } catch (e) { statsErr.value = String(e); }
}
const median = (v: number[]) => [...v].sort((a, b) => a - b)[Math.floor(v.length / 2)] ?? 0;
const market = computed(() => {
  const d = stats.value?.days90 ?? [];
  if (!d.length) return null;
  const hi = Math.max(...d.map((p) => p.max || p.avg)), vmax = Math.max(1, ...d.map((p) => p.volume));
  const x = (i: number) => (i / Math.max(1, d.length - 1)) * 100;
  const y = (v: number) => 95 - (v / Math.max(1, hi)) * 85;
  const med90 = median(d.map((p) => p.median));
  const h = stats.value!.hours48;
  const recent = h.length ? h : d.slice(-2);
  const med48 = median(recent.map((p) => p.median));
  return {
    median: d.map((p, i) => `${x(i)},${y(p.median)}`).join(" "),
    band: d.map((p, i) => `${x(i)},${y(p.max)}`).concat(d.map((p, i) => `${x(i)},${y(p.min)}`).reverse()).join(" "),
    vols: d.map((p, i) => ({ x: x(i), h: (p.volume / vmax) * 30 })),
    min: Math.min(...recent.map((p) => p.min)), max: Math.max(...recent.map((p) => p.max)),
    med48, med90, vol48: recent.reduce((s, p) => s + p.volume, 0), hi,
    hint: med48 > med90 * 1.1 ? "above its 90-day median: a good moment to sell" : med48 < med90 * 0.9 ? "below its 90-day median: better to buy than to sell" : "close to its 90-day median",
  };
});

// Prime Resurgence: things I trade that Varzia sells right now get cheaper.
const resurgence = computed(() => {
  const inv = live.world?.vaultTrader?.inventory ?? [];
  const bases = inv.map((i) => norm(i.item.replace(/ (set|pack|bundle)$/i, ""))).filter((b) => b.length > 4);
  const mine = new Map<string, string>();
  for (const r of props.all) if (bases.some((b) => norm(r.name).startsWith(b))) mine.set(norm(r.name), r.name);
  return { names: [...mine.values()].slice(0, 10), until: live.world?.vaultTrader?.expiry };
});

onMounted(() => {
  startLive();
  if (traded.value.length) { pick.value = traded.value[0].slug; loadStats(); }
});
</script>

<template>
  <div v-if="resurgence.names.length" class="card resurg">
    <b>Prime Resurgence</b>
    <span class="muted">back from the vault until {{ resurgence.until ? new Date(resurgence.until).toLocaleDateString() : "soon" }}, prices usually drop:</span>
    <span>{{ resurgence.names.join(", ") }}</span>
  </div>

  <h2>Profit per item</h2>
  <div class="card">
    <p v-if="!profit.length" class="muted" style="margin:0">No sales in this range.</p>
    <table v-else>
      <thead><tr><th>Item</th><th>Sold</th><th>Revenue</th><th>Cost</th><th>Margin</th></tr></thead>
      <tbody>
        <tr v-for="p in profit" :key="p.name">
          <td><span class="row" style="gap:8px"><ItemIcon :slug="p.slug" :size="20" />{{ p.name }}</span></td>
          <td>{{ p.qty }}</td>
          <td class="plat">{{ p.revenue }}p</td>
          <td>
            <template v-if="p.cost != null">{{ p.cost }}p<span v-if="p.assembled" class="muted" title="Cost from the parts you bought"> · from parts</span></template>
            <span v-else class="muted">no purchase</span>
          </td>
          <td><span v-if="p.margin != null" :class="p.margin >= 0 ? 'ok' : 'error'">{{ p.margin >= 0 ? "+" : "" }}{{ p.margin }}p</span><span v-else class="muted">–</span></td>
        </tr>
      </tbody>
    </table>
    <p class="muted" style="margin:8px 0 0">Cost is your average buy price over all time. A set counts as assembled when you bought its parts instead.</p>
  </div>

  <div class="two">
    <div>
      <h2>Trading partners</h2>
      <div class="card">
        <p v-if="!partners.length" class="muted" style="margin:0">Partners are known for trades recorded in VoidTrace.</p>
        <table v-else>
          <thead><tr><th>Player</th><th>Trades</th><th>Sold to</th><th>Best deal</th></tr></thead>
          <tbody>
            <tr v-for="p in partners" :key="p.name">
              <td class="hi">{{ p.name }}</td><td>{{ p.trades }}</td><td class="plat">{{ p.sold }}p</td>
              <td class="muted">{{ p.best ? `${p.best.name} ${p.best.total}p` : "–" }}</td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>
    <div>
      <h2>Credits &amp; ducats</h2>
      <div class="card">
        <p v-if="!lines" class="muted" style="margin:0">Needs two inventory syncs.</p>
        <template v-else>
          <svg class="spark" viewBox="0 0 100 100" preserveAspectRatio="none">
            <polyline :points="lines.credits" fill="none" stroke="var(--fg2)" stroke-width="2" vector-effect="non-scaling-stroke" />
            <polyline :points="lines.ducats" fill="none" stroke="var(--warn)" stroke-width="2" vector-effect="non-scaling-stroke" />
          </svg>
          <div class="row" style="justify-content:space-between">
            <span><span class="muted">credits</span> {{ lines.last.credits.toLocaleString() }} <span :class="lines.dc >= 0 ? 'ok' : 'error'">{{ lines.dc >= 0 ? "+" : "" }}{{ lines.dc.toLocaleString() }}</span></span>
            <span><span class="ducat">ducats</span> {{ lines.last.ducats.toLocaleString() }} <span :class="lines.dd >= 0 ? 'ok' : 'error'">{{ lines.dd >= 0 ? "+" : "" }}{{ lines.dd.toLocaleString() }}</span></span>
          </div>
        </template>
      </div>
    </div>
  </div>

  <h2>Market history</h2>
  <div class="card">
    <div class="row">
      <select v-model="pick" @change="loadStats" style="min-width:260px">
        <option value="" disabled>Pick an item you traded</option>
        <option v-for="t in traded" :key="t.slug" :value="t.slug">{{ t.name }}</option>
      </select>
      <span v-if="statsErr" class="error">{{ statsErr }}</span>
    </div>
    <template v-if="market">
      <svg class="hist" viewBox="0 0 100 100" preserveAspectRatio="none">
        <polygon :points="market.band" fill="var(--dim)" opacity=".5" />
        <rect v-for="(v, i) in market.vols" :key="i" :x="v.x - 0.4" :y="100 - v.h" width="0.8" :height="v.h" fill="var(--muted)" opacity=".5" />
        <polyline :points="market.median" fill="none" stroke="var(--accent)" stroke-width="2" vector-effect="non-scaling-stroke" />
      </svg>
      <div class="row muted" style="justify-content:space-between"><span>90 days ago</span><span>max {{ Math.round(market.hi) }}p</span><span>today</span></div>
      <div class="grid cols-4" style="margin-top:8px">
        <div class="stat">{{ Math.round(market.med48) }}p<small>median · 48h</small></div>
        <div class="stat">{{ Math.round(market.min) }}–{{ Math.round(market.max) }}p<small>range · 48h</small></div>
        <div class="stat">{{ Math.round(market.med90) }}p<small>median · 90d</small></div>
        <div class="stat">{{ market.vol48 }}<small>sold · 48h</small></div>
      </div>
      <p style="margin:8px 0 0">Right now it trades <b>{{ market.hint }}</b>.</p>
    </template>
  </div>
</template>

<style scoped>
.resurg { margin: 14px 0 0; border-color: var(--warn); display: flex; gap: 8px; flex-wrap: wrap; align-items: baseline; }
.resurg b { color: var(--warn); font-weight: 500; text-transform: uppercase; letter-spacing: .1em; font-size: 11px; }
.two { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; }
@media (max-width: 1000px) { .two { grid-template-columns: 1fr; } }
.hi { color: var(--accent2); }
.spark { width: 100%; height: 90px; display: block; margin-bottom: 6px; }
.hist { width: 100%; height: 160px; display: block; margin-top: 10px; }
</style>
