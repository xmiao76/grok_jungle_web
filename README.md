# Jungle (Dou Shou Qi) — Web Edition

Play the classic **Jungle / 斗兽棋** board game against an AI, entirely in
your browser. The `grok_jungle` Python engine and searcher run **client-side
via [Pyodide](https://pyodide.org)** (CPython compiled to WebAssembly) inside a
Web Worker — no server, no backend, zero hosting cost on the
**Cloudflare Pages free tier**.

This is the web port of the Windows desktop app in `grok_jungle`
(Python 3.12 + PySide6). The `jungle.engine` and `jungle.ai` packages are reused
**verbatim** — a single Python source of truth for the rules and the AI — and
only the presentation layer was rewritten for the web (HTML/Canvas/JS).

## Live demo

**https://grok-jungle.pages.dev** — deployed on the Cloudflare Pages free tier.
(The project name derives from the repo name `grok_jungle_web`; DNS subdomains
cannot contain underscores, so `grok_jungle` becomes `grok-jungle`.)

## Features

- Human vs AI and AI-vs-AI (watch) modes
- Three difficulties: Easy / Normal / Strong — the desktop iterative-deepening
  searcher, upgraded for the web with a richer eval, static exchange pruning,
  and a small opening book. Easy stays noisy and shallow so humans can win.
- You always own Gold. Choose who opens: human first (Gold) or AI first (Scarlet)
- Two-click move input with legal-move indicators, capture rings, move
  animation, capture flash, capture trays, move history, undo, resign,
  board flip (view only), synthesized sound with mute
- Loading screen while the Python runtime initializes (~10 MB first load,
  cached afterwards)

## Architecture

```
Main thread (JS)                     Web Worker
┌─────────────────────────┐  msgs   ┌────────────────────────────┐
│ main.js    UI state     │ ──────► │ engine-worker.js           │
│ board-renderer.js Canvas│ ◄────── │  Pyodide (Python 3.12 wasm)│
│ coords.js  mapping      │  JSON   │   web_api.py  JSON bridge  │
│ audio.js   Web Audio    │         │    jungle.engine  rules    │
│ worker-client.js        │         │    jungle.ai     search    │
└─────────────────────────┘         └────────────────────────────┘
```

- The worker owns the authoritative `Game`; the UI never re-implements rules
  in JS (no rule drift).
- The AI searches inside the worker, so the page stays responsive — the web
  equivalent of the desktop app's `QThread`.
- Board flip is purely visual: the engine always sees Gold at the south
  (rank 1), exactly like the desktop renderer.

## Run locally

Requires Node ≥ 20 and Python ≥ 3.12 (Python only for tests and asset prep).

```text
python -m venv .venv
.venv\Scripts\activate
pip install pytest pillow
npm install
node tools/sync-engine.mjs
python tools/prep-assets.py
npm run dev          # serves public/ at http://localhost:8788
```

## Tests

```text
npm run test:py        # pytest: desktop engine/AI suite + web_api bridge
npm run test:js        # JS unit tests (coordinate mapping, including flip)
npm run test:pyodide   # engine smoke test under a real Pyodide runtime (Node)
npm run test:e2e       # Playwright: browser load, clicks, AI replies, full game
                       # (first time: npx playwright install chromium)
```

## Deploy to Cloudflare Pages (free)

Option A — Git integration (recommended):
1. Push this repo to GitHub.
2. Cloudflare dashboard → Workers & Pages → Create → Pages → connect the repo.
3. Build command: *(leave empty)* · Build output directory: `public`.
4. Every push to `main` deploys automatically.

Option B — Direct upload from the CLI:
```text
npx wrangler login
npm run deploy       # wrangler pages deploy public --project-name grok-jungle
```

Post-deploy check: open the URL, wait for the loading screen to finish, play a
few moves on Easy, and confirm no errors in the browser console.

## Gameplay

- 7×9 board. Gold sits south (den on `d1`); Scarlet sits north (den on `d9`).
- Animals move one square orthogonally. Stronger or equal rank captures, with
  the Rat/Elephant exception.
- Rank: Elephant 8 > Lion 7 > Tiger 6 > Leopard 5 > Wolf 4 > Dog 3 > Cat 2 > Rat 1.
- Only the Rat swims. A Rat in water can be captured only by another Rat in water.
- The Rat may capture the Elephant from land (not from water). The Elephant
  **cannot** capture a Rat, except a Rat sitting on the Elephant’s own trap.
- Enemy traps drop a piece’s rank to 0. Your own traps do nothing.
- You may not enter your own den. Entering the enemy den wins. Capturing every
  enemy piece also wins.
- **Lion** may jump two river blocks (a↔d, d↔g) and three blocks (rank 3↔7).
- **Tiger** may jump two blocks only. A Rat of either color on the jump path
  blocks the leap.
- Draw by threefold repetition or 100 half-moves without a capture. A side
  with no legal move loses.

## Controls

| Action | How |
|---|---|
| Move | Click your piece, then a highlighted square |
| New Game | Menu → Play vs AI / Watch AI vs AI |
| Flip board | Button or `F` — **view only**; state and turn do not change |
| Undo | Button or `Ctrl+Z` — undoes your last turn (and the AI reply) |
| Resign | Button |
| Mute | Button — synthesized move/capture/win cues |
| Human first | Gold opens; you own Gold |
| AI first | Scarlet opens; you still own Gold |

## Project layout

```
public/            deployed static site
  index.html       UI shell
  css/, js/        presentation layer (Canvas renderer, worker client)
  js/engine-worker.js  Pyodide host (pinned v0.27.7 from jsDelivr CDN)
  py/              Python sources loaded into Pyodide
    jungle/        engine (verbatim from grok_jungle) + web-strengthened AI
    web_api.py     JSON bridge for the worker (web-only)
  assets/          compressed portraits, terrain, Cinzel
tests/             pytest, node --test, Pyodide smoke, Playwright
tools/             sync-engine, prep-assets, static server
```

## Credits

- Game: traditional Jungle / Dou Shou Qi
  ([rules](https://en.wikipedia.org/wiki/Jungle_(board_game)))
- Desktop engine: `C:\GitHub\grok_jungle`
- **This application was implemented by Grok 4.6 (xAI) using the Grok Build
  coding agent.**
