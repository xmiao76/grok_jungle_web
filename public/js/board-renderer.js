// Canvas renderer: lacquer table, terrain, portraits, nameplates, highlights.
// Engine coordinates stay unflipped; flip is applied only in coords.js.

import {
  FILES, RANKS, boardToPixel, parseSquare, squareName, pixelToBoard,
} from './coords.js';

const MOVE_ANIM_MS = 240;
const CAPTURE_FLASH_MS = 320;

const SPECIES = {
  rat: '#8B5A2B',
  cat: '#E09A3D',
  dog: '#C47A3A',
  wolf: '#6B7380',
  leopard: '#D4A017',
  tiger: '#E85D12',
  lion: '#F0C43A',
  elephant: '#7A838C',
};

// Terrain in engine rank 0 = south. 0 land, 1 water, 2 trap, 3 den.
export const TERRAIN = (() => {
  const t = Array.from({ length: RANKS }, () => Array(FILES).fill(0));
  for (const [f, r] of [
    [1, 3], [2, 3], [1, 4], [2, 4], [1, 5], [2, 5],
    [4, 3], [5, 3], [4, 4], [5, 4], [4, 5], [5, 5],
  ]) t[r][f] = 1;
  for (const [f, r] of [[2, 0], [3, 1], [4, 0], [2, 8], [3, 7], [4, 8]]) t[r][f] = 2;
  t[0][3] = 3;
  t[8][3] = 3;
  return t;
})();

export class BoardRenderer {
  constructor(canvas, assets) {
    this.canvas = canvas;
    this.ctx = canvas.getContext('2d');
    this.assets = assets;
    this.cellSize = 64;
    this.frame = 28;
    this.flipped = false;
    this.debugCoords = false;
    this.state = null;
    this.selected = null;
    this.targets = [];
    this._animations = [];
    this._flashes = [];
  }

  get boardWidth() {
    return this.frame * 2 + FILES * this.cellSize;
  }

  get boardHeight() {
    return this.frame * 2 + RANKS * this.cellSize;
  }

  resize(cellSize) {
    const dpr = window.devicePixelRatio || 1;
    this.cellSize = cellSize;
    this.frame = Math.max(22, Math.round(cellSize * 0.40));
    const w = this.boardWidth;
    const h = this.boardHeight;
    this.canvas.width = w * dpr;
    this.canvas.height = h * dpr;
    this.canvas.style.width = `${w}px`;
    this.canvas.style.height = `${h}px`;
    this.ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  }

  squareToPixel(name) {
    const sq = parseSquare(name);
    if (!sq) return null;
    const [x, y] = boardToPixel(sq.file, sq.rank, this.cellSize, this.flipped);
    return {
      x: this.frame + x + this.cellSize / 2,
      y: this.frame + y + this.cellSize / 2,
    };
  }

  pixelToSquare(px, py) {
    const sq = pixelToBoard(px - this.frame, py - this.frame, this.cellSize, this.flipped);
    return sq ? squareName(sq.file, sq.rank) : null;
  }

  startMoveAnimation(piece, src, dst) {
    this._animations.push({
      piece, src, dst, start: performance.now(),
    });
  }

  triggerCaptureFlash(sq) {
    this._flashes.push({ sq, start: performance.now() });
  }

  hasActiveAnimation(now = performance.now()) {
    return this._animations.some((a) => now - a.start < MOVE_ANIM_MS);
  }

  clearAnimations() {
    this._animations = [];
    this._flashes = [];
  }

  draw(now = performance.now()) {
    this._animations = this._animations.filter((a) => now - a.start < MOVE_ANIM_MS);
    this._flashes = this._flashes.filter((f) => now - f.start < CAPTURE_FLASH_MS);
    const { ctx } = this;
    const w = this.boardWidth;
    const h = this.boardHeight;
    ctx.clearRect(0, 0, w, h);
    this._drawTable(w, h);
    this._drawCells();
    this._drawLastMove();
    this._drawSelection();
    this._drawPieces(now);
    this._drawFlashes(now);
    this._drawLabels();
  }

  _origin() {
    return this.frame;
  }

  _cellOrigin(file, rank) {
    const [x, y] = boardToPixel(file, rank, this.cellSize, this.flipped);
    return [this.frame + x, this.frame + y];
  }

