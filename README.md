# Jungle (Dou Shou Qi) — Web Edition

Play the classic **Jungle / 斗兽棋** board game against an AI, entirely in
your browser. The engine is a **Rust bitboard searcher compiled to
WebAssembly**, running inside a Web Worker — no server, no backend, zero
hosting cost on the **Cloudflare Pages free tier**.

This is the web port of the Windows desktop app in `grok_jungle`. The
presentation layer is HTML/Canvas/JS. The playing AI is the native
`jungle-engine` crate (make/unmake, path-aware den races, 2-piece
tablebase, opening book). A Python rule oracle stays in-tree for tests only
and is **not** shipped to the browser.

## Live demo

**https://grok-jungle.pages.dev** — deployed on the Cloudflare Pages free tier.
(The project name derives from the repo name `grok_jungle_web`; DNS subdomains
cannot contain underscores, so `grok_jungle` becomes `grok-jungle`.)

## Features

- Human vs AI and AI-vs-AI (watch) modes
- Three difficulties: Easy / Normal / Strong. Easy is noisy and depth-2 so
  humans can win. Normal and Strong use the full native search, book, and
  1-vs-1 endgame table.
- You always own Gold. Choose who opens: human first (Gold) or AI first (Scarlet)
- Two-click move input with legal-move indicators, capture rings, move
  animation, capture flash, capture trays, move history, undo, resign,
  board flip (view only), synthesized sound with mute
- Loading screen while the wasm engine initializes (a few hundred KB, cached
  afterwards)

## Architecture

```
Main thread (JS)                     Web Worker
┌─────────────────────────┐  msgs   ┌────────────────────────────┐
│ main.js    UI state     │ ──────► │ engine-worker.js           │
│ board-renderer.js Canvas│ ◄────── │  jungle_engine.wasm        │
│ coords.js  mapping      │  JSON   │   rules + Game session     │
│ audio.js   Web Audio    │         │   alphabeta + TT + TB      │
│ worker-client.js        │         │                            │
└─────────────────────────┘         └────────────────────────────┘
```

- The worker owns the authoritative game; the UI never re-implements rules
  in JS (no rule drift).
- The AI searches inside the worker, so the page stays responsive.
- Board flip is purely visual: the engine always sees Gold at the south
  (rank 1), exactly like the desktop renderer.

## Run locally

Requires Node ≥ 20, Python ≥ 3.12 (tests and asset prep), and a Rust toolchain
only if you rebuild the engine (`wasm-pack`, `wasm32-unknown-unknown`).

```text
python -m venv .venv
.venv\Scripts\activate
pip install pytest pillow
npm install
python tools/prep-assets.py
npm run dev          # serves public/ at http://localhost:8788
```

Prebuilt wasm lives in `public/wasm/`. After changing `engine/`:

```text
npm run build:wasm
```

## Tests

```text
npm run test:engine    # cargo test --release (perft, session, tactics)
npm run test:py        # pytest: Python rule oracle + native CLI session
npm run test:js        # JS unit tests (coordinate mapping, including flip)
npm run test:wasm      # load public/wasm in Node and exercise the session
npm run test:e2e       # Playwright: browser load, clicks, AI replies, full game
                       # (first time: npx playwright install chromium)
python tools/strength_gate.py   # native vs frozen Python searcher, ≥ 9/12
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
- A swimming Rat cannot capture any piece on land (including a land Rat).
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
engine/            Rust crate (rules, search, session, wasm bindings)
public/            deployed static site
  index.html       UI shell
  css/, js/        presentation layer (Canvas renderer, worker client)
  js/engine-worker.js  loads public/wasm/jungle_engine.js
  wasm/            checked-in wasm-bindgen output
  py/              Python rule oracle (tests only; not loaded by the worker)
  assets/          compressed portraits, terrain, Cinzel
tests/             pytest, node --test, wasm smoke, Playwright
tools/             build-wasm, prep-assets, static server, strength_gate
```

## Credits

- Game: traditional Jungle / Dou Shou Qi
  ([rules](https://en.wikipedia.org/wiki/Jungle_(board_game)))
- Desktop native engine: `C:\GitHub\grok_jungle`
- **This application was implemented by Grok 4.6 (xAI) using the Grok Build
  coding agent.**
