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
.wf { background: rgba(7, 7, 8, var(--wo, 0.88)); border: 1px solid #2a2a2e; color: #b4b4b4; font: 11px/1.45 var(--mono); padding: 8px 10px; min-width: 170px; user-select: none; }
.wf .t { display: flex; justify-content: space-between; gap: 12px; color: #e8e8e8; text-transform: uppercase; letter-spacing: .12em; font-size: 10px; margin-bottom: 6px; }
.wf .t i { font-style: normal; color: #4e4e52; }
.wf .dim { color: #4e4e52; }
.wf .hi { color: #e8e8e8; }
.wf .red { color: #c8102e; }
.wf .gold { color: #b89a5a; }
.wf .kv { display: grid; grid-template-columns: 1fr auto; gap: 1px 16px; }
.wf .kv > :nth-child(even) { color: #e8e8e8; text-align: right; }
</style>
