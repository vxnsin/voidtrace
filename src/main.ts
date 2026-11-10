import { createApp } from "vue";
import App from "./App.vue";
import Overlay from "./overlay/Overlay.vue";
import { api } from "./api";
import { initTheme } from "./theme";
import "./style.css";

const isOverlay = location.hash.startsWith("#/overlay");
const app = createApp(isOverlay ? Overlay : App);

// UI errors go into the same log file as the backend, so bug reports have them.
const where = isOverlay ? "overlay" : "main";
let last = "";
function report(msg: string) {
  if (msg === last) return; // a render loop shouldn't flood the log
  last = msg;
  api.logFrontend("error", `[${where}] ${msg}`).catch(() => {});
}
app.config.errorHandler = (err, _vm, info) => {
  report(`${info}: ${err instanceof Error ? err.stack ?? err.message : String(err)}`);
  console.error(err);
};
window.addEventListener("error", (e) => report(`${e.message} at ${e.filename}:${e.lineno}`));
window.addEventListener("unhandledrejection", (e) => report(`unhandled: ${e.reason instanceof Error ? e.reason.stack ?? e.reason.message : String(e.reason)}`));

// Theme first, so the intro already draws in the right colours.
initTheme().finally(() => app.mount("#app"));
