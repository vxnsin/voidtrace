<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { api } from "../api";
import { relicOverview, squadValue, type RelicRow } from "../relics";
import { live } from "./live";
import type { WidgetCfg } from "./registry";
import WidgetFrame from "./WidgetFrame.vue";

// Best relics I own, for the relic selection screen before a fissure. Best
// refinement I have of each, valued for a full squad.
const props = defineProps<{ cfg: WidgetCfg }>();
const rows = ref<RelicRow[]>([]);
const loaded = ref(false);
const inApp = !location.hash.startsWith("#/overlay");

async function load() {
  try { rows.value = (await relicOverview(true)).rows; } catch { rows.value = []; }
  loaded.value = true;
}
// Each trigger on its own: or-ed together with inApp the value never changes.
watch([() => live.relicSelect, () => live.editing], ([sel, edit]) => (sel || edit || inApp) && load(), { immediate: true });
const offs: (() => void)[] = [];
onMounted(async () => {
  offs.push(await api.on("inventory-synced", () => load()));
  // Prices for a cold cache arrive in the background.
  offs.push(await api.on("relic-prices", () => load()));
});
onUnmounted(() => offs.forEach((f) => f()));

const best = computed(() => {
  const by = props.cfg.relicSort ?? "plat";
  return rows.value
    .map((r) => {
      const ref = [3, 2, 1, 0].find((i) => r.owned[i] > 0) ?? 0;
      const v = squadValue(r.plan, ref, 4, (d) => (by === "ducats" ? d.ducats : d.plat ?? 0));
      return { name: r.name, ref, v, count: r.owned[ref] };
    })
    .sort((a, b) => b.v - a.v)
    .slice(0, props.cfg.size === "compact" ? 3 : 6);
});
const REF = ["Int", "Exc", "Fla", "Rad"];
</script>

<template>
  <WidgetFrame title="Relic picks" :meta="cfg.relicSort === 'ducats' ? 'ducats' : 'plat'">
    <div v-if="best.length" class="kv">
      <template v-for="b in best" :key="b.name">
        <span>{{ b.name }} <span class="dim">{{ REF[b.ref] }} ×{{ b.count }}</span></span>
        <span>{{ b.v.toFixed(cfg.relicSort === "ducats" ? 0 : 1) }}{{ cfg.relicSort === "ducats" ? "" : "p" }}</span>
      </template>
    </div>
    <div v-else class="dim" style="font-size:10px">{{ loaded ? "needs the inventory sync" : "…" }}</div>
  </WidgetFrame>
</template>
