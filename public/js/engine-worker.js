// Engine Web Worker: hosts the Rust/wasm Jungle engine so the UI thread
// never blocks while the AI thinks. Module worker (imports the wasm glue).

import init, {
  init_engine,
  engine_info,
  new_game,
  apply_move,
  ai_move,
  undo,
  resign,
  get_state,
  replay_moves,
} from '../wasm/jungle_engine.js';

let ready = null;

function post(msg) { self.postMessage(msg); }
function progress(stage) { post({ type: 'progress', stage }); }

async function ensure() {
  if (ready) return ready;
  ready = (async () => {
    progress('wasm');
    await init();
    progress('engine');
    const mb = (typeof navigator !== 'undefined' && navigator.deviceMemory && navigator.deviceMemory < 4)
      ? 4
      : 16;
    init_engine(mb);
    return JSON.parse(engine_info());
  })();
  return ready;
}

function callApi(type, payload) {
  switch (type) {
    case 'new_game':
      return new_game(
        payload.mode || 'human_ai',
        payload.humanFirst !== false,
        payload.difficulty || 'normal',
      );
    case 'apply_move':
      return apply_move(payload.src, payload.dst);
    case 'ai_move':
      return ai_move(payload.budgetMs ?? 0);
    case 'undo':
      return undo();
    case 'resign':
      return resign();
    case 'get_state':
      return get_state();
    case 'replay':
      return replay_moves(JSON.stringify(payload.moves));
    default:
      throw new Error(`unknown message type: ${type}`);
  }
}

self.onmessage = async (ev) => {
  const { id, type, payload } = ev.data;
  try {
    if (type === 'init') {
      const info = await ensure();
      post({ id, ok: true, data: info.data });
      return;
    }
    await ensure();
    const envelope = JSON.parse(callApi(type, payload || {}));
    if (envelope.ok) post({ id, ok: true, data: envelope.data });
    else post({ id, ok: false, error: envelope.error });
  } catch (err) {
    post({ id, ok: false, error: String((err && err.message) || err) });
  }
};
