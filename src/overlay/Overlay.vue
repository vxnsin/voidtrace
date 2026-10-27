<script setup lang="ts">
import { onMounted, ref } from "vue";
import { api, type RewardHit } from "../api";
import ItemIcon from "../components/ItemIcon.vue";

const hits = ref<RewardHit[]>([]);
const error = ref("");
onMounted(() => {
  document.body.classList.add("overlay");
  api.on<RewardHit[]>("rewards", (h) => { hits.value = h; error.value = ""; });
  api.on<string>("overlay-error", (e) => (error.value = e));
});
</script>

<template>
  <div class="ov">
    <div v-if="error" class="ov-card bad">{{ error }}</div>
    <div v-for="h in hits" :key="h.raw" class="ov-card" :class="{ best: h.best }">
      <ItemIcon :slug="h.price?.slug" :size="48" class="pic" />
      <div class="txt">
        <div class="head">{{ h.best ? ">" : "·" }} {{ h.price?.name ?? h.raw }}<span v-if="h.price?.vaulted" class="vault">vaulted</span></div>
        <div class="nums">
          <span class="p">{{ h.price?.sell ?? "??" }}p</span>
          <span class="d">{{ h.price?.ducats ?? "??" }}d</span>
        </div>
        <div class="sub" v-if="h.price">{{ h.price.sell_count }} online · buy {{ h.price.buy ?? "--" }}p</div>
        <div class="sub" v-else>no market match</div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.ov { display: flex; gap: 10px; justify-content: center; padding: 10px; font-family: var(--mono); }
.ov-card { display: flex; gap: 10px; align-items: center; }
.pic { background: rgba(255, 255, 255, .04) !important; }
.txt { flex: 1; min-width: 0; }
.ov-card { flex: 1 1 0; min-width: 0; max-width: 340px; background: rgba(7, 7, 8, .92); border: 1px solid #2a2a2e; padding: 10px 12px; color: #b4b4b4; font-size: 11px; letter-spacing: .04em; }
.ov-card.best { border-color: #c8102e; }
.ov-card.bad { border-color: #c8102e; color: #c8102e; }
.head { color: #e8e8e8; text-transform: uppercase; letter-spacing: .1em; margin-bottom: 6px; }
.best .head { color: #c8102e; }
.nums { display: flex; justify-content: space-between; font-size: 18px; }
.p { color: #e8e8e8; } .d { color: #b89a5a; }
.sub { color: #4e4e52; font-size: 10px; margin-top: 4px; }
.vault { margin-left: 8px; padding: 0 5px; border: 1px solid #4a3f28; color: #b89a5a; font-size: 9px; letter-spacing: .08em; }
</style>
