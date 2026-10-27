import { reactive } from "vue";
import { api, type GameEvent, type RewardHit, type WorldState } from "../api";

// Shared live data for the widgets, in whichever window shows them (overlay
// layer, Widgets tab). Started once per window.
export const live = reactive({
  now: Date.now(),
  world: null as WorldState | null,
  rewards: [] as RewardHit[],
  showRewards: false,
  game: { running: false, focused: false, where: "orbiter" as "orbiter" | "mission" },
  session: { start: Date.now() / 1000, plat: null as number | null, ducats: null as number | null, relics: 0, trades: 0 },
});

let started = false;

async function refreshSession() {
  const since = live.session.start;
  const [runs, trades, points] = await Promise.all([
    api.relicRuns(500).catch(() => []), api.trades().catch(() => []), api.datapoints(500).catch(() => []),
  ]);
  live.session.relics = runs.filter((r) => r.ts >= since).length;
  live.session.trades = trades.filter((t) => t.ts >= since).length;
  // Plat / ducat change needs two inventory syncs: the last one before (or at)
  // the start and the newest one.
  const sorted = [...points].sort((a, b) => a.ts - b.ts);
  const base = [...sorted].reverse().find((p) => p.ts <= since + 120) ?? sorted.find((p) => p.ts >= since);
  const last = sorted[sorted.length - 1];
  const has = base && last && last.ts > base.ts;
  live.session.plat = has ? last.platinum - base.platinum : null;
  live.session.ducats = has ? last.ducats - base.ducats : null;
}

function onGame(e: GameEvent) {
  switch (e.kind) {
    case "game_started":
      live.game.running = true;
      live.session.start = Date.now() / 1000;
      refreshSession();
      break;
    case "game_closed":
      live.game.running = false;
      live.game.focused = false;
      break;
    case "game_focus":
      live.game.focused = !!e.focused;
      break;
    case "mission_start":
      live.game.where = "mission";
      break;
    case "mission_end":
    case "logged_in":
      live.game.where = "orbiter";
      refreshSession();
      break;
  }
}

export function startLive() {
  if (started) return;
  started = true;
  setInterval(() => (live.now = Date.now()), 1000);
  api.worldstate().then((w) => w && (live.world = w)).catch(() => {});
  api.gameRunning().then((r) => (live.game.running = r)).catch(() => {});
  api.gameFocused().then((f) => (live.game.focused = f)).catch(() => {});
  api.on<WorldState>("worldstate", (w) => (live.world = w));
  api.on<RewardHit[]>("rewards", (h) => { live.rewards = h; live.showRewards = true; });
  api.on("rewards-done", () => (live.showRewards = false));
  api.on<GameEvent>("game-event", onGame);
  refreshSession();
  setInterval(refreshSession, 30000);
}

/** "1h 04m" / "12m 05s" until an ISO time. */
export function left(iso: string | undefined, now = live.now) {
  if (!iso) return "–";
  let s = Math.max(0, Math.floor((Date.parse(iso) - now) / 1000));
  const d = Math.floor(s / 86400); s %= 86400;
  const h = Math.floor(s / 3600), m = Math.floor((s % 3600) / 60), x = s % 60;
  if (d) return `${d}d ${h}h`;
  return h ? `${h}h ${String(m).padStart(2, "0")}m` : `${m}m ${String(x).padStart(2, "0")}s`;
}

/** 0..1 through the current phase. */
export function progress(a: string | undefined, b: string | undefined, now = live.now) {
  if (!a || !b) return 0;
  const s = Date.parse(a), e = Date.parse(b);
  return Math.min(1, Math.max(0, (now - s) / Math.max(1, e - s)));
}
