<script setup lang="ts">
import { onMounted, ref } from "vue";
import { api, fmtTs, type RelicHit, type RelicPlan, type RelicRun } from "../api";
import ItemIcon from "../components/ItemIcon.vue";
import MyRelics from "../components/MyRelics.vue";

const REF = ["Intact", "Exceptional", "Flawless", "Radiant"];
const want = ref("");
const hits = ref<RelicHit[]>([]);
const relics = ref<string[]>([]);
const pick = ref("");
const plan = ref<RelicPlan | null>(null);
const err = ref("");
const busy = ref(false);
const runs = ref<RelicRun[]>([]);

async function findRelics() {
  err.value = "";
  try { hits.value = await api.relicsFor(want.value); } catch (e) { err.value = String(e); }
}
async function loadPlan(r = pick.value) {
  if (!r) return;
  pick.value = r; busy.value = true; err.value = "";
  try { plan.value = await api.relicPlan(r); } catch (e) { err.value = String(e); } finally { busy.value = false; }
}
onMounted(async () => {
  try { relics.value = await api.relicList(); } catch (e) { err.value = String(e); }
  runs.value = await api.relicRuns(15).catch(() => []);
});
</script>

<template>
  <h1>Relic Planner</h1>
  <p v-if="err" class="error">{{ err }}</p>
  <div class="grid" style="grid-template-columns: 1fr 1fr">
    <div class="card">
      <h2 style="margin-top:0">I want an item</h2>
      <div class="row">
        <input v-model="want" placeholder="e.g. Nikana Prime Blade" style="flex:1" @keyup.enter="findRelics" />
        <button class="btn primary" @click="findRelics">Find relics</button>
      </div>
      <table v-if="hits.length" style="margin-top:10px">
        <thead><tr><th>Relic</th><th>Rarity</th><th v-for="r in REF" :key="r">{{ r.slice(0, 3) }}</th></tr></thead>
        <tbody>
          <tr v-for="h in hits" :key="h.relic" style="cursor:pointer" @click="loadPlan(h.relic)">
            <td>{{ h.relic }} <span v-if="h.vaulted" class="tag vaulted">vaulted</span></td><td><span class="tag" :class="h.rarity.toLowerCase()">{{ h.rarity }}</span></td>
            <td v-for="(c, i) in h.chance" :key="i">{{ c.toFixed(1) }}%</td>
          </tr>
        </tbody>
      </table>
    </div>
    <div class="card">
      <h2 style="margin-top:0">What is this relic worth</h2>
      <div class="row">
        <input list="relics" v-model="pick" placeholder="Lith A1" style="flex:1" @change="loadPlan()" />
        <datalist id="relics"><option v-for="r in relics" :key="r" :value="r" /></datalist>
        <button class="btn primary" :disabled="busy" @click="loadPlan()">Plan</button>
      </div>
      <template v-if="plan">
        <div class="grid cols-4" style="margin:12px 0">
          <div class="card" v-for="(r, i) in REF" :key="r" style="padding:10px">
            <div class="muted" style="font-size:12px">{{ r }}</div>
            <div class="plat">{{ plan.expected_plat[i].toFixed(1) }}p</div>
            <div class="ducat">{{ plan.expected_ducats[i].toFixed(0) }} ducats</div>
          </div>
        </div>
        <table>
          <thead><tr><th>Drop</th><th>Rarity</th><th>Rad %</th><th>Plat</th><th>Ducats</th></tr></thead>
          <tbody>
            <tr v-for="d in plan.drops" :key="d.item">
              <td><span class="row" style="gap:10px;flex-wrap:nowrap"><ItemIcon :slug="d.slug" /><span>{{ d.item }}</span><span v-if="d.vaulted" class="tag vaulted">vaulted</span></span></td><td><span class="tag" :class="d.rarity.toLowerCase()">{{ d.rarity }}</span></td>
              <td>{{ d.chance[3].toFixed(1) }}%</td><td class="plat">{{ d.plat ?? "–" }}</td><td class="ducat">{{ d.ducats || "–" }}</td>
            </tr>
          </tbody>
        </table>
      </template>
    </div>
  </div>

  <h2>My relics</h2>
  <MyRelics @pick="(r) => loadPlan(r)" />

  <h2>Recent relic runs</h2>
  <table v-if="runs.length">
    <thead><tr><th>When</th><th>Relic</th><th>Rewards seen</th></tr></thead>
    <tbody>
      <tr v-for="r in runs" :key="r.id" style="cursor:pointer" @click="r.relic && loadPlan(r.relic)"><td class="muted">{{ fmtTs(r.ts) }}</td><td>{{ r.relic || "?" }}</td><td>{{ r.rewards.join(" · ") }}</td></tr>
    </tbody>
  </table>
  <p v-else class="muted">Runs show up here after the first relic reward screen.</p>
</template>
