<script setup lang="ts">
import { onMounted, ref } from "vue";
import { NOTICE_KINDS, notifyApi, type DiscordHook } from "../api";

// Discord webhooks: each with its own set of notices.
const hooks = ref<DiscordHook[]>([]);
const msg = ref<Record<number, string>>({});
const HOOK = /^https:\/\/(ptb\.|canary\.)?discord(app)?\.com\/api\/webhooks\//;

onMounted(async () => (hooks.value = await notifyApi.hooks()));
async function save() {
  await notifyApi.saveHooks(hooks.value.filter((h) => HOOK.test(h.url.trim())).map((h) => ({ ...h, url: h.url.trim() })));
}
function add() { hooks.value.push({ name: `Webhook ${hooks.value.length + 1}`, url: "", kinds: ["fissure", "baro", "undercut", "price"] }); }
async function remove(i: number) { hooks.value.splice(i, 1); await save(); }
function toggle(h: DiscordHook, k: string) { h.kinds = h.kinds.includes(k) ? h.kinds.filter((x) => x !== k) : [...h.kinds, k]; save(); }
async function test(i: number) {
  const h = hooks.value[i];
  if (!HOOK.test(h.url.trim())) { msg.value[i] = "Paste the webhook URL from Discord (channel settings → Integrations → Webhooks)."; return; }
  msg.value[i] = "Sending…";
  try { await notifyApi.discordTest(h.url); msg.value[i] = "Sent. Check the channel."; await save(); }
  catch (e) { msg.value[i] = String(e); }
}
</script>

<template>
  <div class="card">
    <h2 style="margin-top:0">Discord</h2>
    <p class="muted" style="margin-top:0">Post notices to a Discord channel, for when you're away from the PC or to share fissures with your clan. At most six posts a minute per webhook.</p>
    <div v-for="(h, i) in hooks" :key="i" class="hook">
      <div class="row">
        <input v-model="h.name" style="width:150px" @change="save" />
        <input v-model="h.url" placeholder="https://discord.com/api/webhooks/…" style="flex:1;min-width:260px" @change="save" />
        <button class="btn" @click="test(i)">Test</button>
        <button class="btn danger" @click="remove(i)">Remove</button>
      </div>
      <div class="row kinds">
        <span v-for="k in NOTICE_KINDS" :key="k.kind" class="chip" :class="{ on: h.kinds.includes(k.kind) }" @click="toggle(h, k.kind)">{{ k.label }}</span>
      </div>
      <span v-if="msg[i]" class="muted">{{ msg[i] }}</span>
    </div>
    <button class="btn" @click="add">Add webhook</button>
  </div>
</template>

<style scoped>
.hook { display: grid; gap: 6px; padding: 8px 0 10px; border-bottom: 1px dotted var(--border); margin-bottom: 10px; }
.kinds { gap: 5px; }
.chip { border: 1px solid var(--dim); padding: 1px 7px; cursor: pointer; color: var(--muted); font-size: 10.5px; }
.chip.on { border-color: var(--accent); color: var(--accent2); }
</style>
