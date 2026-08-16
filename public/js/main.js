// UI controller: screens, worker, canvas, input (web stand-in for MainWindow).

import { BoardRenderer, loadAssets } from './board-renderer.js';
import { EngineClient } from './worker-client.js';
import { AudioManager } from './audio.js';

const AVA_MOVE_DELAY_MS = 380;

const LOADING_STAGES = {
  wasm: 'Loading native engine…',
  engine: 'Initializing AI…',
};

const $ = (id) => document.getElementById(id);

class App {
  constructor() {
    this.client = null;
    this.renderer = null;
    this.audio = new AudioManager();
    this.screen = 'loading';
    this.mode = 'human_ai';
    this.difficulty = 'normal';
    this.humanFirst = true;
    this.state = null;
    this.aiThinking = false;
    this.busy = false;
    this.avaPaused = false;
    this.gen = 0;
    this.selected = null;
  }

  async boot() {
    try {
      this.client = new EngineClient((stage) => {
        $('loading-stage').textContent = LOADING_STAGES[stage] || stage;
      });
      const [info, assets] = await Promise.all([this.client.init(), loadAssets()]);
      this.renderer = new BoardRenderer($('board'), assets);
      $('engine-info').textContent =
        `engine v${info.engineVersion} · ${info.runtime || 'Rust/wasm'} in your browser`;
      this._bindUi();
      this._resize();
      window.addEventListener('resize', () => this._resize());
      this._showScreen('menu');
      requestAnimationFrame(() => this._frame());
      window.__jungle.ready = true;
    } catch (err) {
      this._fatal(`Failed to load the game engine: ${err.message}`);
    }
  }

  _bindUi() {
    for (const btn of document.querySelectorAll('.diff-btn')) {
      btn.addEventListener('click', () => {
        this.difficulty = btn.dataset.d;
        this._syncMenu();
      });
    }
    $('first-toggle').addEventListener('change', (ev) => {
      this.humanFirst = ev.target.checked;
    });
    $('btn-start-hva').addEventListener('click', () => this.startGame('human_ai'));
    $('btn-start-ava').addEventListener('click', () => this.startGame('ai_ai'));
    $('btn-undo').addEventListener('click', () => this.undo());
    $('btn-flip').addEventListener('click', () => this.toggleFlip());
    $('btn-resign').addEventListener('click', () => this.resign());
    $('btn-mute').addEventListener('click', () => {
      const muted = this.audio.toggleMute();
      $('btn-mute').textContent = muted ? 'Unmute' : 'Mute';
    });
    $('btn-menu').addEventListener('click', () => this.toMenu());
    $('btn-pause').addEventListener('click', () => this.togglePause());
    $('btn-again').addEventListener('click', () => this.startGame(this.mode));
    $('btn-tomenu').addEventListener('click', () => this.toMenu());
    $('btn-reload').addEventListener('click', () => location.reload());
    $('board').addEventListener('click', (ev) => this._onBoardClick(ev));
    window.addEventListener('keydown', (ev) => this._onKey(ev));
    this._syncMenu();
  }

  _syncMenu() {
    for (const btn of document.querySelectorAll('.diff-btn')) {
      btn.classList.toggle('active', btn.dataset.d === this.difficulty);
    }
    $('first-toggle').checked = this.humanFirst;
  }

  _resize() {
    const panel = 300;
    const availW = Math.max(window.innerWidth - panel - 56, 280);
    const availH = Math.max(window.innerHeight - 40, 360);
    const frameFrac = 0.40;
    const cell = Math.max(36, Math.min(88, Math.floor(availW / (7 + 2 * frameFrac)),
      Math.floor(availH / (9 + 2 * frameFrac))));
    if (this.renderer) this.renderer.resize(cell);
  }

  _frame() {
    if (this.renderer && this.screen !== 'loading' && this.screen !== 'menu') {
      this.renderer.draw();
    }
    requestAnimationFrame(() => this._frame());
  }

