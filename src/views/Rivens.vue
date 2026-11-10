<script setup lang="ts">
import { ref } from "vue";
import { api, type RivenGrade, type StatInput } from "../api";
import RivenMarket from "../components/RivenMarket.vue";

const STATS = [
  "damage", "multishot", "critical_chance", "critical_damage", "status_chance", "status_duration", "fire_rate",
  "heat", "cold", "electricity", "toxin", "impact", "puncture", "slash", "magazine", "ammo_max", "reload_speed",
  "zoom", "projectile_speed", "recoil", "punch_through", "grineer", "corpus", "infested",
  "range", "combo_duration", "slide_crit", "finisher_damage", "initial_combo", "heavy_efficiency", "combo_chance",
];
const weapon = ref("");
const stats = ref<StatInput[]>([
  { stat: "damage", value: 0, negative: false },
  { stat: "multishot", value: 0, negative: false },
  { stat: "critical_chance", value: 0, negative: false },
]);
const result = ref<RivenGrade | null>(null);
const err = ref("");

function addStat() { if (stats.value.length < 4) stats.value.push({ stat: "heat", value: 0, negative: stats.value.length === 3 }); }
function rm(i: number) { stats.value.splice(i, 1); }
async function grade() {
  err.value = "";
  try { result.value = await api.gradeRiven(weapon.value, stats.value.filter((s) => s.value)); } catch (e) { err.value = String(e); }
}
</script>

<template>
  <h1>Riven Explorer</h1>
  <p class="muted">Enter the riven exactly as shown in game (rank 8 values as printed on the card or unranked ones, either is detected; the grade compares against the roll range at the weapon's current disposition, shown unranked).</p>
  <div class="card">
    <div class="row">
      <input v-model="weapon" placeholder="Weapon (e.g. Kuva Bramma)" style="width:260px" />
      <button class="btn" :disabled="stats.length >= 4" @click="addStat">+ stat</button>
      <button class="btn primary" @click="grade">Grade</button>
    </div>
    <div class="row" v-for="(s, i) in stats" :key="i" style="margin-top:8px">
      <select v-model="s.stat"><option v-for="st in STATS" :key="st" :value="st">{{ st.replace(/_/g, " ") }}</option></select>
      <input type="number" step="0.1" v-model.number="s.value" style="width:100px" placeholder="%" />
      <label class="pill"><input type="checkbox" v-model="s.negative" /> negative</label>
      <button class="btn danger" @click="rm(i)">✕</button>
    </div>
  </div>
  <p v-if="err" class="error">{{ err }}</p>
  <div v-if="result" class="card" style="margin-top:12px">
    <div class="row" style="justify-content:space-between">
      <b>{{ result.weapon }}</b><span class="muted">Disposition {{ result.disposition.toFixed(2) }}</span>
    </div>
    <table style="margin-top:8px">
      <thead><tr><th>Stat</th><th>Value</th><th>Range</th><th>Grade</th></tr></thead>
      <tbody>
        <tr v-for="s in result.stats" :key="s.stat">
          <td>{{ s.negative ? "−" : "+" }} {{ s.stat.replace(/_/g, " ") }}</td>
          <td>{{ s.value }}%</td>
          <td class="muted">{{ s.min.toFixed(1) }} – {{ s.max.toFixed(1) }}%</td>
          <td><span class="grade" :class="s.grade">{{ s.grade }}</span></td>
        </tr>
      </tbody>
    </table>
  </div>

  <RivenMarket />
</template>
