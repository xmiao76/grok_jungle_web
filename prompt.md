# Prompt: Jungle (Dou Shou Qi) — Web UI Edition

Act as a senior software architect and technical lead.

Create a step-by-step development plan for a **browser-based** Jungle board game application with a web UI and built-in AI, so a human can play against the computer on a visual board — running **entirely client-side** with the Python game engine executed in the browser via **Pyodide**, and deployed as a **static site on Cloudflare Pages (free tier)** so the game can be demoed on a public URL with zero hosting fees.

This is a web port of an existing, proven Windows desktop implementation located at `C:\GitHub\grok_jungle` (Python 3.12 + PySide6). The desktop project's `jungle.engine` and `jungle.ai` packages are pure Python (stdlib only, no PySide6/numpy dependency) and MUST be reused as-is or with minimal adaptation. Only the presentation layer (Qt GUI, input, main loop) is rewritten for the web.

Use the standard Jungle / Dou Shou Qi rules from this page as the main game specification and reference:
https://en.wikipedia.org/wiki/Jungle_(board_game)

If the board layout, terrain layout, or initial piece positions are unclear from the wiki page, also refer to:
https://veryspecial.us/free-downloads/AncientChess.com-DouShouQi.pdf

Do not restate the full rules in detail unless needed. The authoritative rule interpretation is the one already implemented and regression-tested in the desktop codebase (`jungle/engine/position.py`, `jungle/engine/movegen.py`, `jungle/engine/apply.py`, `jungle/engine/game.py`); the web version must behave identically.

## Architecture requirements

- **No server-side compute.** The deployed artifact is a static site: HTML, CSS, JavaScript, Python source files, and image/font assets. All game logic and AI search run in the visitor's browser.
- **Pyodide** loads the Python engine + AI. Pin a specific Pyodide version (0.27.7) and load it from the official CDN.
- **The AI search must run in a Web Worker**, not the main thread. The desktop app used a `QThread` (`_Thinker`) to keep the UI responsive; the web equivalent is a dedicated Worker hosting the Pyodide runtime, communicating with the main thread via `postMessage`. The UI thread must never freeze while the AI thinks.
- Recommended split (adjust with justification if a better structure exists):
  - Main thread: JS UI — board rendering (Canvas), input handling, animations, audio (Web Audio), menus, game-state display.
  - Worker: Pyodide + `jungle.engine` + `jungle.ai` — owns the authoritative `Game`; receives `{type: "apply_move" | "ai_move" | "new_game" | ...}` messages; returns legal moves, move results, win status, and AI moves.
- **Port, don't rewrite, the engine.** These files are reused verbatim:
  - `jungle/__init__.py`
  - `jungle/engine/types.py`, `board.py`, `position.py`, `apply.py`, `movegen.py`, `game.py`, `zobrist.py`, `__init__.py`
  - `jungle/ai/eval.py`, `search.py`, `__init__.py`
  - Do not copy `jungle/ui/` (PySide6). Do not fork rule logic into JavaScript — a single Python source of truth prevents rule drift.
- Desktop piece portraits are 512×512 (~400 KB each, ~6.5 MB total) plus a 1.2 MB body font. **Downscale portraits to 256×256 and compress** before shipping. Ship **Cinzel.ttf** only; do not ship `SourceSerif4.ttf`.

## Requirements

- Phase 1 must deliver a working web UI where a human can play against the AI in a modern desktop browser (Chrome, Edge, Firefox, Safari).
- The engine must remain responsive: Pyodide runs Python roughly 3–10× slower than CPython, so AI difficulty levels must be re-budgeted (time-limited iterative deepening already exists in `jungle.ai.search` — override per-difficulty `Limits` in the web JSON bridge only; do not edit `search.py`). `time.perf_counter` works under Pyodide.
- Three difficulty levels matching the desktop keys: **Easy / Normal / Strong** (`easy` / `normal` / `strong`) and human-vs-AI plus AI-vs-AI modes.
- Support choosing who moves first before a game starts. **The human always owns Gold.** Human first means Gold opens; AI first means Scarlet opens. This must not swap sides internally.
- Show a clear loading screen with progress while Pyodide and the engine initialize (first load is several seconds and a multi-MB download); the game must be interactive immediately after initialization completes.
- The site must work when served from any static host path (no absolute-path assumptions) and must be fully functional offline after first load is acceptable but not required.
- Automated tests must be created and maintained during development.
- Bugs found in testing or gameplay must be fixed and regression-tested until stable.
- The final application must complete full Jungle games correctly in the browser.

## Rule clarification for river jumping

