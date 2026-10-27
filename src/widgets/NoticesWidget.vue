<script setup lang="ts">
import { computed } from "vue";
import { live } from "./live";
import type { WidgetCfg } from "./registry";

// Top-centre stack of the last notices (up to three, a few seconds each).
// With nothing to show it is invisible in game; in the app and the editor it
// shows an example so it can be placed.
const props = defineProps<{ cfg: WidgetCfg }>();
const inApp = !location.hash.startsWith("#/overlay");
const SAMPLE = [{ key: -1, kind: "cycle", title: "Cetus: night in 5 min", body: "day until 21:40", icon: undefined }];
const list = computed(() => (live.notices.length ? live.notices : inApp || live.editing ? SAMPLE : []).slice(0, props.cfg.size === "compact" ? 1 : 3));
</script>

<template>
  <TransitionGroup name="nt" tag="div" class="notices">
    <div v-for="n in list" :key="n.key" class="wf nt" :class="n.kind">
      <img v-if="n.icon" :src="n.icon" alt="" />
      <div class="txt">
        <div class="hi">{{ n.title }}</div>
        <div v-if="n.body && cfg.size === 'full'" class="dim">{{ n.body }}</div>
      </div>
    </div>
  </TransitionGroup>
</template>

<style scoped>
.notices { display: grid; gap: 6px; justify-items: center; min-width: 260px; }
.nt { display: flex; gap: 10px; align-items: center; min-width: 260px; max-width: 460px; border-left: 2px solid var(--accent); }
.nt.cycle, .nt.baro { border-left-color: var(--warn); }
.nt.trade { border-left-color: var(--ok); }
.nt img { width: 28px; height: 28px; object-fit: contain; }
.txt { min-width: 0; }
.txt .hi { letter-spacing: .04em; }
.txt .dim { font-size: 10.5px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.nt-enter-from { opacity: 0; transform: translateY(-8px); }
.nt-leave-to { opacity: 0; }
.nt-enter-active, .nt-leave-active { transition: all .25s; }
</style>