  _showScreen(name) {
    this.screen = name;
    $('loading-overlay').hidden = name !== 'loading';
    $('menu-overlay').hidden = name !== 'menu';
    $('game-area').hidden = name === 'loading' || name === 'menu';
    $('gameover-overlay').hidden = name !== 'gameover';
  }

  _fatal(message) {
    $('error-text').textContent = message;
    $('error-overlay').hidden = false;
  }

  async startGame(mode) {
    this.gen += 1;
    const gen = this.gen;
    this.mode = mode;
    this.aiThinking = false;
    this.busy = false;
    this.avaPaused = false;
    this.selected = null;
    const humanFirst = this.humanFirst;
    const difficulty = this.difficulty;
    try {
      const res = await this.client.newGame({ mode, humanFirst, difficulty });
      if (gen !== this.gen) return;
      this._setState(res.state);
      this.renderer.clearAnimations();
      this.renderer.selected = null;
      this.renderer.targets = [];
      $('btn-pause').hidden = mode !== 'ai_ai';
      $('btn-undo').hidden = mode !== 'human_ai';
      $('btn-resign').hidden = false;
      $('btn-pause').textContent = 'Pause';
      this._showScreen('playing');
      this._updatePanel();
      if (mode === 'ai_ai' || res.state.turn !== 'gold') {
        this._requestAiMove(gen);
      }
    } catch (err) {
      if (gen === this.gen) this._fatal(err.message);
    }
  }

  toMenu() {
    this.gen += 1;
    this.aiThinking = false;
    this.busy = false;
    this.selected = null;
    if (this.renderer) this.renderer.clearAnimations();
    this._showScreen('menu');
  }

  togglePause() {
    this.avaPaused = !this.avaPaused;
    $('btn-pause').textContent = this.avaPaused ? 'Resume' : 'Pause';
    if (!this.avaPaused && !this.aiThinking && this.screen === 'playing') {
      this._requestAiMove(this.gen);
    }
    this._updatePanel();
  }

  toggleFlip() {
    if (!this.renderer) return;
    this.renderer.flipped = !this.renderer.flipped;
    this._updatePanel();
  }

  async undo() {
    if (this.mode !== 'human_ai' || this.aiThinking || this.busy) return;
    if (!this.state || this.state.ply < 1 || this.state.terminal) return;
    const gen = this.gen;
    this.busy = true;
    this._updatePanel();
    try {
      const res = await this.client.undo();
      if (gen !== this.gen) return;
      this._setState(res.state);
      this.renderer.clearAnimations();
      this._select(null);
      this._showScreen('playing');
      this._updatePanel();
      if (this.state.turn === 'scarlet' && !this.state.terminal) {
        this._requestAiMove(gen);
      }
    } catch {
      // nothing to undo
    } finally {
      if (gen === this.gen) {
        this.busy = false;
        this._updatePanel();
      }
    }
  }

  async resign() {
    if (!this.state || this.state.terminal || this.aiThinking || this.busy) return;
    const gen = this.gen;
    this.busy = true;
    try {
      const res = await this.client.resign();
      if (gen !== this.gen) return;
      this._setState(res.state);
      this._finishGame(gen);
    } catch (err) {
      if (gen === this.gen) this._fatal(err.message);
    } finally {
      if (gen === this.gen) this.busy = false;
    }
  }

  _onKey(ev) {
    if (ev.target && ['INPUT', 'TEXTAREA', 'SELECT'].includes(ev.target.tagName)) return;
    if (ev.key === 'f' || ev.key === 'F') {
      if (this.screen === 'playing' || this.screen === 'gameover') this.toggleFlip();
    } else if ((ev.key === 'z' || ev.key === 'Z') && ev.ctrlKey) {
      ev.preventDefault();
      this.undo();
    } else if ((ev.key === 'd' || ev.key === 'D') && ev.ctrlKey) {
      ev.preventDefault();
      if (this.renderer) this.renderer.debugCoords = !this.renderer.debugCoords;
    }
  }

