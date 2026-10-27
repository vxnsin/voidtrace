<script setup lang="ts">
import ItemIcon from "../components/ItemIcon.vue";
import { live } from "./live";
import type { WidgetCfg } from "./registry";
import WidgetFrame from "./WidgetFrame.vue";

defineProps<{ cfg: WidgetCfg }>();
</script>

<template>
  <WidgetFrame title="Relic rewards" :meta="live.rewards.length ? `${live.rewards.length} cards` : 'reward screen'">
    <div v-if="live.rewards.length" class="rw">
      <div v-for="h in live.rewards" :key="h.raw" class="card" :class="{ best: h.best }">
        <ItemIcon :slug="h.price?.slug" :size="cfg.size === 'compact' ? 32 : 44" />
        <div class="txt">
          <div class="nm">{{ h.best ? "> " : "" }}{{ h.price?.name ?? h.raw }}<span v-if="h.price?.vaulted" class="vault">vaulted</span></div>
          <div class="nums"><span class="hi">{{ h.price?.sell ?? "??" }}p</span><span class="gold">{{ h.price?.ducats ?? "??" }}d</span></div>
          <div v-if="cfg.size === 'full'" class="dim sub">{{ h.price ? `${h.price.sell_count} online · buy ${h.price.buy ?? "--"}p` : "no market match" }}</div>
        </div>
      </div>
    </div>
    <div v-else class="dim">Shows itself when a relic reward screen opens.</div>
  </WidgetFrame>
</template>

<style scoped>
.rw { display: flex; gap: 8px; }
.card { display: flex; gap: 8px; align-items: center; border: 1px solid #2a2a2e; padding: 6px 8px; flex: 1 1 0; min-width: 0; max-width: 300px; }
.card.best { border-color: #c8102e; }
.txt { min-width: 0; }
.nm { color: #e8e8e8; text-transform: uppercase; font-size: 9.5px; letter-spacing: .08em; }
.best .nm { color: #c8102e; }
.nums { display: flex; justify-content: space-between; gap: 12px; font-size: 16px; }
.sub { font-size: 10px; }
.vault { margin-left: 6px; padding: 0 4px; border: 1px solid #4a3f28; color: #b89a5a; font-size: 9px; }
</style>
