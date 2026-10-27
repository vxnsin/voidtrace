<script setup lang="ts">
import { onMounted, ref } from "vue";
import { listingPrompt } from "../api";
import { extra, type SetInfo } from "../extra";
import ItemIcon from "./ItemIcon.vue";

// Sets I own parts of; a set I could hand over gets a sell button.
defineProps<{ loggedIn: boolean }>();
const sets = ref<SetInfo[]>([]);
const err = ref("");
const all = ref(false);
async function load() { try { sets.value = await extra.sets(); } catch (e) { err.value = String(e); } }
defineExpose({ load });
onMounted(load);
</script>

<template>
  <div class="card">
    <p v-if="err" class="error">{{ err }}</p>
    <p v-else-if="!sets.length" class="muted" style="margin:0">No set parts in the inventory yet.</p>
    <div v-for="s in all ? sets : sets.slice(0, 12)" :key="s.slug" class="set">
      <span class="row" style="gap:10px"><ItemIcon :slug="s.slug" /><b>{{ s.name }}</b><span v-if="s.complete" class="ok">{{ s.complete }} complete</span></span>
      <span class="parts">
        <span v-for="p in s.parts" :key="p.slug" class="pill" :class="{ got: p.have >= p.need }" :title="p.name">{{ p.name.replace(s.name.replace(/ Set$/, ''), '').trim() || p.name }} {{ p.have }}/{{ p.need }}</span>
      </span>
      <button v-if="s.complete && loggedIn" class="btn primary" @click="listingPrompt = { mode: 'sell', slug: s.slug }">Sell set</button>
    </div>
    <button v-if="sets.length > 12" class="btn" @click="all = !all">{{ all ? "Show fewer" : `Show all ${sets.length}` }}</button>
  </div>
</template>

<style scoped>
.set { display: grid; grid-template-columns: minmax(220px, 1fr) 2fr auto; gap: 12px; align-items: center; padding: 6px 0; border-bottom: 1px dotted var(--border); }
.set b { color: var(--accent2); font-weight: 500; }
.parts { display: flex; flex-wrap: wrap; gap: 4px; }
.pill { font-size: 10.5px; color: var(--muted); }
.pill.got { color: var(--ok); border-color: color-mix(in srgb, var(--ok) 40%, transparent); }
</style>