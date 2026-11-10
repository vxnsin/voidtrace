<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import { api, listingPrompt } from "../api";
import { extra, type MasteryRoutes } from "../extra";
import ItemIcon from "./ItemIcon.vue";

// Two ways to more mastery: build from relics I already own, or buy the
// missing parts (cheapest first). Plus intrinsics, 1500 mastery per rank.
const data = ref<MasteryRoutes | null>(null);
const err = ref("");
const tab = ref<"relics" | "plat" | "intrinsics">("relics");
const progress = ref("");
let off: (() => void) | undefined;
let t: number | undefined;
async function load() { try { data.value = await extra.mastery(); } catch (e) { err.value = String(e); } }
const pct = (v: number) => `${Math.round(v * 100)}%`;
const intrinsicMr = () => (data.value?.intrinsics ?? []).reduce((s, i) => s + Math.max(0, 10 - i.rank) * 1500, 0);

onMounted(async () => {
  await load();
  off = await api.on<[number, number]>("mastery-prices", ([d, n]) => {
    progress.value = d < n ? `fetching prices ${d}/${n}` : "";
    clearTimeout(t);
    t = window.setTimeout(load, 800);
  });
});
onUnmounted(() => off?.());
</script>

<template>
  <h2>Ways to more mastery</h2>
  <div class="card">
    <div class="row" style="margin-bottom:10px">
      <div class="modes">
        <button :class="{ on: tab === 'relics' }" @click="tab = 'relics'">from my relics</button>
        <button :class="{ on: tab === 'plat' }" @click="tab = 'plat'">with platinum</button>
        <button :class="{ on: tab === 'intrinsics' }" @click="tab = 'intrinsics'">intrinsics</button>
      </div>
      <span class="muted">{{ progress || (data?.pending ? `${data.pending} prices still loading` : "") }}</span>
    </div>
    <p v-if="err" class="error">{{ err }}</p>
    <p v-if="data && !data.synced" class="muted">These need the inventory sync (Settings): owned relics, blueprints and parts.</p>

    <template v-if="data && tab === 'relics'">
      <p v-if="!data.relics.length" class="muted" style="margin:0">Nothing unmastered can be completed from the relics you own.</p>
      <table v-else>
        <thead><tr><th>Item</th><th>All parts</th><th>Parts (chance from your relics, solo)</th></tr></thead>
        <tbody>
          <tr v-for="r in data.relics" :key="r.unique_name">
            <td>{{ r.name }}</td>
            <td :class="r.chance_all > 0.8 ? 'ok' : r.chance_all > 0.4 ? '' : 'muted'">{{ pct(r.chance_all) }}</td>
            <td><span v-for="p in r.parts" :key="p.name" class="pill" style="margin:0 4px 4px 0" :class="{ have: p.have }">{{ p.name }} <span class="muted">{{ p.have ? "owned" : pct(p.chance) }}</span></span></td>
          </tr>
        </tbody>
      </table>
    </template>

    <template v-if="data && tab === 'plat'">
      <table>
        <thead><tr><th></th><th>Item</th><th>Missing parts</th><th>Total</th><th></th></tr></thead>
        <tbody>
          <tr v-for="r in data.plat" :key="r.unique_name">
            <td><ItemIcon :src="r.image" /></td>
            <td>{{ r.name }} <span class="muted">{{ r.category }}</span></td>
            <td class="muted">{{ r.missing.map((m) => `${m.name.replace(r.name, "").trim() || m.name} ${m.plat ?? "?"}p`).join(" · ") }}</td>
            <td class="plat">{{ r.total != null ? r.total + "p" : "…" }}</td>
            <td><button class="btn" @click="listingPrompt = { mode: 'buy', slug: r.missing[0].slug }">Buy</button></td>
          </tr>
        </tbody>
      </table>
      <p class="muted" style="margin:8px 0 0">Items you have never owned whose missing parts are all tradable. Buying the whole set can be cheaper.</p>
    </template>

    <template v-if="data && tab === 'intrinsics'">
      <div class="grid cols-3">
        <div v-for="i in data.intrinsics" :key="i.name" class="row" style="justify-content:space-between">
          <span>{{ i.name }}</span><span :class="i.rank >= 10 ? 'ok' : ''">{{ i.rank }}/10</span>
        </div>
      </div>
      <p class="muted" style="margin:10px 0 0">{{ intrinsicMr().toLocaleString() }} mastery left in intrinsics (1,500 per rank).</p>
    </template>
  </div>
</template>

<style scoped>
.pill.have { color: var(--ok); }
</style>