<script setup lang="ts">
import { onMounted, ref, shallowRef } from "vue";
import { getVersion } from "@tauri-apps/api/app";
import { api, listingPrompt, tradePrompt, type Status, type GameEvent } from "./api";
import Intro from "./intro/Intro.vue";
import TradeDialog from "./components/TradeDialog.vue";
import NewListing from "./components/NewListing.vue";
import Setup from "./components/Setup.vue";
import Foundry from "./views/Foundry.vue";
import Inventory from "./views/Inventory.vue";
import Mastery from "./views/Mastery.vue";
import RelicPlanner from "./views/RelicPlanner.vue";
import Rivens from "./views/Rivens.vue";
import Trading from "./views/Trading.vue";
import Market from "./views/Market.vue";
import Chat from "./views/Chat.vue";
import Widgets from "./views/Widgets.vue";
import Settings from "./views/Settings.vue";

const tabs = [
  { id: "foundry", label: "Foundry", c: Foundry },
  { id: "inventory", label: "Inventory", c: Inventory },
  { id: "mastery", label: "Mastery", c: Mastery },
  { id: "relics", label: "Relic Planner", c: RelicPlanner },
  { id: "rivens", label: "Riven Explorer", c: Rivens },
  { id: "trading", label: "Trading", c: Trading },
  { id: "market", label: "Warframe.Market", c: Market },
  { id: "chat", label: "WFM Chat", c: Chat },
  { id: "widgets", label: "Widgets", c: Widgets },
  { id: "settings", label: "Settings", c: Settings },
];
const active = ref("foundry");
const current = shallowRef(tabs[0].c);
const status = ref<Status | null>(null);
const lastEvent = ref("");
const intro = ref(true);
const clock = ref("");
const version = ref("");
const setup = ref(false);
function setupDone() { setup.value = false; refresh(); }

function go(id: string) {
  active.value = id;
  current.value = tabs.find((t) => t.id === id)!.c;
}
async function refresh() {
  status.value = await api.status();
}
function tick() {
  clock.value = new Date().toLocaleTimeString("en-GB", { hour12: false });
}
onMounted(async () => {
  await refresh();
  tick();
  version.value = (await getVersion()).replace(/\.0$/, "");
  setup.value = (await api.getSetting("setup_done")) !== "1";
  api.on("ready", refresh);
  api.on<GameEvent>("game-event", (e) => {
    if (e.kind === "game_focus") return;
    lastEvent.value = e.kind + (e.name ? ` ${e.name}` : "") + (e.relic ? ` ${e.relic}` : "");
    if (e.kind === "logged_in") refresh();
    if (e.kind === "trade_complete" && !tradePrompt.value) tradePrompt.value = {};
  });
  setInterval(refresh, 10000);
  setInterval(tick, 1000);
});
</script>

<template>
  <Intro v-if="intro" :backend-ok="!!status" @done="intro = false" />
  <div class="shell" :class="{ boot: !intro }">
    <nav class="nav">
      <div class="brand"><b>VOIDTRACE</b> / v{{ version }}<br /><span class="dim">tenno companion</span></div>
      <button v-for="t in tabs" :key="t.id" :class="{ active: active === t.id }" @click="go(t.id)">{{ t.label }}</button>
      <div class="foot" v-if="status">
        <div><span class="dot" :class="{ on: status.ee_log_exists }"></span>ee.log</div>
        <div><span class="dot" :class="{ on: status.export_loaded }"></span>game data</div>
        <div><span class="dot" :class="{ on: status.market_items > 0 }"></span>market {{ status.market_items }}</div>
        <div><span class="dot" :class="{ on: status.wfm_logged_in }"></span>wfm session</div>
        <div v-if="status.player">operator · {{ status.player }}</div>
        <div v-if="lastEvent" class="ev">&gt; {{ lastEvent }}</div>
        <div class="clock">{{ clock }}</div>
      </div>
    </nav>
    <main class="page">
      <component :is="current" :status="status" @refresh="refresh" />
    </main>
  </div>
  <Setup v-if="!intro && setup" @done="setupDone" />
  <TradeDialog v-if="tradePrompt" />
  <NewListing v-if="listingPrompt" />
</template>

<style scoped>
.shell { opacity: 0; }
.shell.boot { animation: boot .5s steps(6) forwards; }
@keyframes boot { from { opacity: 0; } to { opacity: 1; } }
.dim { color: var(--muted); letter-spacing: .06em; }
.ev { color: var(--fg2); text-transform: none; letter-spacing: 0; }
.clock { color: var(--dim); margin-top: 6px; }
</style>