  _drawTable(w, h) {
    const { ctx } = this;
    const g = ctx.createLinearGradient(0, 0, w, h);
    g.addColorStop(0, '#3A2218');
    g.addColorStop(1, '#1A100C');
    ctx.fillStyle = g;
    ctx.fillRect(0, 0, w, h);
    ctx.strokeStyle = '#C9A227';
    ctx.lineWidth = 3.5;
    roundRect(ctx, 6, 6, w - 12, h - 12, 8);
    ctx.stroke();
    ctx.strokeStyle = '#E8D48B';
    ctx.lineWidth = 1.2;
    roundRect(ctx, 12, 12, w - 24, h - 24, 6);
    ctx.stroke();
  }

  _drawCells() {
    const occ = this._occupancy();
    for (let rank = 0; rank < RANKS; rank++) {
      for (let file = 0; file < FILES; file++) {
        const [px, py] = this._cellOrigin(file, rank);
        this._paintTerrain(file, rank, px, py, occ.has(squareName(file, rank)));
      }
    }
  }

  _paintTerrain(file, rank, px, py, occupied) {
    const { ctx, cellSize: cs } = this;
    const kind = TERRAIN[rank][file];
    if (kind === 1) {
      const g = ctx.createLinearGradient(px, py, px + cs, py + cs);
      g.addColorStop(0, '#4AD4DC');
      g.addColorStop(0.5, '#178A94');
      g.addColorStop(1, '#0A3E46');
      ctx.fillStyle = g;
      ctx.fillRect(px, py, cs, cs);
      const water = this.assets.terrain.water;
      if (water) {
        ctx.globalAlpha = 0.32;
        ctx.drawImage(water, px, py, cs, cs);
        ctx.globalAlpha = 1;
      }
      ctx.strokeStyle = 'rgba(220,255,255,0.45)';
      ctx.lineWidth = 1.6;
      for (let i = 0; i < 3; i++) {
        const y = py + cs * (0.28 + i * 0.22);
        ctx.beginPath();
        ctx.ellipse(px + cs / 2, y, cs * 0.36, 5, 0, 0.2, Math.PI - 0.2);
        ctx.stroke();
      }
      if (!occupied) this._cellCaption(px, py, 'RIVER', 'rgba(240,255,255,0.7)');
    } else if (kind === 2) {
      ctx.fillStyle = (file + rank) % 2 === 0 ? '#7A4024' : '#5C2C16';
      ctx.fillRect(px, py, cs, cs);
      const pit = cs * 0.16;
      ctx.fillStyle = '#140A08';
      ctx.strokeStyle = '#F0D060';
      ctx.lineWidth = 2.2;
      roundRect(ctx, px + pit, py + pit, cs - pit * 2, cs - pit * 2, 6);
      ctx.fill();
      ctx.stroke();
      ctx.beginPath();
      ctx.moveTo(px + pit + 6, py + pit + 6);
      ctx.lineTo(px + cs - pit - 6, py + cs - pit - 6);
      ctx.moveTo(px + cs - pit - 6, py + pit + 6);
      ctx.lineTo(px + pit + 6, py + cs - pit - 6);
      ctx.strokeStyle = '#E8C547';
      ctx.stroke();
      if (!occupied) this._cellCaption(px, py, 'TRAP', '#F3E6C8', true);
    } else if (kind === 3) {
      const goldDen = rank === 0;
      const team = goldDen ? '#F2D36B' : '#3EE0D2';
      ctx.fillStyle = '#1A100C';
      ctx.fillRect(px, py, cs, cs);
      ctx.fillStyle = '#0C0806';
      ctx.strokeStyle = team;
      ctx.lineWidth = 3.2;
      ctx.beginPath();
      ctx.moveTo(px + 12, py + cs - 8);
      ctx.lineTo(px + 12, py + cs * 0.52);
      ctx.quadraticCurveTo(px + cs / 2, py + 10, px + cs - 12, py + cs * 0.52);
      ctx.lineTo(px + cs - 12, py + cs - 8);
      ctx.closePath();
      ctx.fill();
      ctx.stroke();
      ctx.strokeStyle = team;
      ctx.lineWidth = 3;
      roundRect(ctx, px + 3, py + 3, cs - 6, cs - 6, 7);
      ctx.stroke();
      if (!occupied) this._cellCaption(px, py, 'DEN', team, true);
    } else {
      ctx.fillStyle = (file + rank) % 2 === 0 ? '#E8C99A' : '#6E3B24';
      ctx.fillRect(px, py, cs, cs);
    }
    ctx.strokeStyle = 'rgba(30,14,8,0.5)';
    ctx.lineWidth = 1;
    ctx.strokeRect(px + 0.5, py + 0.5, cs - 1, cs - 1);
    if (this.debugCoords) {
      ctx.fillStyle = 'rgba(20,12,8,0.75)';
      ctx.font = `600 ${Math.max(9, Math.round(cs * 0.16))}px "Segoe UI", sans-serif`;
      ctx.textAlign = 'left';
      ctx.textBaseline = 'top';
      ctx.fillText(squareName(file, rank), px + 3, py + 3);
    }
  }

