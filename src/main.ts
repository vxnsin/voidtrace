import { createApp } from "vue";
import App from "./App.vue";
import Overlay from "./overlay/Overlay.vue";
import "./style.css";

const isOverlay = location.hash.startsWith("#/overlay");
createApp(isOverlay ? Overlay : App).mount("#app");
