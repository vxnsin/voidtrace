<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { api } from "../api";
import { extra, type RivenSearch, type RivenSnipe, type RivenWeapon } from "../extra";

// Riven finder (current buyout auctions, DE weekly prices, which stats raise
// the price) and the sniper: saved searches that notify on new matches.
const weapons = ref<RivenWeapon[]>([]);
const q = ref("");
const weapon = computed(() => weapons.value.find((w) => w.name.toLowerCase() === q.value.trim().toLowerCase()) ?? null);
const res = ref<RivenSearch | null>(null);
const busy = ref(false);
const err = ref("");
const snipes = ref<RivenSnipe[]>([]);
const pos = ref<string[]>([]);
const max = ref<number | null>(null);
const label = (s: string) => s.replace(/_/g, " ");

async function search() {
  if (!weapon.value) { err.value = "Pick a weapon from the list."; return; }
  busy.value = true; err.value = "";
  try {
    res.value = await extra.rivenSearch(weapon.value.slug, weapon.value.name);
    if (max.value == null && res.value.prices.median) max.value = Math.round(res.value.prices.median * 0.8);
  } catch (e) { err.value = String(e); } finally { busy.value = false; }
}
const unrolled = computed(() => res.value?.weekly.find((w) => !w.rerolled));
const rolled = computed(() => res.value?.weekly.find((w) => w.rerolled));
function togglePos(s: string) { pos.value = pos.value.includes(s) ? pos.value.filter((x) => x !== s) : [...pos.value, s].slice(-3); }
async function addSnipe() {
  if (!weapon.value || !max.value) return;
  snipes.value = [...snipes.value, { weapon: weapon.value.slug, name: weapon.value.name, positives: pos.value, max: max.value }];
  await extra.setSnipes(snipes.value);
}
async function removeSnipe(i: number) { snipes.value = snipes.value.filter((_, j) => j !== i); await extra.setSnipes(snipes.value); }

onMounted(async () => {
  try { weapons.value = await extra.rivenWeapons(); } catch (e) { err.value = String(e); }
  snipes.value = await extra.snipes().catch(() => []);
});
</script>

<template>
  <h2>Riven market</h2>
  <div class="card">
    <div class="row">
      <input list="riven-weapons" v-model="q" placeholder="Weapon" style="width:260px" @keyup.enter="search" />
      <datalist id="riven-weapons"><option v-for="w in weapons" :key="w.slug" :value="w.name" /></datalist>
      <button class="btn primary" :disabled="busy" @click="search">{{ busy ? "Searching…" : "Search auctions" }}</button>
      <span v-if="weapon" class="muted">disposition {{ weapon.disposition.toFixed(2) }} · {{ weapon.riven_type }}</span>
    </div>
    <p v-if="err" class="error">{{ err }}</p>
    <template v-if="res">
      <div class="grid cols-4" style="margin-top:12px">
        <div class="stat">{{ Math.round(res.prices.trimmed_avg) }}p<small>average now · {{ res.prices.count }} buyouts, outliers dropped</small></div>
        <div class="stat">{{ Math.round(res.prices.low) }}–{{ Math.round(res.prices.high) }}p<small>usual range now</small></div>
        <div class="stat">{{ unrolled ? Math.round(unrolled.median) + "p" : "–" }}<small>unrolled · DE weekly median{{ unrolled ? `, avg ${Math.round(unrolled.avg)}p` : "" }}</small></div>
        <div class="stat">{{ rolled ? Math.round(rolled.median) + "p" : "–" }}<small>rolled · DE weekly median{{ rolled ? `, avg ${Math.round(rolled.avg)}p` : "" }}</small></div>
      </div>
      <h2>Stats that raise the price</h2>
      <div class="row">
        <span v-for="a in res.prices.attrs.slice(0, 12)" :key="a.url_name" class="chip" :class="{ on: pos.includes(a.url_name) }" @click="togglePos(a.url_name)">
          {{ label(a.url_name) }} <b :class="a.ratio >= 1 ? 'ok' : 'muted'">{{ a.ratio >= 1 ? "+" : "" }}{{ Math.round((a.ratio - 1) * 100) }}%</b>
        </span>
      </div>
      <h2>Cheapest buyouts</h2>
      <table>
        <thead><tr><th>Riven</th><th>Positives</th><th>Negative</th><th>Rolls</th><th>Price</th><th>Seller</th></tr></thead>
        <tbody>
          <tr v-for="a in res.auctions.slice(0, 20)" :key="a.id" style="cursor:pointer" @click="api.openUrl(`https://warframe.market/auction/${a.id}`)">
            <td>{{ a.name }}</td>
            <td>{{ a.attributes.filter((t) => t.positive).map((t) => `${label(t.url_name)} ${t.value}`).join(", ") }}</td>
            <td class="muted">{{ a.attributes.filter((t) => !t.positive).map((t) => label(t.url_name)).join(", ") || "–" }}</td>
            <td class="muted">{{ a.rerolls }}</td>
            <td class="plat">{{ a.buyout }}p</td>
            <td :class="a.owner_status === 'ingame' ? 'ok' : 'muted'">{{ a.owner }}</td>
          </tr>
        </tbody>
      </table>
    </template>
  </div>

  <h2>Sniper</h2>
  <div class="card">
    <div class="row">
      <span class="muted">{{ weapon ? weapon.name : "search a weapon first" }}</span>
      <span class="muted">with</span><span>{{ pos.length ? pos.map(label).join(", ") : "any stats" }}</span>
      <span class="muted">for at most</span><input type="number" min="1" v-model.number="max" style="width:80px" /><span class="muted">p</span>
      <button class="btn primary" :disabled="!weapon || !max" @click="addSnipe">Save search</button>
    </div>
    <table v-if="snipes.length" style="margin-top:8px">
      <tbody>
        <tr v-for="(s, i) in snipes" :key="i">
          <td>{{ s.name }}</td><td class="muted">{{ s.positives.length ? s.positives.map(label).join(", ") : "any stats" }}</td>
          <td class="plat">≤ {{ s.max }}p</td><td style="text-align:right"><button class="btn danger" @click="removeSnipe(i)">Remove</button></td>
        </tr>
      </tbody>
    </table>
    <p class="muted" style="margin:8px 0 0">Saved searches are checked every five minutes, one at a time. New matches show in game and go to Discord if a webhook takes "Riven sniper". No limit on searches, but each one is a request, so keep it reasonable.</p>
  </div>
</template>

<style scoped>
.chip { border: 1px solid var(--dim); padding: 2px 8px; cursor: pointer; color: var(--fg2); }
.chip.on { border-color: var(--accent); color: var(--accent2); }
.chip b { font-weight: 500; margin-left: 4px; }
</style>