  _cellCaption(px, py, text, color, bottom) {
    const { ctx, cellSize: cs } = this;
    ctx.fillStyle = color;
    ctx.font = `700 ${Math.max(8, Math.round(cs * 0.12))}px "Segoe UI", sans-serif`;
    ctx.textAlign = 'center';
    ctx.textBaseline = bottom ? 'bottom' : 'top';
    const y = bottom ? py + cs - 4 : py + 3;
    ctx.fillText(text, px + cs / 2, y);
  }

  _drawLastMove() {
    const last = this.state && this.state.lastMove;
    if (!last) return;
    const { ctx, cellSize: cs } = this;
    const src = parseSquare(last.src);
    const dst = parseSquare(last.dst);
    for (const sq of [src, dst]) {
      if (!sq) continue;
      const [px, py] = this._cellOrigin(sq.file, sq.rank);
      ctx.fillStyle = 'rgba(232, 196, 64, 0.38)';
      ctx.fillRect(px, py, cs, cs);
      ctx.strokeStyle = '#F2D36B';
      ctx.lineWidth = 3;
      roundRect(ctx, px + 2.5, py + 2.5, cs - 5, cs - 5, 5);
      ctx.stroke();
    }
    if (src && dst) {
      const [sx, sy] = this._cellOrigin(src.file, src.rank);
      const [dx, dy] = this._cellOrigin(dst.file, dst.rank);
      const x0 = sx + cs / 2;
      const y0 = sy + cs / 2;
      const x1 = dx + cs / 2;
      const y1 = dy + cs / 2;
      ctx.strokeStyle = 'rgba(242, 211, 107, 0.9)';
      ctx.lineWidth = Math.max(2.5, cs * 0.045);
      ctx.lineCap = 'round';
      ctx.beginPath();
      ctx.moveTo(x0, y0);
      ctx.lineTo(x1, y1);
      ctx.stroke();
      const ang = Math.atan2(y1 - y0, x1 - x0);
      const head = Math.max(7, cs * 0.16);
      ctx.fillStyle = '#F2D36B';
      ctx.beginPath();
      ctx.moveTo(x1, y1);
      ctx.lineTo(x1 - head * Math.cos(ang - 0.45), y1 - head * Math.sin(ang - 0.45));
      ctx.lineTo(x1 - head * Math.cos(ang + 0.45), y1 - head * Math.sin(ang + 0.45));
      ctx.closePath();
      ctx.fill();
    }
  }

