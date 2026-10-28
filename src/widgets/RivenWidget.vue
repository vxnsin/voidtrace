<script setup lang="ts">
import { computed, ref } from "vue";
import type { RivenAuction } from "../api";
import { live } from "./live";
import type { WidgetCfg } from "./registry";

// Riven panel, laid out like the in-game helpers people know: the card's stats
// graded, similar listings as a grid, roll advice in columns.
defineProps<{ cfg: WidgetCfg }>();
const tab = ref<"mandatory" | "optional">("mandatory");
const r = computed(() => live.riven);

const LABEL: Record<string, string> = {
  damage: "Damage", multishot: "Multishot", critical_chance: "Crit. chance", critical_damage: "Crit. damage",
  status_chance: "Status chance", status_duration: "Status duration", fire_rate: "Attack speed", punch_through: "Punch through",
  projectile_speed: "Projectile speed", zoom: "Zoom", recoil: "Recoil", magazine: "Magazine", ammo_max: "Ammo max",
  reload_speed: "Reload speed", range: "Range", combo_duration: "Combo duration", combo_chance: "Combo chance",
  initial_combo: "Initial combo", heavy_efficiency: "Heavy attack eff.", slide_crit: "Crit % on slide",
  finisher_damage: "Finisher damage", heat: "Heat", cold: "Cold", electricity: "Electricity", toxin: "Toxin",
  impact: "Impact", puncture: "Puncture", slash: "Slash", grineer: "Dmg to Grineer", corpus: "Dmg to Corpus", infested: "Dmg to Infested",
};
const label = (k: string) => LABEL[k] ?? k.replace(/_/g, " ");
const ATTR: Record<string, string> = {
  "base_damage_/_melee_damage": "Damage", "fire_rate_/_attack_speed": "Attack speed", critical_chance: "Crit. chance",
  critical_damage: "Crit. damage", critical_chance_for_slide_attack: "Crit % on slide", chance_to_gain_combo_count: "Combo chance",
  damage_vs_grineer: "Dmg to Grineer", damage_vs_corpus: "Dmg to Corpus", damage_vs_infested: "Dmg to Infested",
  heavy_attack_efficiency: "Heavy attack eff.", magazine_capacity: "Magazine", ammo_maximum: "Ammo max",
  heat_damage: "Heat", cold_damage: "Cold", electric_damage: "Electricity", toxin_damage: "Toxin",
  impact_damage: "Impact", puncture_damage: "Puncture", slash_damage: "Slash",
};
const attr = (u: string) => ATTR[u] ?? u.replace(/_/g, " ").replace(/^\w/, (c) => c.toUpperCase());
const unit = (k: string) => (k === "punch_through" || k === "range" ? "" : k === "combo_duration" ? "s" : "%");
const price = (a: RivenAuction) => a.buyout ?? a.starting;
const match = (a: RivenAuction) => {
  const p = r.value?.positives ?? [];
  if (!p.length) return 0;
  return Math.round((a.attributes.filter((t) => t.positive && p.includes(t.url_name)).length / p.length) * 100);
};
const fill = (s: { value: number; min: number; max: number }) =>
  Math.min(100, Math.max(4, ((Math.abs(s.value) - s.min) / Math.max(0.01, s.max - s.min)) * 100));
const gradeCls = (g: string) => "g" + g.replace("+", "p").replace("-", "m");
const close = () => (live.showRiven = false);
</script>

<template>
  <div class="rv wf" :class="cfg.size">
    <template v-if="r">
      <div class="hd">
        <span class="pol" :title="r.card.weapon">◆</span>
        <span class="title"><b>{{ r.card.weapon }}</b> <span class="nm">{{ r.card.name }}</span></span>
        <button class="x" title="Close" @click="close">×</button>
      </div>
      <div class="sub dim">disposition {{ r.grade.disposition.toFixed(2) }} · {{ r.prices.count }} listings{{ r.prices.median ? ` · median ${r.prices.median}p` : "" }}</div>

      <div class="stats">
        <div v-for="s in r.grade.stats" :key="s.stat" class="pill" :class="{ neg: s.negative }">
          <span class="ic">{{ s.negative ? "▼" : "▲" }}</span>
          <span class="txt">{{ s.negative ? "−" : "+" }}{{ Math.abs(s.value).toFixed(1) }}{{ unit(s.stat) }} {{ label(s.stat) }}</span>
          <span class="grade" :class="gradeCls(s.grade)">{{ s.grade }}</span>
          <span class="bar"><b :style="{ width: fill(s) + '%' }"></b></span>
        </div>
      </div>

      <template v-if="cfg.size === 'full'">
        <div class="h">Similar listings</div>
        <div v-if="r.similar.length" class="grid">
          <div v-for="a in r.similar.slice(0, 6)" :key="a.id" class="lst">
            <div class="top">
              <span class="who" :class="a.owner_status">{{ a.owner }}</span>
            </div>
            <div class="top">
              <span class="dim">{{ match(a) }}%</span>
              <span class="pp">{{ price(a) }}p</span>
            </div>
            <div v-for="t in a.attributes" :key="t.url_name" class="at" :class="{ neg: !t.positive, hit: t.positive && r.positives.includes(t.url_name) }">
              {{ t.positive ? "+" : "" }}{{ attr(t.url_name) }}
            </div>
            <div class="dim foot">{{ a.rerolls }} rolls · MR {{ a.mastery }}</div>
          </div>
        </div>
        <div v-else class="dim">No similar listings online right now.</div>

        <div class="h row">
          <span>Suggested rolls</span>
          <span class="tabs">
            <button :class="{ on: tab === 'mandatory' }" @click="tab = 'mandatory'">Mandatory</button>
            <button :class="{ on: tab === 'optional' }" @click="tab = 'optional'">Optional</button>
          </span>
        </div>
        <div class="sug">
          <div class="col">
            <span v-for="k in (tab === 'mandatory' ? r.suggest.mandatory : r.suggest.optional)" :key="k" class="chip"
              :class="{ have: r.card.stats.some((s) => !s.negative && s.stat === k) }">{{ label(k) }}</span>
          </div>
          <div class="col negs">
            <span class="cap">Best negatives</span>
            <span v-for="k in r.suggest.negatives" :key="k" class="chip neg"
              :class="{ have: r.card.stats.some((s) => s.negative && s.stat === k) }">{{ label(k) }}</span>
          </div>
        </div>
        <div class="ft">Prices shown are <b>asking prices</b></div>
      </template>
    </template>
    <div v-else class="dim">Opens by itself on the riven cycling screen, or press the riven hotkey with a riven card on screen.</div>
  </div>
