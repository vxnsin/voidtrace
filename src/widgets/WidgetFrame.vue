<script setup lang="ts">
defineProps<{ title: string; meta?: string }>();
</script>

<template>
  <div class="wf">
    <div class="t"><span>{{ title }}</span><i v-if="meta">{{ meta }}</i></div>
    <slot />
  </div>
</template>

<style>
/* Shared widget look: the same in the game overlay and in the app. */
.wf { /* per-widget opacity (--wo) scaled by the theme's (--two, .88 = as set) */
  background: color-mix(in srgb, var(--wbg) calc(min(1, var(--wo, .88) * var(--two, .88) / .88) * 100%), transparent); border: 1px solid var(--dim); color: var(--fg); font: 11px/1.45 var(--mono); padding: 8px 10px; min-width: 170px; user-select: none; }
.wf .t { display: flex; justify-content: space-between; gap: 12px; color: var(--accent2); text-transform: uppercase; letter-spacing: .12em; font-size: 10px; margin-bottom: 6px; }
.wf .t i { font-style: normal; color: var(--muted); }
.wf .dim { color: var(--muted); }
.wf .hi { color: var(--accent2); }
.wf .red { color: var(--accent); }
.wf .gold { color: var(--warn); }
.wf .kv { display: grid; grid-template-columns: 1fr auto; gap: 1px 16px; }
.wf .kv > :nth-child(even) { color: var(--accent2); text-align: right; }
</style>