  _drawSelection() {
    const { ctx, cellSize: cs } = this;
    if (this.selected) {
      const sq = parseSquare(this.selected);
      if (sq) {
        const [px, py] = this._cellOrigin(sq.file, sq.rank);
        ctx.fillStyle = 'rgba(242, 211, 107, 0.22)';
        ctx.strokeStyle = '#F8E38A';
        ctx.lineWidth = 5;
        roundRect(ctx, px + 2, py + 2, cs - 4, cs - 4, 8);
        ctx.fill();
        ctx.stroke();
      }
    }
    const occ = this._occupancy();
    for (const m of this.targets) {
      const sq = parseSquare(m.dst);
      if (!sq) continue;
      const [px, py] = this._cellOrigin(sq.file, sq.rank);
      const cx = px + cs / 2;
      const cy = py + cs / 2;
      if (occ.has(m.dst)) {
        ctx.beginPath();
        ctx.arc(cx, cy, cs * 0.36, 0, Math.PI * 2);
        ctx.strokeStyle = '#1A1008';
        ctx.lineWidth = 6;
        ctx.stroke();
        ctx.beginPath();
        ctx.arc(cx, cy, cs * 0.36, 0, Math.PI * 2);
        ctx.strokeStyle = '#E85D4C';
        ctx.lineWidth = 3.6;
        ctx.stroke();
      } else {
        ctx.beginPath();
        ctx.arc(cx, cy, Math.max(6, cs * 0.14), 0, Math.PI * 2);
        ctx.fillStyle = 'rgba(20, 40, 32, 0.45)';
        ctx.fill();
        ctx.beginPath();
        ctx.arc(cx, cy, Math.max(5, cs * 0.12), 0, Math.PI * 2);
        ctx.fillStyle = '#3AA37F';
        ctx.fill();
      }
    }
  }

  _occupancy() {
    const set = new Set();
    if (this.state) {
      for (const p of this.state.board) set.add(p.sq);
    }
    return set;
  }

  _drawPieces(now) {
    if (!this.state) return;
    const movingDest = new Set(this._animations.map((a) => a.dst));
    for (const piece of this.state.board) {
      if (movingDest.has(piece.sq)) continue;
      const sq = parseSquare(piece.sq);
      if (!sq) continue;
      const [px, py] = this._cellOrigin(sq.file, sq.rank);
      this._drawPiece(piece, px, py);
    }
    for (const a of this._animations) {
      const t = Math.min((now - a.start) / MOVE_ANIM_MS, 1);
      const ease = 1 - (1 - t) * (1 - t);
      const src = parseSquare(a.src);
      const dst = parseSquare(a.dst);
      if (!src || !dst) continue;
      const [fx, fy] = this._cellOrigin(src.file, src.rank);
      const [tx, ty] = this._cellOrigin(dst.file, dst.rank);
      this._drawPiece(a.piece, fx + (tx - fx) * ease, fy + (ty - fy) * ease);
    }
  }

  _drawPiece(piece, px, py) {
    const { ctx, cellSize: cs } = this;
    const red = piece.side === 'red';
    const artSide = red ? 'gold' : 'scarlet';
    const img = this.assets.pieces[`${artSide}_${piece.type}`];
    const team = red ? '#F2D36B' : '#3EE0D2';
    const padFill = red ? '#C8962A' : '#157A74';
    const padEdge = red ? '#6A4208' : '#083F3C';
    const cx = px + cs / 2;
    const cy = py + cs / 2;
    const padR = cs * 0.44;
    const portrait = cs * 0.78;
    const py0 = py + cs * 0.02;

    ctx.beginPath();
    ctx.arc(cx, cy, padR, 0, Math.PI * 2);
    ctx.fillStyle = 'rgba(12, 8, 6, 0.35)';
    ctx.fill();
    ctx.beginPath();
    ctx.arc(cx, cy - cs * 0.01, padR, 0, Math.PI * 2);
    ctx.fillStyle = padFill;
    ctx.fill();
    ctx.strokeStyle = padEdge;
    ctx.lineWidth = Math.max(2, cs * 0.035);
    ctx.stroke();
    ctx.strokeStyle = team;
    ctx.lineWidth = Math.max(1.6, cs * 0.022);
    ctx.stroke();

    ctx.save();
    ctx.beginPath();
    ctx.arc(cx, py0 + portrait / 2, portrait / 2 - 1, 0, Math.PI * 2);
    ctx.clip();
    if (img) ctx.drawImage(img, cx - portrait / 2, py0, portrait, portrait);
    else {
      ctx.fillStyle = SPECIES[piece.type] || '#888';
      ctx.fillRect(cx - portrait / 2, py0, portrait, portrait);
    }
    ctx.restore();

    const badgeR = Math.max(8, cs * 0.175);
    const bx = px + cs - badgeR - cs * 0.04;
    const by = py + badgeR + cs * 0.04;
    ctx.beginPath();
    ctx.arc(bx, by, badgeR, 0, Math.PI * 2);
    ctx.fillStyle = team;
    ctx.fill();
    ctx.strokeStyle = '#1A1008';
    ctx.lineWidth = 2.2;
    ctx.stroke();
    ctx.fillStyle = '#1A1008';
    ctx.font = `800 ${Math.max(11, Math.round(cs * 0.20))}px "Segoe UI", sans-serif`;
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillText(String(piece.rank), bx, by + 0.5);

    const raw = piece.type[0].toUpperCase() + piece.type.slice(1);
    const fontPx = raw.length >= 8 ? cs * 0.145 : raw.length >= 7 ? cs * 0.155 : cs * 0.175;
    ctx.font = `800 ${Math.max(10, Math.round(fontPx))}px "Segoe UI", sans-serif`;
    ctx.textAlign = 'center';
    ctx.textBaseline = 'bottom';
    ctx.lineJoin = 'round';
    ctx.lineWidth = Math.max(3, Math.round(fontPx / 3.2));
    ctx.strokeStyle = 'rgba(12, 8, 6, 0.92)';
    const ty = py + cs - cs * 0.045;
    ctx.strokeText(raw, cx, ty);
    ctx.fillStyle = '#FFF6DC';
    ctx.fillText(raw, cx, ty);
  }

