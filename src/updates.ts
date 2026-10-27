import { reactive } from "vue";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { api } from "./api";

// Updates come from the latest GitHub release (latest.json, signed). Checked on
// start and every 6 hours unless turned off; never installed while the game runs.
export const updates = reactive({
  available: null as Update | null,
  checking: false,
  installing: false,
  progress: 0,
  error: "",
  checkedAt: 0,
  dismissed: false,
});

export async function checkForUpdate() {
  updates.checking = true;
  updates.error = "";
  try {
    updates.available = await check();
    updates.dismissed = false;
  } catch (e) {
    updates.error = String(e);
  } finally {
    updates.checking = false;
    updates.checkedAt = Date.now();
  }
  return updates.available;
}

export async function installUpdate() {
  const u = updates.available;
  if (!u) return;
  if (await api.gameRunning()) {
    updates.error = "Close Warframe first — the update restarts VoidTrace.";
    return;
  }
  updates.installing = true;
  updates.error = "";
  let total = 0, done = 0;
  try {
    await u.downloadAndInstall((ev) => {
      if (ev.event === "Started") total = ev.data.contentLength ?? 0;
      if (ev.event === "Progress") { done += ev.data.chunkLength; updates.progress = total ? done / total : 0; }
    });
    await relaunch();
  } catch (e) {
    updates.error = String(e);
    updates.installing = false;
  }
}

export async function startUpdateChecks() {
  if ((await api.getSetting("update_auto")) === "0") return;
  checkForUpdate();
  setInterval(checkForUpdate, 6 * 3600 * 1000);
}
