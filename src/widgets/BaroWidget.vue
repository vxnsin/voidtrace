<script setup lang="ts">
import { computed, reactive, watch } from "vue";
import { api } from "../api";
import { left, live } from "./live";
import type { WidgetCfg } from "./registry";
import WidgetFrame from "./WidgetFrame.vue";

const props = defineProps<{ cfg: WidgetCfg }>();
const baro = computed(() => live.world?.voidTrader);
const here = computed(() => !!baro.value && Date.parse(baro.value.activation) <= live.now && live.now < Date.parse(baro.value.expiry));

// Plat value of his offer, looked up once per item.
const prices = reactive<Record<string, number | null>>({});
watch(() => baro.value?.inventory, async (inv) => {
  for (const it of inv ?? []) {
    if (it.item in prices) continue;
    prices[it.item] = null;
    const hit = (await api.marketSearch(it.item, 3).catch(() => [])).find((m) => m.name.toLowerCase() === it.item.toLowerCase());
    if (hit) prices[it.item] = (await api.marketPrice(hit.slug).catch(() => null))?.sell ?? null;
  }
}, { immediate: true });
const offer = computed(() =>
  [...(baro.value?.inventory ?? [])].sort((a, b) => (prices[b.item] ?? -1) - (prices[a.item] ?? -1)).slice(0, props.cfg.size === "compact" ? 0 : 6),
);
</script>

<template>
  <WidgetFrame title="Baro Ki'Teer" :meta="baro?.location ?? ''">
    <div v-if="!baro" class="dim">loading…</div>
    <template v-else>
      <div class="kv">
        <span>{{ here ? "Leaves in" : "Arrives in" }}</span><span>{{ left(here ? baro.expiry : baro.activation) }}</span>
      </div>
      <div v-if="here && offer.length" class="offer">
        <template v-for="o in offer" :key="o.item">
          <span>{{ o.item }}</span><span class="gold">{{ o.ducats }}d</span><span class="hi">{{ prices[o.item] != null ? prices[o.item] + "p" : "–" }}</span>
        </template>
      </div>
    </template>
  </WidgetFrame>
</template>

<style scoped>
.offer { display: grid; grid-template-columns: 1fr auto auto; gap: 1px 10px; margin-top: 6px; }
.offer > :nth-child(3n) { text-align: right; }
</style>
