<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { api, type InvRow } from "../api";
import { TRACES, relicOverview, squadValue, type RelicOverview, type RelicRow } from "../relics";

// Every relic (or the ones I own) ranked: expected plat or ducats per crack
// for the squad size, what refining to radiant gains per trace, and how many
// of its rewards I still lack.
const emit = defineEmits<{ pick: [relic: string] }>();
const REF = ["Int", "Exc", "Fla", "Rad"];
const data = ref<RelicOverview | null>(null);
const err = ref("");
const onlyOwned = ref(true);
const squad = ref(4);
const refine = ref(0);
const sort = ref<"plat" | "ducats" | "upgrade" | "missing">("plat");
const vault = ref<"all" | "vaulted" | "available">("all");
const many = ref(false);
const complete = ref(false);
const inv = ref<Set<string>>(new Set());
const progress = ref("");
let off: (() => void) | undefined;
let reloadTimer: number | undefined;

async function load() {
  err.value = "";
  try { data.value = await relicOverview(onlyOwned.value); } catch (e) { err.value = String(e); }
}
const plat = (d: RelicRow["plan"]["drops"][number]) => d.plat ?? 0;
const ducats = (d: RelicRow["plan"]["drops"][number]) => d.ducats;
const lacking = (r: RelicRow) => r.plan.drops.filter((d) => d.slug && !inv.value.has(d.slug) && !/forma/i.test(d.item)).length;

const rows = computed(() => {
  const list = (data.value?.rows ?? []).filter((r) => {
    const total = r.owned.reduce((a, b) => a + b, 0);
    if (vault.value === "vaulted" && !r.vaulted) return false;
    if (vault.value === "available" && r.vaulted) return false;
    if (many.value && total < 10) return false;
    if (complete.value && lacking(r) > 0) return false;
    return true;
  }).map((r) => {
    const p = squadValue(r.plan, refine.value, squad.value, plat);
    const d = squadValue(r.plan, refine.value, squad.value, ducats);
    const p0 = squadValue(r.plan, 0, squad.value, plat), p3 = squadValue(r.plan, 3, squad.value, plat);
    return { r, p, d, gain: (p3 - p0) / TRACES[3] * 100, lack: lacking(r) };
  });
  const key = { plat: (x: typeof list[number]) => x.p, ducats: (x: typeof list[number]) => x.d, upgrade: (x: typeof list[number]) => x.gain, missing: (x: typeof list[number]) => x.lack }[sort.value];
  return list.sort((a, b) => key(b) - key(a)).slice(0, 80);
});

onMounted(async () => {
  try { inv.value = new Set((await api.inventory()).map((i: InvRow) => i.slug)); } catch {}
  await load();
  if (data.value && !data.value.synced) onlyOwned.value = false, await load();
  off = await api.on<[number, number]>("relic-prices", ([done, total]) => {
    progress.value = done < total ? `fetching prices ${done}/${total}` : "";
    clearTimeout(reloadTimer);
    reloadTimer = window.setTimeout(load, 800);
  });
});
onUnmounted(() => off?.());
</script>

<template>
  <div class="card">
    <div class="row" style="margin-bottom:10px">
      <label class="row"><input type="checkbox" v-model="onlyOwned" @change="load" :disabled="!data?.synced" /> only relics I own</label>
      <span class="muted">squad</span>
      <div class="modes"><button v-for="n in 4" :key="n" :class="{ on: squad === n }" @click="squad = n">{{ n }}</button></div>
      <span class="muted">refined</span>
      <div class="modes"><button v-for="(r, i) in REF" :key="r" :class="{ on: refine === i }" @click="refine = i">{{ r }}</button></div>
      <span class="muted">sort</span>
      <select v-model="sort">
        <option value="plat">plat per crack</option>
        <option value="ducats">ducats per crack</option>
        <option value="upgrade">best to refine (plat per 100 traces)</option>
        <option value="missing">most rewards I lack</option>
      </select>
    </div>
    <div class="row" style="margin-bottom:10px">
      <div class="modes">
        <button v-for="[k, l] in ([['all', 'all'], ['available', 'in drop tables'], ['vaulted', 'vaulted']] as const)" :key="k" :class="{ on: vault === k }" @click="vault = k">{{ l }}</button>
      </div>
      <label class="row"><input type="checkbox" v-model="many" /> 10+ copies</label>
      <label class="row"><input type="checkbox" v-model="complete" /> all rewards owned</label>
      <span class="muted">{{ progress || (data?.pending ? `${data.pending} prices still loading` : "") }}</span>
    </div>
    <p v-if="data && !data.synced" class="muted">Turn on the inventory sync (Settings) to see which relics you own; until then every relic is listed.</p>
    <p v-if="err" class="error">{{ err }}</p>
    <table v-if="rows.length">
      <thead><tr><th>Relic</th><th title="Intact / Exceptional / Flawless / Radiant">Owned I·E·F·R</th><th>Plat</th><th>Ducats</th><th title="Extra plat per crack for 100 void traces (intact to radiant)">Refine gain</th><th>Lacking</th></tr></thead>
      <tbody>
        <tr v-for="x in rows" :key="x.r.name" style="cursor:pointer" @click="emit('pick', x.r.name)">
          <td>{{ x.r.name }} <span v-if="x.r.vaulted" class="tag vaulted">vaulted</span></td>
          <td class="muted">{{ x.r.owned.join(" · ") }}</td>
          <td class="plat">{{ x.p.toFixed(1) }}p</td>
          <td class="ducat">{{ x.d.toFixed(0) }}</td>
          <td :class="x.gain > 0 ? 'ok' : 'muted'">+{{ x.gain.toFixed(1) }}p</td>
          <td>{{ x.lack || "–" }}</td>
        </tr>
      </tbody>
    </table>
    <p class="muted" style="margin:8px 0 0">With a squad, everyone opens the same relic and you take the best reward shown, so value goes up with each player. Prices are the cheapest online sellers.</p>
  </div>
</template>
