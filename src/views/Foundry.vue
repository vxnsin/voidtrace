<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { listingPrompt } from "../api";
import { extra, type FoundryOverview, type FoundryRow, type PartSource } from "../extra";
import { live, startLive } from "../widgets/live";
import ItemIcon from "../components/ItemIcon.vue";

// Everything masterable with its crafting tree: blueprint, parts (built or as
// blueprints), what is still missing and where to get it.
const data = ref<FoundryOverview | null>(null);
const err = ref("");
const q = ref("");
const cat = ref("");
const status = ref<"all" | "ready" | "blueprint" | "missing" | "unmastered" | "mastered">("unmastered");
const vaulted = ref(false);
const mrOk = ref(false);
const resurg = ref(false);
const favOnly = ref(false);
const open = ref<string | null>(null);
const sources = ref<Record<string, PartSource[] | "loading">>({});
const favs = ref<Set<string>>(new Set());
try { favs.value = new Set(JSON.parse(localStorage.getItem("foundry_favs") ?? "[]")); } catch {}

const norm = (s: string) => s.toLowerCase().replace(/[^a-z0-9]/g, "");
const resurgence = computed(() => new Set((live.world?.vaultTrader?.inventory ?? []).map((i) => norm(i.item.replace(/ (set|pack|bundle)$/i, "")))));
const cats = computed(() => [...new Set(data.value?.rows.map((r) => r.category) ?? [])].sort());
const haveAll = (r: FoundryRow) => r.parts.filter((p) => p.have + (p.blueprint ? p.bp_have : 0) >= p.need).length;
const rows = computed(() => {
  const n = q.value.toLowerCase();
  return (data.value?.rows ?? []).filter((r) => {
    if (n && !r.name.toLowerCase().includes(n)) return false;
    if (cat.value && r.category !== cat.value) return false;
    if (vaulted.value && !r.vaulted) return false;
    if (mrOk.value && r.mastery_req > (data.value?.mastery_rank ?? 0)) return false;
    if (resurg.value && !resurgence.value.has(norm(r.name))) return false;
    if (favOnly.value && !favs.value.has(r.unique_name)) return false;
    switch (status.value) {
      case "ready": return r.ready;
      case "blueprint": return r.bp_have > 0;
      case "missing": return !r.mastered && r.xp === 0;
      case "unmastered": return !r.mastered;
      case "mastered": return r.mastered;
    }
    return true;
  }).sort((a, b) => Number(b.ready) - Number(a.ready) || haveAll(b) / Math.max(1, b.parts.length) - haveAll(a) / Math.max(1, a.parts.length) || a.name.localeCompare(b.name));
});

function fav(r: FoundryRow) {
  const s = new Set(favs.value);
  s.has(r.unique_name) ? s.delete(r.unique_name) : s.add(r.unique_name);
  favs.value = s;
  try { localStorage.setItem("foundry_favs", JSON.stringify([...s])); } catch {}
}
async function toggle(r: FoundryRow) {
  open.value = open.value === r.unique_name ? null : r.unique_name;
  if (open.value && !sources.value[r.unique_name]) {
    sources.value[r.unique_name] = "loading";
    try { sources.value[r.unique_name] = await extra.sources(r.unique_name); } catch { delete sources.value[r.unique_name]; }
  }
}
const src = (r: FoundryRow) => (Array.isArray(sources.value[r.unique_name]) ? (sources.value[r.unique_name] as PartSource[]) : []);

onMounted(async () => {
  startLive();
  try { data.value = await extra.foundry(); } catch (e) { err.value = String(e); }
});
</script>

