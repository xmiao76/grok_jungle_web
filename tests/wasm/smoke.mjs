// WASM smoke test: load the checked-in glue + wasm the same way the worker does.
// Usage: node tests/wasm/smoke.mjs

import { readFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..', '..');
const WASM_DIR = join(ROOT, 'public', 'wasm');
const GLUE = join(WASM_DIR, 'jungle_engine.js');
const WASM = join(WASM_DIR, 'jungle_engine_bg.wasm');

let failures = 0;
function check(cond, label) {
  console.log(`${cond ? 'ok' : 'FAIL'} - ${label}`);
  if (!cond) failures++;
}

const { existsSync } = await import('node:fs');
if (!existsSync(GLUE) || !existsSync(WASM)) {
  console.error('missing public/wasm artifacts; run npm run build:wasm');
  process.exit(1);
}

// wasm-bindgen web target uses import.meta.url to find the .wasm.
// Import the generated glue as a module from its real path.
const glueUrl = pathToFileURL(GLUE).href;
const mod = await import(glueUrl);
const t0 = performance.now();
const wasmBytes = await readFile(WASM);
mod.initSync({ module: wasmBytes });
console.log(`# wasm init in ${Math.round(performance.now() - t0)}ms`);

const info = JSON.parse(mod.engine_info());
check(info.ok && info.data.engineVersion, `engine_info: v${info.data.engineVersion} ${info.data.runtime}`);
check(String(info.data.runtime).includes('Rust'), 'runtime is Rust/wasm');

check(info.data.title === 'Grok-4.7 · GrokBuild', 'title omits official platform');
check(info.data.rulesId === 'desktop-jungle-v2', 'rules id');
check(info.data.wasmFilename === 'grok-4-7-grokbuild-2.0.0.wasm', 'release filename');

for (const difficulty of ['easy', 'medium', 'hard']) {
  const res = JSON.parse(mod.new_game('human_ai', true, difficulty));
  check(res.ok && res.data.state.legalMoves.length === 24, `new_game(${difficulty})`);
}

let env = JSON.parse(mod.new_game('human_ai', true, 'easy'));
check(env.data.state.turn === 'red', 'human first is red');
env = JSON.parse(mod.apply_move('g3', 'g4'));
check(env.ok && env.data.state.ply === 1, 'apply g3g4');
const t1 = performance.now();
env = JSON.parse(mod.ai_move(80));
const dt = performance.now() - t1;
check(env.ok && env.data.state.ply === 2, `ai reply in ${Math.round(dt)}ms`);
check(dt < 2000, 'ai 80ms budget finishes under 2s');
env = JSON.parse(mod.undo());
check(env.ok && env.data.state.ply === 0, 'undo pair');

env = JSON.parse(mod.new_game('human_ai', false, 'easy'));
check(env.data.state.turn === 'blue', 'ai first is blue');
env = JSON.parse(mod.ai_move(80));
check(env.ok && env.data.state.ply === 1, 'blue opening');
env = JSON.parse(mod.undo());
check(env.ok && env.data.state.turn === 'blue' && env.data.state.ply === 0, 'undo ai opening');

env = JSON.parse(mod.new_game('human_ai', true, 'hard'));
env = JSON.parse(mod.apply_move('g3', 'g4'));
const tHard = performance.now();
env = JSON.parse(mod.ai_move(0));
const hardDt = performance.now() - tHard;
check(env.ok && env.data.state.ply === 2, `hard reply in ${Math.round(hardDt)}ms`);
check(hardDt < 1000, 'hard move returns within 1s');

const illegal = JSON.parse(mod.apply_move('a1', 'd5'));
check(!illegal.ok, 'illegal rejected');

if (failures) {
  console.error(`# ${failures} failure(s)`);
  process.exit(1);
}
console.log('# wasm smoke passed');
