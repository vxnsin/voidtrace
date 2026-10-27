import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { ref } from "vue";

export type Status = {
  export_loaded: boolean;
  market_items: number;
  wfm_logged_in: boolean;
  player: string | null;
  ee_log: string;
  ee_log_exists: boolean;
};
export type MarketItem = { id: string; slug: string; name: string; icon: string; ducats: number; tradable: boolean; tags: string[]; vaulted: boolean };
export type PriceSummary = { slug: string; name: string; ducats: number; sell: number | null; buy: number | null; sell_count: number; vaulted: boolean };
export type ClosedOrder = { id: string; kind: "buy" | "sell"; quantity: number; platinum: number; closed: string; slug: string; name: string };
export type Order = { id: string; type: "buy" | "sell"; platinum: number; quantity: number; visible: boolean; itemId: string; rank?: number; user?: { ingameName: string; status: string; reputation: number } };
export type MasteryItem = { unique_name: string; name: string; category: string; xp: number; max_xp: number; mastered: boolean; owned: boolean; mastery_points: number; image: string | null };
export type MasterySummary = { items: MasteryItem[]; mastered: number; total: number; points: number; points_available: number; rank: number };
export type RelicHit = { relic: string; rarity: string; chance: [number, number, number, number]; vaulted: boolean };
export type DropInfo = { item: string; rarity: string; chance: [number, number, number, number]; plat: number | null; ducats: number; slug: string | null; vaulted: boolean };
export type RelicPlan = { relic: string; drops: DropInfo[]; expected_plat: number[]; expected_ducats: number[] };
export type InvRow = { slug: string; name: string; quantity: number };
export type StatInput = { stat: string; value: number; negative: boolean };
export type StatGrade = StatInput & { min: number; max: number; grade: string };
export type RivenGrade = { weapon: string; disposition: number; stats: StatGrade[] };
export type Trade = { id: number; ts: number; item: string; quantity: number; platinum: number; direction: "buy" | "sell"; partner: string };
export type TradeDone = Omit<Trade, "id" | "ts"> & { order_id: string | null; slug: string | null };
export type Datapoint = { ts: number; platinum: number; ducats: number; credits: number };
export type RelicRun = { id: number; ts: number; relic: string; rewards: string[]; chosen: string | null };
export type Activity = { type: string; details?: string; startedAt: number | string };
export type RichStatus = { status: "online" | "ingame" | "invisible"; statusSetAt?: string; statusUntil?: string; activity?: Activity | null };
export type WsEvent = { kind: "connected" | "disconnected" | "signed_in" | "status" | "raw"; reason?: string; route?: string; payload?: unknown } & Partial<RichStatus>;
export type ChatUser = { id: string; ingame_name: string; status: string; avatar?: string | null };
export type Chat = { id: string; chat_with: ChatUser[]; unread_count: number; last_update: string; messages: ChatMessage[] };
export type ChatMessage = { id: string; message: string; message_from: string; send_date: string; raw_message?: string | null };
export type SyncResult ={ platinum: number; credits: number; ducats: number; items_total: number; items_tradeable: number; seconds: number };
export type RewardHit ={ raw: string; price: PriceSummary | null; best: boolean };
export type GameEvent = { kind: string; name?: string; relic?: string; line?: string; focused?: boolean };
/** api.warframestat.us subset (see wfc-core/worldstate.rs); timestamps are ISO strings. */
export type Cycle = { activation: string; expiry: string; state: string };
export type Fissure = { id: string; expiry: string; node: string; missionType: string; enemy: string; tier: string; tierNum: number; isStorm: boolean; isHard: boolean };
export type VoidTrader = { activation: string; expiry: string; location: string; inventory: { item: string; ducats: number; credits: number }[] };
export type WorldState = { cetusCycle?: Cycle; vallisCycle?: Cycle; cambionCycle?: Cycle; zarimanCycle?: Cycle; duviriCycle?: Cycle; earthCycle?: Cycle; fissures?: Fissure[]; voidTrader?: VoidTrader };

const iconUrl = (icon: string) => `https://warframe.market/static/assets/${icon}`;
const icons = new Map<string, Promise<string | null>>();

