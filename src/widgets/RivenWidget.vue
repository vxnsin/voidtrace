<script setup lang="ts">
import { live } from "./live";
import type { WidgetCfg } from "./registry";
import WidgetFrame from "./WidgetFrame.vue";

defineProps<{ cfg: WidgetCfg }>();
const label = (k: string) => k.replace(/_/g, " ");
const short = (u: string) =>
  u.replace("base_damage_/_melee_damage", "dmg").replace("fire_rate_/_attack_speed", "fire rate").replace("critical_", "crit ")
   .replace("_damage", "").replace("damage_vs_", "vs ").replace("chance_to_gain_combo_count", "combo chance").replace(/_/g, " ");
const price = (a: { buyout: number | null; starting: number }) => a.buyout ?? a.starting;
</script>

<template>
  <WidgetFrame title="Riven" :meta="live.riven ? `${live.riven.card.weapon} · dispo ${live.riven.grade.disposition.toFixed(2)}` : 'hotkey'">
    <template v-if="live.riven">
      <div class="nm">{{ live.riven.card.weapon }} <span class="hi">{{ live.riven.card.name }}</span></div>
      <div class="stats">
        <div v-for="s in live.riven.grade.stats" :key="s.stat" class="st" :class="{ neg: s.negative }">
          <span class="sign">{{ s.negative ? "−" : "+" }}</span>
          <span class="val">{{ Math.abs(s.value).toFixed(1) }}%</span>
          <span class="lbl">{{ label(s.stat) }}</span>
          <span class="grade" :class="'g' + s.grade.replace('+', 'p').replace('-', 'm')">{{ s.grade }}</span>
          <i class="bar"><b :style="{ width: Math.min(100, Math.max(0, ((Math.abs(s.value) - s.min) / Math.max(0.01, s.max - s.min)) * 100)) + '%' }"></b></i>
        </div>
      </div>
      <template v-if="cfg.size === 'full'">
        <div class="h">Similar listings <i>{{ live.riven.similar.length ? `${live.riven.similar.length} · from ${Math.min(...live.riven.similar.map(price))}p` : "none online" }}</i></div>
        <div class="list">
          <div v-for="a in live.riven.similar.slice(0, 4)" :key="a.id" class="row">
            <span class="hi">{{ price(a) }}p</span>
            <span class="dim">{{ a.rerolls }}rr</span>
            <span class="attrs">
              <span v-for="t in a.attributes" :key="t.url_name" :class="t.positive ? 'pos' : 'neg'">{{ t.positive ? "+" : "−" }}{{ short(t.url_name) }}</span>
            </span>
          </div>
        </div>
        <div class="h">Suggested rolls</div>
        <div class="sug">
          <span class="dim">must</span><span v-for="k in live.riven.suggest.mandatory" :key="k" class="chip on">{{ label(k) }}</span>
        </div>
        <div class="sug">
          <span class="dim">nice</span><span v-for="k in live.riven.suggest.optional" :key="k" class="chip">{{ label(k) }}</span>
        </div>
        <div class="sug">
          <span class="dim">neg</span><span v-for="k in live.riven.suggest.negatives" :key="k" class="chip neg">{{ label(k) }}</span>
        </div>
      </template>
    </template>
    <div v-else class="dim">Press the riven hotkey with a riven card on screen.</div>
  </WidgetFrame>
</template>

<style scoped>
.nm { color: var(--accent2); text-transform: uppercase; letter-spacing: .08em; font-size: 10px; margin-bottom: 6px; }
.stats { display: grid; gap: 3px; min-width: 260px; }
.st { display: grid; grid-template-columns: 10px 52px 1fr 26px; gap: 6px; align-items: center; }
.st .bar { grid-column: 1 / -1; height: 3px; background: var(--dim); display: block; }
.st .bar b { display: block; height: 100%; background: var(--accent2); }
.st.neg .bar b { background: var(--accent); }
.st.neg .sign, .st.neg .val { color: var(--accent); }
.val { color: var(--accent2); }
.grade { text-align: right; font-weight: 600; }
.gS, .gAp, .gA { color: var(--accent2); } .gAm, .gBp, .gB { color: var(--fg); } .gBm, .gCp, .gC { color: var(--warn); } .gCm, .gF { color: var(--accent); }
.h { margin-top: 8px; color: var(--muted); text-transform: uppercase; letter-spacing: .12em; font-size: 9.5px; display: flex; justify-content: space-between; gap: 10px; }
.h i { font-style: normal; text-transform: none; letter-spacing: 0; }
.list { display: grid; gap: 2px; margin-top: 3px; }
.row { display: flex; gap: 8px; align-items: baseline; }
.attrs { display: flex; gap: 6px; flex-wrap: wrap; font-size: 10px; }
.attrs .pos { color: var(--fg); } .attrs .neg { color: var(--accent); }
.sug { display: flex; gap: 4px; flex-wrap: wrap; align-items: center; margin-top: 3px; }
.sug .dim { width: 30px; font-size: 10px; }
.chip { border: 1px solid var(--dim); padding: 0 5px; font-size: 10px; color: var(--fg2); }
.chip.on { border-color: var(--accent2); color: var(--accent2); }
.chip.neg { border-color: color-mix(in srgb, var(--accent) 50%, transparent); color: var(--accent); }
</style>