  _onBoardClick(ev) {
    if (this.screen !== 'playing' || this.mode !== 'human_ai') return;
    if (this.aiThinking || this.busy || !this.state || this.state.terminal) return;
    if (this.state.turn !== 'gold') return;

    const rect = $('board').getBoundingClientRect();
    const name = this.renderer.pixelToSquare(ev.clientX - rect.left, ev.clientY - rect.top);
    if (!name) {
      this._select(null);
      return;
    }

    if (this.selected) {
      const move = this.renderer.targets.find((m) => m.dst === name);
      if (move) {
        this._select(null);
        this._applyHumanMove(move);
        return;
      }
    }

    const piece = this.state.board.find((p) => p.sq === name);
    if (piece && piece.side === 'gold' && this.state.turn === 'gold') {
      const moves = this.state.legalMoves.filter((m) => m.src === name);
      this._select(moves.length ? name : null, moves);
    } else {
      this._select(null);
    }
  }

  _select(square, moves = []) {
    this.selected = square;
    this.renderer.selected = square;
    this.renderer.targets = square ? moves : [];
  }

  async _applyHumanMove(move) {
    const gen = this.gen;
    this.busy = true;
    this._updatePanel();
    try {
      const res = await this.client.applyMove(move.src, move.dst);
      if (gen !== this.gen) return;
      this._animateMove(res, res.state);
      if (res.state.terminal) {
        this._finishGame(gen);
        return;
      }
      this._requestAiMove(gen);
    } catch (err) {
      if (gen === this.gen) this._fatal(err.message);
    } finally {
      if (gen === this.gen) this.busy = false;
    }
  }

  async _requestAiMove(gen) {
    this.aiThinking = true;
    this._updatePanel();
    try {
      const res = await this.client.aiMove();
      if (gen !== this.gen) return;
      await this._waitForAnimation();
      if (gen !== this.gen) return;
      this._animateMove(res, res.state);
      if (res.state.terminal) {
        this._finishGame(gen);
        return;
      }
      if (this.mode === 'ai_ai' && !this.avaPaused) {
        setTimeout(() => {
          if (gen === this.gen && this.screen === 'playing' && !this.avaPaused) {
            this._requestAiMove(gen);
          }
        }, AVA_MOVE_DELAY_MS);
      }
      this._updatePanel();
    } catch (err) {
      if (gen === this.gen) this._fatal(err.message);
    } finally {
      if (gen === this.gen) {
        this.aiThinking = false;
        this._updatePanel();
      }
    }
  }

  _waitForAnimation() {
    return new Promise((resolve) => {
      const check = () => {
        if (!this.renderer.hasActiveAnimation()) resolve();
        else setTimeout(check, 40);
      };
      check();
    });
  }

  _animateMove(res, newState) {
    const piece = {
      sq: res.move.dst,
      side: this._moverSide(res),
      type: this._moverType(res, newState),
      rank: this._moverRank(res, newState),
    };
    this.renderer.startMoveAnimation(piece, res.move.src, res.move.dst);
    if (res.captured) {
      this.renderer.triggerCaptureFlash(res.move.dst);
      this.audio.play('capture');
    } else {
      this.audio.play('move');
    }
    this._setState(newState);
    this._updatePanel();
  }

  _moverSide(res) {
    // After the move, turn has flipped; the mover is the opposite of the new turn.
    return res.state.turn === 'gold' ? 'scarlet' : 'gold';
  }

  _moverType(res, newState) {
    const landed = newState.board.find((p) => p.sq === res.move.dst);
    return landed ? landed.type : 'rat';
  }

  _moverRank(res, newState) {
    const landed = newState.board.find((p) => p.sq === res.move.dst);
    return landed ? landed.rank : 1;
  }

  _finishGame(gen) {
    this.aiThinking = false;
    this.audio.play('win');
    const title = this._resultTitle();
    const sub = this._resultSub();
    this._updatePanel();
    setTimeout(() => {
      if (gen === this.gen && this.state && this.state.terminal &&
          (this.screen === 'playing' || this.screen === 'gameover')) {
        $('gameover-title').textContent = title;
        $('gameover-sub').textContent = sub;
        this._showScreen('gameover');
        $('game-area').hidden = false;
      }
    }, 450);
  }

