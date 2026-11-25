// Promise-based client for the engine. The worker is the normal path.
// If a module worker cannot start (some file:// browsers), the same API
// runs on the main thread and window.__jungle.thread records which one.

export class EngineClient {
  constructor(onProgress) {
    this._onProgress = onProgress || (() => {});
    this._nextId = 1;
    this._pending = new Map();
    this._thread = 'worker';
    this._worker = null;
    this._mod = null;
    this._useMain = false;
    this._startWorker();
  }

  get thread() {
    return this._thread;
  }

  _startWorker() {
    const url = globalThis.JUNGLE_WORKER_URL || 'js/engine-worker.js';
    try {
      this._worker = new Worker(url, { type: 'module' });
    } catch (err) {
      this._worker = null;
      this._workerError = err;
      return;
    }
    this._worker.onmessage = (ev) => this._onMessage(ev.data);
    this._worker.onerror = (ev) => {
      const message = ev.message || 'worker error';
      if (!this._useMain && this._pending.size > 0 && !this._mod) {
        this._failAll(message);
        try { this._worker.terminate(); } catch { /* already gone */ }
        this._worker = null;
      } else {
        this._failAll(message);
      }
    };
  }

  _onMessage(msg) {
    if (msg.type === 'progress') {
      this._onProgress(msg.stage);
      return;
    }
    const pending = this._pending.get(msg.id);
    if (!pending) return;
    this._pending.delete(msg.id);
    if (msg.ok) pending.resolve(msg.data);
    else pending.reject(new Error(msg.error || 'engine error'));
  }

  _failAll(message) {
    for (const { reject } of this._pending.values()) {
      reject(new Error(message));
    }
    this._pending.clear();
  }

  _call(type, payload = {}, timeoutMs = 120000) {
    if (!this._worker) {
      return Promise.reject(this._workerError || new Error('worker unavailable'));
    }
    const id = this._nextId++;
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        this._pending.delete(id);
        reject(new Error(`engine call '${type}' timed out`));
      }, timeoutMs);
      this._pending.set(id, {
        resolve: (v) => { clearTimeout(timer); resolve(v); },
        reject: (e) => { clearTimeout(timer); reject(e); },
      });
      this._worker.postMessage({ id, type, payload });
    });
  }

  async _startMain() {
    this._useMain = true;
    this._thread = 'main';
    globalThis.__JUNGLE_THREAD = 'main';
    const mod = globalThis.JUNGLE_ENGINE_MODULE
      || await import('../wasm/jungle_engine.js');
    this._onProgress('wasm');
    const start = mod.default || mod.init;
    if (typeof start !== 'function') {
      throw new Error('engine module has no init export');
    }
    if (globalThis.JUNGLE_ENGINE_BYTES) {
      await start({ module_or_path: globalThis.JUNGLE_ENGINE_BYTES });
    } else {
      await start();
    }
    this._onProgress('engine');
    const mb = (typeof navigator !== 'undefined' && navigator.deviceMemory && navigator.deviceMemory < 4)
      ? 4
      : 16;
    mod.init_engine(mb);
    this._mod = mod;
  }

  _callMain(type, payload = {}) {
    const mod = this._mod;
    let raw;
    switch (type) {
      case 'init':
        raw = mod.engine_info();
        break;
      case 'new_game':
        raw = mod.new_game(
          payload.mode || 'human_ai',
          payload.humanFirst !== false,
          payload.difficulty || 'medium',
        );
        break;
      case 'apply_move':
        raw = mod.apply_move(payload.src, payload.dst);
        break;
      case 'ai_move':
        raw = mod.ai_move(payload.budgetMs ?? 0);
        break;
      case 'undo':
        raw = mod.undo();
        break;
      case 'resign':
        raw = mod.resign();
        break;
      case 'get_state':
        raw = mod.get_state();
        break;
      case 'replay':
        raw = mod.replay_moves(JSON.stringify(payload.moves));
        break;
      default:
        throw new Error(`unknown message type: ${type}`);
    }
    const envelope = JSON.parse(raw);
    if (!envelope.ok) throw new Error(envelope.error || 'engine error');
    return envelope.data;
  }

  async _rpc(type, payload = {}, timeoutMs = 120000) {
    if (this._useMain) return this._callMain(type, payload);
    return this._call(type, payload, timeoutMs);
  }

  async init() {
    if (!this._useMain && this._worker) {
      try {
        const info = await this._call('init', {}, 180000);
        this._thread = 'worker';
        globalThis.__JUNGLE_THREAD = 'worker';
        return info;
      } catch (err) {
        if (this._worker) {
          try { this._worker.terminate(); } catch { /* ignore */ }
          this._worker = null;
        }
        await this._startMain();
        return this._callMain('init');
      }
    }
    await this._startMain();
    return this._callMain('init');
  }

  newGame(opts = {}) {
    return this._rpc('new_game', {
      mode: opts.mode || 'human_ai',
      humanFirst: opts.humanFirst !== false,
      difficulty: opts.difficulty || 'medium',
    });
  }
  applyMove(src, dst) { return this._rpc('apply_move', { src, dst }); }
  aiMove(budgetMs) { return this._rpc('ai_move', { budgetMs }, 180000); }
  undo() { return this._rpc('undo'); }
  resign() { return this._rpc('resign'); }
  getState() { return this._rpc('get_state'); }
  replay(moves) { return this._rpc('replay', { moves }); }
}