- Implement the lion and tiger river-jump behavior exactly as in the desktop engine:
- The lion can jump **two-block** (horizontal, across 2 river squares: a↔d and d↔g) and **three-block** (vertical, across 3 river squares: rank 3↔7 on files b, c, e, f).
- The tiger jumps **two-block only** (horizontal). It cannot take the three-block vertical crossing.
- A rat on any water square along the jump path blocks the jump.
- Rat/water boundary rules, the Elephant-cannot-take-Rat exception (except a Rat on the Elephant's own trap), trap rank-zeroing, and all capture rules must match `jungle/engine/position.py` exactly. Since the engine is reused, this is guaranteed by construction — do not reimplement these rules in JS.

## UI requirements

- The final UI must be polished and attractive, not just functional — suitable for a public demo. Match the desktop lacquer / gold-leaf / cinnabar / jade aesthetic.
- The board should visually show river, trap, den, land, and other terrain clearly (reuse prepared tile art and/or the desktop's procedural paint).
- Each piece should look like its animal (reuse prepared gold/scarlet portraits), and each piece must also display the animal's short English name (e.g. "Rat", "Lion") and rank badge, readable on both sides' colors at any board size and following the piece during move animations. When the board is flipped, names stay upright.
- The main menu must display which AI model and which code agent were used to build the game (e.g. "Made with model Grok 4.6 (xAI) · code agent Grok Build"), alongside the engine version and runtime info.
- The main menu must include a "How to play" section with simple but complete guidance for a brand-new player: the two win conditions, how to select and move pieces with the UI's input model and move/capture indicators, the full rank order and the Rat-vs-Elephant exception (including a trapped Rat), river/swimming rules, Lion and Tiger river jumps (including rat-blocking), trap behavior, the own-den restriction, the stalemate loss, the threefold-repetition and 100-quiet-half-move draw rules, and the in-game controls (undo, board flip, resign, mute).
- Include good usability details: piece selection highlights, legal move indicators, capture feedback, move animations, turn display, capture trays, move history, and win/loss messaging.
- Support flipping the board upside down as a view option. This must only change the visual mapping of squares to pixels and mouse/touch coordinates back to squares. It must not change game state, must not swap sides internally, and must not change whose turn it is. Do not auto-flip when the AI opens — the desktop does not.
- Support a clear UI option to choose whether the human player or the AI moves first; it must work correctly together with the board-flip option. Human always owns Gold.
- Sound effects for move / capture / win with a mute toggle (browsers require a user gesture before audio — handle this gracefully). Synthesize via Web Audio; the desktop has no WAV files.
- Responsive layout: excellent on desktop; usable on tablet/mobile (touch input for the two-click select-then-move model) is a stretch goal for Phase 2, not Phase 1.
- Avoid placeholder-style visuals in the final release.

## Testing requirements

- The reused Python engine/AI tests from the desktop repo (pytest) must be brought over and kept passing under CPython in CI — they validate the single source of truth. Do **not** port `test_ui_smoke.py` (it requires PySide6).
- Add tests for the web JSON bridge (`web_api.py`).
- Add a smoke test that imports and exercises the engine under Pyodide itself (e.g. `pyodide` in Node) to catch Pyodide-specific breakage.
- Add JS unit tests for the UI layer's coordinate mapping (board↔pixel, including flipped mode) and worker message protocol.
- Add at least one end-to-end browser test (Playwright) that loads the deployed bundle, waits for Pyodide init, plays a scripted sequence of moves against the AI (Gold rat `g3`→`g4` is a legal opening), and asserts the game reaches a valid terminal state.

## Deployment requirements

- Deploy to **Cloudflare Pages free tier** as a static site (no Functions, no Workers paid features, no server).
- Derive the Pages project name (and therefore the `*.pages.dev` URL) from the meaningful part of the repository/project name, shortened and hyphenated: DNS subdomains cannot contain underscores, so the repo named `grok_jungle_web` deploys as project `grok-jungle` at `https://grok-jungle.pages.dev`. Document this naming derivation in the README.
- Provide the exact local build + deploy workflow: local dev server command, production build command (none — `public/` is the site), and deployment via either Cloudflare Pages git integration or `wrangler pages deploy`.
- Verify the deployed site after each deployment, not only the local build: load the public URL in a browser, complete at least one full game, and confirm assets/Pyodide load correctly (correct MIME types, CDN reachability).
- The repository must include a `README.md` with: live demo URL, how to run locally, how to deploy, gameplay/controls notes, architecture overview (Pyodide + Worker design), and a clear statement identifying which model and which code agent were used to complete the task.
- Save this prompt as `prompt.md` in the codebase.

## Please provide

- recommended tech stack and justification (plain JS vs. a framework, Canvas vs. DOM/SVG, bundler choice)
- architecture and module breakdown (main thread vs. worker, message protocol)
- engine porting plan (what is reused verbatim, what is shimmed, how the Python code is packaged for Pyodide)
- phased roadmap
- test plan for each phase
- automated testing strategy (pytest under CPython, Pyodide smoke test, JS unit tests, Playwright E2E)
- AI/engine performance plan for the browser (time budgets per difficulty, worker lifecycle, first-load experience)
- local build, run, and test workflow
- bug-fix and regression-test workflow
- Cloudflare Pages deployment plan and post-deploy validation plan
- expected repository contents
- suggested README.md contents
- completion criteria

Completion is only achieved when the game is playable and stable in the browser, completes full games correctly, passes required automated tests, is deployed and verified on a public Cloudflare Pages URL, includes README.md with the model/code-agent statement, and includes prompt.md in the codebase.