  _resultTitle() {
    const r = this.state.result;
    if (r === 'draw') return 'Draw';
    if (this.mode === 'human_ai') {
      return r === 'gold_wins' ? 'You win' : 'Scarlet wins';
    }
    return r === 'gold_wins' ? 'Gold wins' : 'Scarlet wins';
  }

  _resultSub() {
    const why = {
      den: 'entered the den',
      wipe: 'captured every animal',
      no_moves: 'opponent has no move',
      resign: 'resignation',
      repetition: 'threefold repetition',
      fifty: 'hundred quiet half-moves',
    }[this.state.outcome] || '';
    if (this.state.result === 'draw') return why;
    const who = this.state.result === 'gold_wins' ? 'Gold' : 'Scarlet';
    return `${who} — ${why}`;
  }

  _setState(state) {
    this.state = state;
    this.renderer.state = state;
    window.__jungle.state = state;
  }

  _updatePanel() {
    if (!this.state) return;
    const turnName = this.state.turn === 'gold' ? 'Gold' : 'Scarlet';
    let status;
    if (this.state.terminal) {
      status = this._resultTitle();
    } else if (this.mode === 'ai_ai') {
      status = this.avaPaused ? `Paused — ${turnName} to move`
        : `${turnName} is thinking…`;
    } else if (this.aiThinking) {
      status = `${turnName} is thinking…`;
    } else if (this.state.turn === 'gold') {
      status = 'Gold to move — your turn';
    } else {
      status = `${turnName} to move`;
    }
    $('status-text').textContent = status;
    $('status-spinner').hidden = !(this.aiThinking ||
      (this.mode === 'ai_ai' && !this.avaPaused && !this.state.terminal));

    $('btn-undo').disabled = !(this.mode === 'human_ai' && !this.aiThinking &&
      !this.busy && this.state.ply >= 1 && !this.state.terminal);
    $('btn-resign').disabled = this.state.terminal || this.aiThinking || this.busy;

    const diffLabel = { easy: 'Easy', normal: 'Normal', strong: 'Strong' }[this.state.difficulty];
    const first = this.state.humanFirst ? 'Human' : 'AI';
    const flip = this.renderer && this.renderer.flipped ? 'flipped' : 'south at bottom';
    const mode = this.mode === 'ai_ai' ? 'AI vs AI' : 'Human vs AI';
    $('game-sub').textContent = `${mode} · ${diffLabel}`;
    $('meta-text').textContent =
      `First move: ${first}\nAI: ${diffLabel}\nView: ${flip}\nPly: ${this.state.ply}`;

    $('captured-scarlet').textContent = this.state.captured.scarlet.join('  ') || '—';
    $('captured-gold').textContent = this.state.captured.gold.join('  ') || '—';

    const last = $('last-move');
    if (this.state.lastMove) {
      last.textContent = `Last: ${this.state.lastMove.src} → ${this.state.lastMove.dst}${this.state.lastMove.isJump ? ' (jump)' : ''}`;
    } else {
      last.textContent = 'New arrangement. Gold sits south, Scarlet north.';
    }

    const list = $('history-list');
    list.innerHTML = '';
    this.state.history.forEach((uci, i) => {
      const li = document.createElement('li');
      const opener = this.state.humanFirst ? 'G' : 'S';
      const tag = i % 2 === 0 ? opener : (opener === 'G' ? 'S' : 'G');
      li.textContent = `${i + 1}. ${tag} ${uci}`;
      list.appendChild(li);
    });
    list.scrollTop = list.scrollHeight;
  }
}

window.__jungle = {
  ready: false,
  state: null,
  app: null,
  squareToPixel(name) {
    return window.__jungle.app && window.__jungle.app.renderer
      ? window.__jungle.app.renderer.squareToPixel(name)
      : null;
  },
};

const app = new App();
window.__jungle.app = app;
app.boot();
