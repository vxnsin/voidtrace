<script setup lang="ts">
import { computed } from "vue";
import { live } from "./live";
import type { WidgetCfg } from "./registry";
import WidgetFrame from "./WidgetFrame.vue";

defineProps<{ cfg: WidgetCfg }>();
const since = computed(() => {
  const m = Math.floor((live.now / 1000 - live.session.start) / 60);
  return m >= 60 ? `${Math.floor(m / 60)}h ${String(m % 60).padStart(2, "0")}m` : `${m}m`;
});
const sign = (v: number | null, unit = "") => (v == null ? "–" : `${v >= 0 ? "+" : ""}${v}${unit}`);
</script>

<template>
  <WidgetFrame title="Session" :meta="since">
    <div class="kv">
      <span>Platinum</span><span>{{ sign(live.session.plat, "p") }}</span>
      <span>Ducats</span><span>{{ sign(live.session.ducats) }}</span>
      <template v-if="cfg.size === 'full'">
        <span>Relics opened</span><span>{{ live.session.relics }}</span>
        <span>Trades</span><span>{{ live.session.trades }}</span>
      </template>
    </div>
    <div v-if="live.session.plat == null && cfg.size === 'full'" class="dim" style="margin-top:4px;font-size:10px">plat needs inventory sync</div>
  </WidgetFrame>
</template>
