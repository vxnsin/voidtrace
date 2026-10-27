<script setup lang="ts">
import { ref } from "vue";
import { api, type MarketItem, type PriceSummary } from "../api";
import ItemIcon from "./ItemIcon.vue";

const props = defineProps<{ slug: string; name: string; icon?: string; ducats?: number }>();
const hover = ref(false);
const price = ref<PriceSummary | null>(null);
const item = ref<MarketItem | null>(null);
let timer: number | undefined;

async function enter() {
  timer = window.setTimeout(async () => {
    hover.value = true;
    if (!price.value) price.value = await api.marketPrice(props.slug).catch(() => null);
    if (!item.value && !props.icon) item.value = (await api.marketSearch(props.name, 3)).find((i) => i.slug === props.slug) ?? null;
  }, 250);
}
function leave() { clearTimeout(timer); hover.value = false; }
const icon = () => props.icon ?? item.value?.icon;
</script>

<template>
  <span class="il" @mouseenter="enter" @mouseleave="leave">
    <ItemIcon :slug="slug" :icon="icon()" :size="24" />
    <a href="#" @click.prevent="api.openUrl(api.itemUrl(slug))">{{ name }}</a>
    <span class="card" v-if="hover">
      <img v-if="icon()" :src="api.iconUrl(icon()!)" width="64" height="64" />
      <span class="body">
        <b>{{ name }}</b>
        <span v-if="price"><span class="plat">{{ price.sell ?? "–" }}p</span> sell · {{ price.buy ?? "–" }}p buy · {{ price.sell_count }} online</span>
        <span v-else class="muted">loading…</span>
        <span class="ducat" v-if="(ducats ?? price?.ducats)">{{ ducats ?? price?.ducats }} ducats</span>
        <span class="muted">click → warframe.market</span>
      </span>
    </span>
  </span>
</template>

<style scoped>
.il { position: relative; display: inline-flex; align-items: center; gap: 10px; }
a { color: inherit; text-decoration: none; border-bottom: 1px dotted var(--dim); }
a:hover { color: var(--accent2); border-bottom-color: var(--accent); }
.card { position: absolute; left: 0; top: 100%; z-index: 20; display: flex; gap: 10px; padding: 10px; background: var(--bg2); border: 1px solid var(--dim); min-width: 280px; pointer-events: none; }
.card img { background: var(--bg3); filter: none; opacity: 1; }
.body { display: flex; flex-direction: column; gap: 3px; font-size: 11px; }
</style>
