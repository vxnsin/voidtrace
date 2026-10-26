<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { api, type MasterySummary } from "../api";

const playerId = ref("");
const name = ref("");
const data = ref<MasterySummary | null>(null);
const err = ref("");
const busy = ref(false);

async function load(fetchProfile = false) {
  busy.value = true; err.value = "";
  try {
    data.value = fetchProfile && playerId.value ? await api.loadProfile(playerId.value) : await api.mastery();
  } catch (e) { err.value = String(e); } finally { busy.value = false; }
}
const nextRank = computed(() => data.value ? 2500 * (data.value.rank + 1) ** 2 : 0);
const todo = computed(() => (data.value?.items ?? []).filter((i) => !i.mastered).sort((a, b) => b.xp - a.xp || a.name.localeCompare(b.name)));
const byCat = computed(() => {
  const m: Record<string, { done: number; total: number }> = {};
  for (const i of data.value?.items ?? []) {
    m[i.category] ??= { done: 0, total: 0 };
    m[i.category].total++;
    if (i.mastered) m[i.category].done++;
  }
  return Object.entries(m).sort();
});

onMounted(async () => {
  name.value = (await api.getSetting("player_name")) ?? "";
  playerId.value = (await api.getSetting("player_id")) ?? "";
  await load();
});
</script>

<template>
  <h1>Mastery Helper <span class="muted" v-if="name">· {{ name }}</span></h1>
  <div class="card" style="margin-bottom:16px">
    <div class="row">
      <input v-model="playerId" placeholder="Account id (24 hex chars)" style="width:280px" @keyup.enter="load(true)" />
      <button class="btn primary" :disabled="busy || playerId.length !== 24" @click="load(true)">{{ busy ? "Loading…" : "Load profile" }}</button>
    </div>
    <p class="muted" style="margin:8px 0 0">
      DE's profile endpoint needs your account id since U38.0.8, not the name. Log in at warframe.com, open
      <code>www.warframe.com/api/user-data</code> and copy the value of <code>user_id</code>. Profile must be public
      in game options. Don't spam this: bad ids or many requests get your IP blocked by DE for a while.
    </p>
  </div>
  <p v-if="err" class="error">{{ err }}</p>
  <template v-if="data">
    <div class="grid cols-4">
      <div class="card"><div class="stat">MR {{ data.rank }}<small>{{ data.points.toLocaleString() }} / {{ nextRank.toLocaleString() }} to next</small></div></div>
      <div class="card"><div class="stat">{{ data.mastered }} / {{ data.total }}<small>items mastered</small></div></div>
      <div class="card"><div class="stat">{{ (data.points_available - data.points).toLocaleString() }}<small>mastery points left</small></div></div>
      <div class="card"><div class="stat">{{ Math.ceil((nextRank - data.points) / 3000) }}<small>weapons to next rank</small></div></div>
    </div>
    <h2>By category</h2>
    <div class="grid cols-4">
      <div class="card" v-for="[c, v] in byCat" :key="c">
        <div style="display:flex;justify-content:space-between"><span>{{ c }}</span><span class="muted">{{ v.done }}/{{ v.total }}</span></div>
        <div class="bar" style="margin-top:6px"><i :style="{ width: (v.done / v.total) * 100 + '%' }"></i></div>
      </div>
    </div>
    <h2>Unmastered — closest first</h2>
    <table>
      <thead><tr><th>Item</th><th>Category</th><th>Progress</th><th>MR points</th></tr></thead>
      <tbody>
        <tr v-for="i in todo.slice(0, 200)" :key="i.unique_name">
          <td>{{ i.name }}</td><td class="muted">{{ i.category }}</td>
          <td style="min-width:140px"><div class="bar"><i :style="{ width: (i.xp / i.max_xp) * 100 + '%' }"></i></div></td>
          <td>+{{ i.max_xp >= 6000 ? 6000 : 3000 }}</td>
        </tr>
      </tbody>
    </table>
  </template>
</template>
