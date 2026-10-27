<script setup lang="ts">
import { ref, watchEffect } from "vue";
import { api } from "../api";

// Item picture from the warframe.market catalogue (by slug or icon path), or a
// ready URL (PublicExport images). Keeps its box when there is no picture.
const props = withDefaults(defineProps<{ slug?: string | null; icon?: string | null; src?: string | null; size?: number }>(), { size: 28 });
const url = ref<string | null>(null);
watchEffect(async () => {
  if (props.src) url.value = props.src;
  else if (props.icon) url.value = api.iconUrl(props.icon);
  else url.value = props.slug ? await api.iconFor(props.slug) : null;
});
</script>

<template>
  <span class="ii" :style="{ width: size + 'px', height: size + 'px' }">
    <img v-if="url" :src="url" :width="size" :height="size" loading="lazy" alt="" />
  </span>
</template>

<style scoped>
.ii { display: inline-flex; flex: none; align-items: center; justify-content: center; background: var(--bg3); vertical-align: middle; }
.ii img { object-fit: contain; filter: none; opacity: 1; }
</style>
