# VOIDTRACE

```
 ▄▄ ▄▄  ▄▄▄  ▄▄ ▄▄▄▄   ▄▄▄▄▄ ▄▄▄▄   ▄▄▄   ▄▄▄▄ ▄▄▄▄▄
 ██ ██ ██ ██ ██ ██ ██    ██  ██ ██ ██ ██ ██    ██
 ██▄██ ██ ██ ██ ██ ██    ██  ████  █████ ██    ████
  ███   ▀█▀  ██ ████     ██  ██ ██ ██ ██  ▀███ ██▄▄▄
```

Warframe companion. Everything AlecaFrame does — without Overwolf, without ads, without a subscription.
Rust core, Tauri shell, Vue UI, terminal aesthetic.

> Status: working prototype, PC only. Built for my own account; use at your own risk (see [What it touches](#what-it-touches)).

## Features

| Tab | What it does | Data source |
|---|---|---|
| **Foundry** | Every masterable item with your XP, mastered / owned / missing | DE PublicExport + DE profile endpoint |
| **Inventory** | Your tradeable parts, mods, relics with live sell/buy/ducat values, one-click listing | Game process (opt-in) or manual |
| **Mastery** | MR progress, points to next rank, unmastered items closest first | DE profile endpoint |
| **Relic Planner** | Which relics drop item X, expected plat/ducats per refinement | PublicExport drop tables + warframe.market |
| **Riven Explorer** | Grade a riven against the roll range at the weapon's current disposition | PublicExport dispositions |
| **Stats / Trading** | Plat/ducat/credit history, trade log, relic run log | Inventory sync + manual |
| **Warframe.Market** | Login, your listings (edit/toggle/delete), price check with order book, online status + activity | warframe.market API v2 + WebSocket |
| **WFM Chat** | Your warframe.market conversations in the app | warframe.market API v1 (undocumented) |
| **Overlay** | Relic reward HUD: plat + ducats per card, best pick highlighted, click-through, always on top | EE.log trigger + Windows OCR |

Plus an ASCII intro sequence (Full / Quick / Off), hover cards with icons and prices on every item, and links straight to the item on warframe.market.

## How it works

```
EE.log ──tail──▶ events ──▶ "Got rewards" ──▶ screenshot ──▶ Windows OCR ──▶ prices ──▶ overlay
                   │
                   ├─▶ "Logged in" / EOM ──▶ memory scan ──▶ inventory ledger + datapoint
                   │
                   └─▶ mission start/end ──▶ warframe.market status (ON_MISSION / IN_ORBITER)
```

- **PublicExport** is fetched once per DE content hash and cached (`%LOCALAPPDATA%\VoidTrace\export.json`). Internal codenames (`PolearmPrimeBlueprint`) are resolved through `ExportRecipes → resultType`, so relic rewards show up as "Orthos Prime Blueprint" and match warframe.market.
- **Log patterns** are verified against a real `EE.log` and covered by a test; they can be overridden in Settings when DE renames something.
- **OCR** uses the Windows built-in engine, nothing to install. The reward row crop is configurable for non-16:9 screens.
- **warframe.market** login goes through the v1 flow (v2 sign-in is first-party only); the token is reused for v2 REST and the `wfm` WebSocket.

## What it touches

- `EE.log` — read only.
- Screen — one screenshot per relic reward screen.
- **Game process memory — opt-in, read only.** The inventory document the game receives at login is located by pattern (no offsets, no hooks, no injection, nothing written) with `ReadProcessMemory`. This is the same data Overwolf companions read, but outside Overwolf it is not covered by DE's third-party policy. It is off by default; enable it in *Settings → Inventory sync* if you accept that.
- Nothing is sent anywhere except api.warframe.market (your own account) and DE's public endpoints.

## Build

```
winget install Rustlang.Rustup
winget install Microsoft.VisualStudio.2022.BuildTools --override "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
npm install
npm run tauri dev        # or: npm run tauri build
```

Requirements: Windows 10/11, WebView2, Warframe in **Borderless Fullscreen**, game language English, same privilege level as the game (no "run as administrator" on either side).

Useful checks:

```
cargo test -p wfc-core                              # log patterns, memory scanner
cargo run -p wfc-core --example smoke               # export + market + relic plan + riven grade
cargo run --release -p wfc-core --example scan_watch  # poll the running game for the inventory document
```

## Layout

```
crates/wfc-core/   export · market · ws · log · profile · memory · inventory · ocr · relic · mastery · riven · db
src-tauri/         app state, commands, overlay window, event wiring
src/               Vue: intro, 9 tabs, overlay page, ItemLink hover card
```

## Settings worth knowing

- **Account id** (Mastery tab): DE's profile endpoint needs it since U38.0.8. Log in at warframe.com, open `www.warframe.com/api/user-data`, copy `user_id`. Don't spam it — DE rate-limits by IP.
- **Reward crop** (Settings): fractions of the screen; *Test OCR now* and *Preview overlay* exist to tune it.
- **Intro**: Full 8s / Quick 2s / Off, Esc skips.

## License

MIT