</template>

<style scoped>
.rv { width: 360px; padding: 10px; }
.rv.compact { width: 300px; }
.hd { display: flex; align-items: center; gap: 8px; }
.pol { color: var(--accent); font-size: 14px; }
.title { flex: 1; min-width: 0; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; font-size: 14px; color: var(--accent2); }
.title .nm { color: color-mix(in srgb, var(--accent2) 60%, var(--accent)); }
.x { all: unset; cursor: pointer; color: var(--muted); font-size: 18px; line-height: 1; padding: 0 2px; }
.x:hover { color: var(--accent); }
.sub { font-size: 10px; margin: 2px 0 8px 22px; }
.stats { display: grid; gap: 4px; }
.pill { display: grid; grid-template-columns: 14px 1fr 26px 72px; gap: 8px; align-items: center; border: 1px solid var(--dim); border-radius: 999px; padding: 4px 10px; }
.ic { font-size: 9px; color: var(--ok, var(--accent2)); }
.pill.neg .ic, .pill.neg .txt { color: var(--accent); }
.txt { color: var(--fg); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.grade { text-align: right; font-weight: 600; }
.bar { height: 6px; border-radius: 3px; background: var(--dim); overflow: hidden; }
.bar b { display: block; height: 100%; background: var(--accent2); border-radius: 3px; }
.pill.neg .bar b { background: var(--accent); }
.gS, .gAp, .gA { color: var(--accent2); } .gAm, .gBp, .gB { color: var(--fg); } .gBm, .gCp, .gC { color: var(--warn); } .gCm, .gF { color: var(--accent); }
.h { margin: 10px 0 5px; color: var(--accent2); font-size: 11px; letter-spacing: .06em; }
.h.row { display: flex; justify-content: space-between; align-items: center; }
.grid { display: grid; grid-template-columns: repeat(3, 1fr); gap: 5px; }
.lst { border: 1px solid var(--dim); padding: 5px 6px; display: grid; gap: 1px; font-size: 10px; min-width: 0; }
.top { display: flex; justify-content: space-between; gap: 4px; }
.who { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; color: var(--fg2); }
.who.ingame { color: var(--accent2); }
.pp { color: var(--accent2); font-weight: 600; }
.at { color: var(--fg2); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.at.hit { color: var(--accent2); }
.at.neg { color: var(--accent); text-decoration: line-through; }
.foot { margin-top: 2px; font-size: 9px; }
.tabs { display: flex; }
.tabs button { all: unset; cursor: pointer; padding: 1px 8px; border: 1px solid var(--dim); margin-left: -1px; color: var(--muted); font-size: 10px; }
.tabs button.on { color: var(--accent2); border-color: var(--accent2); position: relative; }
.sug { display: grid; grid-template-columns: 1fr 1fr; gap: 6px; }
.col { display: flex; flex-wrap: wrap; gap: 4px; align-content: flex-start; border: 1px solid var(--dim); padding: 5px; }
.col.negs { border-color: color-mix(in srgb, var(--accent) 45%, transparent); }
.cap { width: 100%; color: var(--accent); font-size: 10px; }
.chip { border: 1px solid var(--dim); padding: 1px 6px; font-size: 10px; color: var(--fg2); }
.chip.have { border-color: var(--accent2); color: var(--accent2); }
.chip.neg { color: var(--accent); border-color: color-mix(in srgb, var(--accent) 40%, transparent); }
.chip.neg.have { background: color-mix(in srgb, var(--accent) 18%, transparent); }
.ft { margin-top: 8px; text-align: center; color: var(--muted); font-size: 10px; border-top: 1px solid var(--dim); padding-top: 6px; }
.ft b { color: var(--accent2); font-weight: 500; }
</style>
