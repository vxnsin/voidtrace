<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, ref } from "vue";
import { api, type Chat, type ChatMessage, type Status } from "../api";

const props = defineProps<{ status: Status | null }>();
const chats = ref<Chat[]>([]);
const active = ref<Chat | null>(null);
const messages = ref<ChatMessage[]>([]);
const draft = ref("");
const err = ref("");
const me = ref("");
const pane = ref<HTMLElement | null>(null);
let poll: number | undefined;

async function loadChats() {
  if (!props.status?.wfm_logged_in) return;
  try { chats.value = await api.wfmChats(); err.value = ""; } catch (e) { err.value = String(e); }
}
async function open(c: Chat) {
  active.value = c;
  try {
    messages.value = await api.wfmChat(c.id);
    await nextTick();
    pane.value?.scrollTo({ top: pane.value.scrollHeight });
  } catch (e) { err.value = String(e); }
}
async function send() {
  if (!active.value || !draft.value.trim()) return;
  try {
    await api.wfmChatSend(active.value.id, draft.value.trim());
    draft.value = "";
    await open(active.value);
  } catch (e) { err.value = String(e) + " — sending may not be supported by the v1 API; open the chat on warframe.market."; }
}
const partner = (c: Chat) => c.chat_with.find((u) => u.ingame_name !== me.value) ?? c.chat_with[0];
const when = (iso: string) => iso ? new Date(iso).toLocaleTimeString("en-GB", { hour12: false }) : "";

onMounted(async () => {
  me.value = (await api.getSetting("wfm_name")) ?? "";
  await loadChats();
  poll = window.setInterval(async () => { await loadChats(); if (active.value) await open(active.value); }, 15000);
});
onUnmounted(() => clearInterval(poll));
</script>

<template>
  <h1>Warframe.Market Chat</h1>
  <p v-if="!status?.wfm_logged_in" class="muted">Log in on the Warframe.Market tab first.</p>
  <p v-if="err" class="error">{{ err }}</p>
  <div class="chat" v-if="status?.wfm_logged_in">
    <div class="list card">
      <div v-for="c in chats" :key="c.id" class="row item" :class="{ on: active?.id === c.id }" @click="open(c)">
        <span class="dot" :class="partner(c)?.status"></span>
        <span>{{ partner(c)?.ingame_name ?? "?" }}</span>
        <span class="muted" style="margin-left:auto" v-if="c.unread_count">{{ c.unread_count }} new</span>
      </div>
      <p class="muted" v-if="!chats.length">No chats.</p>
    </div>
    <div class="pane card">
      <template v-if="active">
        <div class="row" style="justify-content:space-between;margin-bottom:8px">
          <b>{{ partner(active)?.ingame_name }}</b>
          <a href="#" class="muted" @click.prevent="api.openUrl(`https://warframe.market/profile/${partner(active)?.ingame_name}`)">profile on warframe.market</a>
        </div>
        <div class="msgs" ref="pane">
          <div v-for="m in messages" :key="m.id" class="msg" :class="{ mine: m.message_from === partner(active)?.id ? false : true }">
            <span class="t">{{ when(m.send_date) }}</span>
            <span v-html="m.raw_message ?? m.message"></span>
          </div>
        </div>
        <div class="row" style="margin-top:8px">
          <input v-model="draft" style="flex:1" placeholder="Message…" @keyup.enter="send" />
          <button class="btn primary" @click="send">Send</button>
        </div>
      </template>
      <p v-else class="muted">Pick a chat. The list refreshes every 15 s.</p>
    </div>
  </div>
</template>

<style scoped>
.chat { display: grid; grid-template-columns: 260px 1fr; gap: 12px; height: calc(100vh - 110px); }
.list { overflow-y: auto; overflow-x: hidden; padding: 6px; }
.list::before, .list::after, .pane::before, .pane::after { display: none; }
.item { padding: 6px 8px; cursor: pointer; }
.item:hover, .item.on { background: var(--bg3); }
.dot { width: 6px; height: 6px; background: var(--muted); display: inline-block; }
.dot.ingame { background: var(--accent); } .dot.online { background: var(--ok); }
.pane { display: flex; flex-direction: column; }
.msgs { flex: 1; overflow: auto; display: flex; flex-direction: column; gap: 4px; }
.msg { padding: 4px 8px; border-left: 2px solid var(--dim); }
.msg.mine { border-left-color: var(--accent); }
.t { color: var(--muted); margin-right: 8px; font-size: 10px; }
</style>