export const api = {
  /** Catalogue picture for a slug; looked up once per session. */
  iconFor: (slug: string) => {
    if (!icons.has(slug)) {
      icons.set(slug, invoke<MarketItem | null>("market_item", { slug }).then((i) => (i?.icon ? iconUrl(i.icon) : null)).catch(() => null));
    }
    return icons.get(slug)!;
  },
  status: () => invoke<Status>("status"),
  refreshData: () => invoke<void>("refresh_data"),
  setSetting: (key: string, value: string) => invoke<void>("set_setting", { key, value }),
  setLaunchWithGame: (on: boolean) => invoke<void>("set_launch_with_game", { on }),
  gameRunning: () => invoke<boolean>("game_running"),
  gameFocused: () => invoke<boolean>("game_focused"),
  worldstate: () => invoke<WorldState | null>("worldstate"),
  overlayEdit: (on: boolean) => invoke<void>("overlay_edit", { on }),
  setOverlayHotkey: (key: string) => invoke<void>("set_overlay_hotkey", { key }),
  getSetting: (key: string) => invoke<string | null>("get_setting", { key }),
  loadProfile: (playerId: string) => invoke<MasterySummary>("load_profile", { playerId }),
  overlayPreview: () => invoke<void>("overlay_preview"),
  mastery: () => invoke<MasterySummary>("mastery"),
  relicsFor: (item: string) => invoke<RelicHit[]>("relics_for", { item }),
  relicList: () => invoke<string[]>("relic_list"),
  relicPlan: (relic: string) => invoke<RelicPlan>("relic_plan", { relic }),
  marketSearch: (q: string, limit = 25) => invoke<MarketItem[]>("market_search", { q, limit }),
  marketItemsById: (ids: string[]) => invoke<Record<string, MarketItem>>("market_items_by_id", { ids }),
  marketItem: (slug: string) => invoke<MarketItem | null>("market_item", { slug }),
  marketPrice: (slug: string) => invoke<PriceSummary>("market_price", { slug }),
  marketOrders: (slug: string) => invoke<Order[]>("market_orders", { slug }),
  wfmLogin: (email: string, password: string) => invoke<string>("wfm_login", { email, password }),
  wfmLogout: () => invoke<void>("wfm_logout"),
  wfmMyOrders: () => invoke<Order[]>("wfm_my_orders"),
  wfmClosedOrders: () => invoke<ClosedOrder[]>("wfm_closed_orders"),
  wfmCreateOrder: (itemId: string, sell: boolean, platinum: number, quantity: number, rank?: number) =>
    invoke<Order>("wfm_create_order", { itemId, sell, platinum, quantity, rank }),
  wfmUpdateOrder: (id: string, platinum: number, quantity: number, visible: boolean) =>
    invoke<Order>("wfm_update_order", { id, platinum, quantity, visible }),
  wfmDeleteOrder: (id: string) => invoke<void>("wfm_delete_order", { id }),
  wfmStatus: () => invoke<RichStatus | null>("wfm_status"),
  wfmSetStatus: (status: string, activity?: string, details?: string) => invoke<void>("wfm_set_status", { status, activity, details }),
  wfmChats: () => invoke<Chat[]>("wfm_chats"),
  wfmChat: (id: string) => invoke<ChatMessage[]>("wfm_chat", { id }),
  wfmChatSend: (id: string, message: string) => invoke<void>("wfm_chat_send", { id, message }),
  openUrl: (url: string) => invoke<void>("open_url", { url }),
  itemUrl: (slug: string) => `https://warframe.market/items/${slug}`,
  iconUrl,
  inventory: () => invoke<InvRow[]>("inventory"),
  inventorySet: (slug: string, name: string, quantity: number) => invoke<void>("inventory_set", { slug, name, quantity }),
  inventoryAdd: (slug: string, name: string, delta: number) => invoke<number>("inventory_add", { slug, name, delta }),
  inventorySync: () => invoke<SyncResult>("inventory_sync"),
  gradeRiven: (weapon: string, stats: StatInput[]) => invoke<RivenGrade>("grade_riven", { weapon, stats }),
  trades: () => invoke<Trade[]>("trades"),
  addTrade: (t: Omit<Trade, "id" | "ts">) => invoke<number>("add_trade", { t }),
  recordTrade: (t: TradeDone) => invoke<number>("record_trade", { t }),
  deleteTrade: (id: number) => invoke<void>("delete_trade", { id }),
  datapoints: (limit = 500) => invoke<Datapoint[]>("datapoints", { limit }),
  addDatapoint: (platinum: number, ducats: number, credits: number) => invoke<void>("add_datapoint", { platinum, ducats, credits }),
  relicRuns: (limit = 200) => invoke<RelicRun[]>("relic_runs", { limit }),
  ocrNow: () => invoke<void>("ocr_now"),
  overlayToggle: () => invoke<boolean>("overlay_toggle"),
  on: <T>(event: string, cb: (payload: T) => void): Promise<UnlistenFn> => listen<T>(event, (e) => cb(e.payload)),
};

export const fmtTs = (ts: number) => new Date(ts * 1000).toLocaleString();

/** Open trade dialog (App.vue renders it). Set by a `trade_complete` game event
 *  or by "sold" on a listing, which preselects that order. */
export const tradePrompt = ref<{ order?: Order; onDone?: () => void } | null>(null);

/** Open "new listing" dialog (App.vue renders it). `slug` preselects an item. */
export const listingPrompt = ref<{ mode: "sell" | "buy"; slug?: string; onDone?: () => void } | null>(null);