<template>
  <h1>Foundry</h1>
  <p v-if="data && !data.synced" class="muted">Turn on the inventory sync (Settings) to see what you own: blueprints, built parts and resources.</p>
  <p v-if="err" class="error">{{ err }}</p>
  <div class="row" style="margin-bottom:8px">
    <input v-model="q" placeholder="Search…" style="width:220px" />
    <select v-model="cat"><option value="">All categories</option><option v-for="c in cats" :key="c">{{ c }}</option></select>
    <div class="modes">
      <button v-for="[k, l] in ([['unmastered', 'not mastered'], ['ready', 'ready to build'], ['blueprint', 'blueprint owned'], ['missing', 'never owned'], ['mastered', 'mastered'], ['all', 'all']] as const)" :key="k" :class="{ on: status === k }" @click="status = k">{{ l }}</button>
    </div>
  </div>
  <div class="row" style="margin-bottom:12px">
    <label class="row"><input type="checkbox" v-model="vaulted" /> vaulted</label>
    <label class="row"><input type="checkbox" v-model="mrOk" /> my MR is enough<span v-if="data" class="muted"> (MR {{ data.mastery_rank }})</span></label>
    <label class="row"><input type="checkbox" v-model="resurg" /> in Prime Resurgence</label>
    <label class="row"><input type="checkbox" v-model="favOnly" /> favourites</label>
    <span class="muted">{{ rows.length }} items</span>
  </div>
  <table>
    <thead><tr><th></th><th>Item</th><th>MR</th><th>Blueprint</th><th>Parts</th><th></th></tr></thead>
    <tbody>
      <template v-for="r in rows.slice(0, 300)" :key="r.unique_name">
        <tr style="cursor:pointer" @click="toggle(r)">
          <td><ItemIcon :src="r.image" /></td>
          <td>{{ r.name }} <span v-if="r.vaulted" class="tag vaulted">vaulted</span> <span v-if="r.mastered" class="ok">· mastered</span></td>
          <td class="muted">{{ r.mastery_req || "" }}</td>
          <td><span :class="r.bp_have ? 'ok' : 'muted'">{{ r.blueprint ? (r.bp_have ? "owned" : "missing") : "–" }}</span></td>
          <td>
            <span v-if="r.ready" class="ok">ready to build</span>
            <span v-else-if="r.parts.length">{{ haveAll(r) }} / {{ r.parts.length }}</span>
          </td>
          <td><span class="star" :class="{ on: favs.has(r.unique_name) }" @click.stop="fav(r)">★</span></td>
        </tr>
        <tr v-if="open === r.unique_name" class="tree">
          <td></td>
          <td colspan="5">
            <div class="parts">
              <div v-for="p in r.parts" :key="p.path" class="part" :class="{ done: p.have + (p.blueprint ? p.bp_have : 0) >= p.need }">
                <span>{{ p.need > 1 ? p.need + " × " : "" }}{{ p.name }}</span>
                <span class="muted">have {{ p.have }}<template v-if="p.blueprint"> · blueprints {{ p.bp_have }}</template></span>
              </div>
              <div class="muted">{{ r.credits ? r.credits.toLocaleString() + " credits to build" : "" }}</div>
            </div>
            <p v-if="sources[r.unique_name] === 'loading'" class="muted">Looking up relics and prices…</p>
            <table v-else-if="src(r).length" class="src">
              <thead><tr><th>Part</th><th>Market</th><th>Relics</th></tr></thead>
              <tbody>
                <tr v-for="s in src(r)" :key="s.name">
                  <td>{{ s.name }}</td>
                  <td>
                    <span v-if="s.plat != null" class="plat">{{ s.plat }}p</span><span v-else class="muted">–</span>
                    <button v-if="s.slug" class="btn" style="margin-left:8px" @click="listingPrompt = { mode: 'buy', slug: s.slug }">Buy</button>
                  </td>
                  <td>
                    <span v-for="h in s.relics" :key="h.relic" class="pill" style="margin:0 4px 4px 0">{{ h.relic }} <span class="muted">{{ h.rarity.toLowerCase() }}</span><span v-if="h.vaulted" class="ducat">v</span></span>
                    <span v-if="!s.relics.length" class="muted">not from relics</span>
                  </td>
                </tr>
              </tbody>
            </table>
          </td>
        </tr>
      </template>
    </tbody>
  </table>
</template>

<style scoped>
.star { cursor: pointer; color: var(--dim); }
.star.on { color: var(--warn); }
.tree td { background: var(--bg2); }
.parts { display: grid; gap: 4px; margin: 4px 0 10px; }
.part { display: flex; justify-content: space-between; gap: 16px; border-left: 2px solid var(--accent); padding-left: 8px; }
.part.done { border-left-color: var(--ok); }
.src th, .src td { padding: 4px 8px; }
</style>