  _drawFlashes(now) {
    const { ctx, cellSize: cs } = this;
    for (const f of this._flashes) {
      const sq = parseSquare(f.sq);
      if (!sq) continue;
      const t = (now - f.start) / CAPTURE_FLASH_MS;
      const [px, py] = this._cellOrigin(sq.file, sq.rank);
      ctx.fillStyle = `rgba(232,93,76,${0.5 * (1 - t)})`;
      ctx.fillRect(px, py, cs, cs);
    }
  }

  _drawLabels() {
    const { ctx, cellSize: cs, frame } = this;
    ctx.fillStyle = '#F6E4A8';
    ctx.font = `700 ${Math.max(11, Math.round(cs * 0.20))}px Palatino, Georgia, serif`;
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    for (let file = 0; file < FILES; file++) {
      const letter = squareName(this.flipped ? FILES - 1 - file : file, 0)[0];
      const x = frame + file * cs + cs / 2;
      ctx.fillText(letter, x, frame * 0.45);
      ctx.fillText(letter, x, frame + RANKS * cs + frame * 0.55);
    }
    for (let row = 0; row < RANKS; row++) {
      const rank = this.flipped ? row + 1 : RANKS - row;
      const y = frame + row * cs + cs / 2;
      ctx.fillText(String(rank), frame * 0.42, y);
      ctx.fillText(String(rank), frame + FILES * cs + frame * 0.58, y);
    }
  }
}

function roundRect(ctx, x, y, w, h, r) {
  const rad = Math.min(r, w / 2, h / 2);
  ctx.beginPath();
  ctx.moveTo(x + rad, y);
  ctx.arcTo(x + w, y, x + w, y + h, rad);
  ctx.arcTo(x + w, y + h, x, y + h, rad);
  ctx.arcTo(x, y + h, x, y, rad);
  ctx.arcTo(x, y, x + w, y, rad);
  ctx.closePath();
}

export async function loadAssets() {
  const terrain = {};
  const pieces = {};
  const load = (url) => new Promise((resolve) => {
    const img = new Image();
    img.onload = () => resolve(img);
    img.onerror = () => resolve(null);
    const embedded = globalThis.JUNGLE_ASSETS && globalThis.JUNGLE_ASSETS[url];
    img.src = embedded || url;
  });
  const animals = ['rat', 'cat', 'dog', 'wolf', 'leopard', 'tiger', 'lion', 'elephant'];
  await Promise.all([
    ...['den', 'land', 'trap', 'water'].map(async (n) => {
      terrain[n] = await load(`assets/terrain/${n}.png`);
    }),
    ...animals.flatMap((animal) => ['gold', 'scarlet'].map(async (side) => {
      pieces[`${side}_${animal}`] = await load(`assets/pieces/${side}/${animal}.png`);
    })),
  ]);
  return { terrain, pieces };
}
