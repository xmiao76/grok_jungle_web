// Pyodide smoke test: load the served Python sources into a real Pyodide
// runtime and exercise the web bridge (same WASM engine as the browser).
//
// Usage: node tests/pyodide/smoke.mjs

import { loadPyodide } from 'pyodide';
import { readFile } from 'node:fs/promises';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const PY_ROOT = join(fileURLToPath(new URL('../..', import.meta.url)), 'public', 'py');
const PY_FILES = [
  'web_api.py',
  'jungle/__init__.py',
  'jungle/engine/__init__.py',
  'jungle/engine/types.py',
  'jungle/engine/board.py',
  'jungle/engine/position.py',
  'jungle/engine/apply.py',
  'jungle/engine/movegen.py',
  'jungle/engine/game.py',
  'jungle/engine/zobrist.py',
  'jungle/ai/__init__.py',
  'jungle/ai/eval.py',
  'jungle/ai/search.py',
  'jungle/ai/see.py',
  'jungle/ai/opening_book.py',
];

let failures = 0;
function check(cond, label) {
  console.log(`${cond ? 'ok' : 'FAIL'} - ${label}`);
  if (!cond) failures++;
}

const t0 = performance.now();
const pyodide = await loadPyodide();
console.log(`# pyodide loaded in ${Math.round(performance.now() - t0)}ms`);

pyodide.FS.mkdirTree('/game/jungle/engine');
pyodide.FS.mkdirTree('/game/jungle/ai');
for (const path of PY_FILES) {
  pyodide.FS.writeFile('/game/' + path, await readFile(join(PY_ROOT, path), 'utf8'));
}
pyodide.runPython("import sys; sys.path.insert(0, '/game')");

const api = pyodide.pyimport('web_api');
const call = (fn, ...args) => JSON.parse(api[fn](...args));

const info = call('engine_info');
check(info.ok && info.data.engineVersion === '1.0.0',
  `engine_info: v${info.data.engineVersion}, Python ${info.data.python}`);

for (const difficulty of ['easy', 'normal', 'strong']) {
  const res = call('new_game', 'human_ai', true, difficulty);
  check(res.ok && res.data.state.legalMoves.length === 24, `new_game(${difficulty})`);
  const times = [];
  let state = res.data.state;
  for (let ply = 0; ply < 4 && !state.terminal; ply++) {
    const t = performance.now();
    const mv = call('ai_move', difficulty === 'easy' ? 120 : 400);
    times.push(Math.round(performance.now() - t));
    check(mv.ok, `ai_move ${difficulty} ply ${ply}`);
    state = mv.data.state;
  }
  console.log(`# difficulty ${difficulty} think times (ms): ${times.join(', ')}`);
}

{
  const res = call('new_game', 'human_ai', false, 'easy');
  check(res.ok && res.data.state.turn === 'scarlet', 'AI first: Scarlet to move');
  check(res.data.state.humanSide === 'gold', 'AI first: human still Gold');
}

{
  const res = call('new_game', 'human_ai', true, 'easy');
  check(res.ok, 'new_game easy for long game');
  let state = res.data.state;
  let plies = 0;
  while (!state.terminal && plies < 80) {
    const mv = call('ai_move', 200);
    if (!mv.ok) {
      check(false, `long game ai_move failed at ply ${plies}: ${mv.error}`);
      break;
    }
    state = mv.data.state;
    plies++;
  }
  check(plies > 10, `long game played ${plies} plies (terminal=${state.terminal}` +
    (state.result ? `, result=${state.result}/${state.outcome})` : ')'));
}

{
  call('new_game', 'human_ai', true, 'easy');
  check(!call('apply_move', 'a1', 'd5').ok, 'illegal move rejected');
  check(call('apply_move', 'g3', 'g4').ok, 'legal human move applied');
  check(call('ai_move', 120).ok, 'ai reply');
  const undo = call('undo');
  check(undo.ok && undo.data.state.ply === 0, 'undo restores start');
}

console.log(failures ? `\n${failures} FAILURE(S)` : '\nall smoke checks passed');
process.exit(failures ? 1 : 0);
