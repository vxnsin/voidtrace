<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { api, type MasterySummary } from "../api";
import ItemIcon from "../components/ItemIcon.vue";

const data = ref<MasterySummary | null>(null);
const q = ref("");
const filter = ref<"all" | "owned" | "missing" | "mastered">("all");
const cat = ref("");
const err = ref("");

const cats = computed(() => Array.from(new Set(data.value?.items.map((i) => i.category) ?? [])).sort());
const items = computed(() => {
  const n = q.value.toLowerCase();
  return (data.value?.items ?? []).filter((i) => {
    if (cat.value && i.category !== cat.value) return false;
    if (filter.value === "owned" && !i.owned) return false;
    if (filter.value === "missing" && i.owned) return false;
    if (filter.value === "mastered" && !i.mastered) return false;
    return !n || i.name.toLowerCase().includes(n);
  });
});

onMounted(async () => {
  try { data.value = await api.mastery(); } catch (e) { err.value = String(e); }
});
</script>

<template>
  <h1>Foundry</h1>
  <p class="muted">Everything masterable in the game, crossed with your profile XP. Load your profile in Mastery first.</p>
  <p v-if="err" class="error">{{ err }}</p>
  <div class="row" style="margin-bottom: 12px">
    <input v-model="q" placeholder="Search…" style="width: 240px" />
    <select v-model="cat"><option value="">All categories</option><option v-for="c in cats" :key="c">{{ c }}</option></select>
    <select v-model="filter">
      <option value="all">All</option><option value="owned">Owned</option><option value="missing">Not owned</option><option value="mastered">Mastered</option>
    </select>
    <span class="muted">{{ items.length }} items</span>
  </div>
  <table>
    <thead><tr><th></th><th>Name</th><th>Category</th><th>XP</th><th>Status</th></tr></thead>
    <tbody>
      <tr v-for="i in items" :key="i.unique_name">
        <td><ItemIcon :src="i.image" /></td>
        <td>{{ i.name }}</td>
        <td class="muted">{{ i.category }}</td>
        <td style="min-width:140px">
          <div class="bar"><i :style="{ width: (i.xp / i.max_xp) * 100 + '%' }"></i></div>
          <small class="muted">{{ i.xp }} / {{ i.max_xp }}</small>
        </td>
        <td><span class="ok" v-if="i.mastered">Mastered</span><span v-else-if="i.owned" class="muted">Owned</span><span v-else class="muted">—</span></td>
      </tr>
    </tbody>
  </table>
</template>
