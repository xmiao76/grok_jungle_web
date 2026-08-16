# jungle-engine

Rust Jungle (Dou Shou Qi) engine used by the web app.

```text
cargo test --release
cargo run --release -- search "l5t/1d3c1/r1p1w1e/7/7/7/E1W1P1R/1C3D1/T5L g" 750 20 0
npm run build:wasm   # from the repo root → public/wasm/
```

`stdio` protocol (used by pytest and `tools/strength_gate.py`):

```text
INFO
NEW human_ai 1 easy
APPLY g3 g4
AI 80
UNDO
SEARCH 90 12 1 1 <fen>[ | <histfen>...]
LEGAL <fen>
```
