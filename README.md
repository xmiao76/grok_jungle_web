# Jungle (Dou Shou Qi) — Web Edition

Play Jungle in the browser, and download a Jungle Arena entrant built from the same rules and search.

**Grok-4.7 · GrokBuild** · engine 2.0.0 · runtime Rust · platform official

Source: https://github.com/xmiao76/grok_jungle_web

Live site: **https://grok-jungle.pages.dev** (Cloudflare Pages project `grok-jungle`).

## Play

Open `release/index.html` directly (`file://`). No server and no network. The page embeds the shell engine and the Arena component.

Or serve the multi-file site:

```text
npm install
npm run dev          # http://localhost:8799
```

- Human vs AI and AI vs AI
- Easy, Medium, and Hard. Hard searches for at most 850 ms so the reply, including worker messaging, is on screen within one second. It returns sooner for a book hit, a single legal move, or a proven win or loss.
- You always play Red, on the bottom. The first-mover switch chooses whether Red or Blue opens. Flip is view-only.
- Undo in Human vs AI takes back your move, or your move and the AI reply when the AI has already moved. Undo during the AI's think drops that reply. AI vs AI undoes one ply.
- Download Arena `.wasm` from the menu. The saved file is `grok-4-7-grokbuild-2.0.0.wasm`.

## Arena entrant

`release/grok-4-7-grokbuild-2.0.0.wasm` is a WebAssembly component for world `jungle:arena/competitor@1.0.0`. It imports only `jungle:arena/display@1.0.0` and exports only `jungle:arena/game@1.0.0`. There is no WASI, no clock, and no threads. Search is bounded by the host node budget.

| | |
|---|---|
| Rules | `desktop-jungle-v2` |
| Rules SHA-256 | `a457bbc009e975a9eafe5ff1f8fa40ca2548f2a689b121c8c4ed99dd0f3fdc57` |
| Component SHA-256 | `37e34cc57a11a304874414800d34fef260ef8362384450a372ac6e991a15893a` |
| `release/index.html` SHA-256 | `e96a4a86dd6bbcca0142890255ea240bbb0c82dbfcc30a629dbe50bc2364ac9e` |

`release/SHA256.txt` records the same digests. Easy arena play uses at most 200 nodes, depth 2, noise, and no book or tablebase. Medium uses one third of the budget. Hard uses the full budget, with book and tablebase. A zero budget returns legal index 0.

Upload at https://jungle-leaderboard.pages.dev/arena/upload/. Wait until every admission stage passes, then choose **Play in left seat** or **Play in right seat**.

On 2026-11-24, fetching `https://jungle-leaderboard.pages.dev/arena/upload/` returned the public leaderboard page rather than an upload form, so no admission run was completed from this environment. Milestones 1–5 are the acceptance path. `node tools/conformance.mjs` transpiles the release bytes with `@bytecodealliance/jco-transpile` 0.24.0, checks the preamble, WIT, metadata, opening, both orientations, determinism, isolation, shell/component parity, and two colors-swapped games against `public/wasm/fixture.wasm`. The browser host is `public/conformance.html`.

## Strength

Two games, colors swapped, against the previous 1.2.0 search (node budget stops that build between depths, so it may finish the iteration that crosses the budget). Score is from 2.0.0. Win 1, draw 0.5.

```text
5000 nodes:  new 1.5/2 (draw as Red, win as Blue); Wilson 95% CI [0.20, 0.97]
20000 nodes: new 0.5/2 (draw as Red, loss as Blue); Wilson 95% CI [0.03, 0.80]
```

The four-game total is 2.0/4. The intervals are wide because each budget is only two games. The 20k loss is consistent with the v2 repetition rule (a position counts on the third occurrence, not the second) and with the older search receiving a deeper completed iteration. Eval, book, and tablebase were left as they were.

## Build

Requires Rust (`wasm32-unknown-unknown`), `wasm-pack`, `wasm-tools`, Node 20+, and Python 3.12 for the oracle.

```text
npm run build:wasm         # shell engine → public/wasm
npm run build:component    # release component + fixture
npm run build:release      # release/index.html
```

`build:component` embeds `wit/jungle.wit` and wraps the core module with `wasm-tools component new`. No WASI adapter.

## Tests

```text
npm run test:engine        # cargo test
npm run test:py            # pytest, including the native session CLI
npm run test:js            # coordinate mapping, including flip
npm run test:wasm          # shell wasm in Node, including a Hard reply under 1s
npm run test:conformance   # admission checklist and both fixture seats
npm run test:e2e           # Playwright Chromium: hosted UI in a worker, file:// on the main thread
python tools/strength_report.py 5000 20000
```

## Architecture

The shell and the Arena component share `engine/` (rules, move generation, search, book, tablebase, software renderer). The shell is a wasm-bindgen module in a Web Worker, with a main-thread fallback when a module worker cannot start. The component is a separate build: `arena` feature, no `clock` feature. `engine/src/identity.rs` is the only source of the title, filename, and `info()` strings.

Wire squares are top-left (`row * 7 + col`). The engine keeps rank-from-bottom. Red is the side that used to be called Gold internally; Blue is the side that used to be called Scarlet. Piece art stays with those colors: Red uses the gold portraits, Blue the scarlet portraits.

Terminal order is den, elimination, immobilization, repetition (third occurrence of the board plus the side to move), then 100 quiet plies (`no-capture`).

## Rules in brief

Red sits on the bottom. Pieces step one square orthogonally. Equal or lower rank captures. A rat takes an elephant; an elephant does not take an untrapped rat. Only a rat swims, and a capture cannot cross the bank. A lion on land jumps two or three water squares; a tiger jumps two. A rat in the water blocks the jump. An enemy trap sets rank to 0. You cannot enter your own den. Entering the enemy den wins, as does capturing every enemy piece. No legal move loses. The third occurrence of a position, and 100 quiet moves, draw.

## Controls

| Action | How |
|---|---|
| Move | Click your piece, then a jade dot or a capture ring |
| New game | Play vs AI, or Watch AI vs AI |
| Flip | Button or `F` |
| Undo | Button or `Ctrl+Z` |
| Resign, mute, menu | Buttons in the side panel |

## Layout

```text
engine/            shared Rust rules, search, paint, component guest
fixture/           legal-index-0 opponent
wit/jungle.wit     Arena contract
rules/             desktop-jungle-v2.json
public/            Cloudflare Pages site, including conformance.html
release/           index.html, the named .wasm, SHA256.txt, strength.txt
tools/             wasm, component, release, and conformance builds
```

## Deploy

```text
npx wrangler login
npm run deploy       # wrangler pages deploy public --project-name grok-jungle
```

Build command on Pages is empty. Output directory is `public`. WASM is served as `application/wasm` via `public/_headers`.

The production site was deployed on 2026-11-24 and serves this build, including the component hash above. Playwright covered Chromium: the hosted game runs the engine in a worker, and `file://` falls back to the main thread. Edge, Firefox, and Safari were not run in this environment.

This application was implemented by Grok-4.7 using the Grok Build coding agent.
