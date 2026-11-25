// Arena admission checklist and a two-game match against the fixture.
// Usage: node tools/conformance.mjs
// Transpiles the exact release bytes with @bytecodealliance/jco-transpile 0.24.0.

import { spawn, spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { transpileBytes } from '@bytecodealliance/jco-transpile';
import { legalWire, snapshotFromState } from '../public/js/wire-map.js';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');
const RELEASE_NAME = 'grok-4-7-grokbuild-2.0.0.wasm';
const RELEASE = join(ROOT, 'release', RELEASE_NAME);
const FIXTURE = join(ROOT, 'public', 'wasm', 'fixture.wasm');
const OUT = join(ROOT, 'public', 'conformance');
const PREAMBLE = Buffer.from([0x00, 0x61, 0x73, 0x6d, 0x0d, 0x00, 0x01, 0x00]);
const RULES_DIGEST = 'a457bbc009e975a9eafe5ff1f8fa40ca2548f2a689b121c8c4ed99dd0f3fdc57';
const FRAME = 448 * 576 * 4;

let failures = 0;
function check(cond, label) {
  console.log(`${cond ? 'ok' : 'FAIL'} - ${label}`);
  if (!cond) failures++;
}

function sha256(buf) {
  return createHash('sha256').update(buf).digest('hex');
}

class Referee {
  constructor() {
    const bin = join(
      ROOT, 'engine', 'target', 'release',
      process.platform === 'win32' ? 'jungle-engine.exe' : 'jungle-engine',
    );
    this.proc = spawn(bin, ['stdio'], { stdio: ['pipe', 'pipe', 'inherit'] });
    this.buf = '';
    this.waiters = [];
    this.proc.stdout.on('data', (chunk) => {
      this.buf += chunk.toString('utf8');
      let nl;
      while ((nl = this.buf.indexOf('\n')) >= 0) {
        const line = this.buf.slice(0, nl);
        this.buf = this.buf.slice(nl + 1);
        const waiter = this.waiters.shift();
        if (waiter) waiter(line);
      }
    });
  }

  call(line) {
    return new Promise((resolve, reject) => {
      this.waiters.push(resolve);
      this.proc.stdin.write(line + '\n', (err) => {
        if (err) reject(err);
      });
    }).then((raw) => JSON.parse(raw));
  }

  close() {
    try { this.proc.stdin.write('quit\n'); } catch { /* gone */ }
    this.proc.kill();
  }
}

async function loadComponent(bytes, name) {
  const t0 = Date.now();
  const result = await transpileBytes(bytes, {
    name,
    instantiation: 'async',
    wasiShim: false,
    minify: false,
    optimize: false,
    emitTypescriptDeclarations: false,
  });
  const ms = Date.now() - t0;
  mkdirSync(OUT, { recursive: true });
  for (const [file, data] of Object.entries(result.files)) {
    writeFileSync(join(OUT, file), data);
  }
  const href = pathToFileURL(join(OUT, `${name}.js`)).href;
  const mod = await import(href);
  return { result, ms, mod, files: result.files };
}

async function instantiate(loaded, onPresent) {
  const { game } = await loaded.mod.instantiate(
    (file) => WebAssembly.compile(loaded.files[file]),
    {
      'jungle:arena/display': {
        present(width, height, rgba) {
          onPresent(width, height, new Uint8Array(rgba));
        },
      },
    },
  );
  return game;
}

function openingSnapshot(state) {
  return snapshotFromState(state, [], 'red');
}

async function main() {
  const bytes = readFileSync(RELEASE);
  const fixtureBytes = readFileSync(FIXTURE);
  check(bytes.length <= 24 * 1024 * 1024, `size ${bytes.length} <= 24 MiB`);
  check(bytes.subarray(0, 8).equals(PREAMBLE), 'component preamble');
  check(!fixtureBytes.subarray(0, 8).equals(Buffer.from([0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00])),
    'fixture is not a core module');
  const hash = sha256(bytes);
  const recorded = readFileSync(join(ROOT, 'release', 'SHA256.txt'), 'utf8');
  check(recorded.includes(hash), `SHA-256 ${hash} recorded`);
  mkdirSync(OUT, { recursive: true });
  writeFileSync(join(OUT, 'release.sha256'), `${hash}\n`);

  const wit = spawnSync('wasm-tools', ['component', 'wit', RELEASE], { encoding: 'utf8' });
  check(wit.status === 0, 'wasm-tools component wit');
  const witText = wit.stdout || '';
  check(witText.includes('import jungle:arena/display@1.0.0;'), 'only display import');
  check(witText.includes('export jungle:arena/game@1.0.0;'), 'only game export');
  check(!/wasi:/i.test(witText), 'no WASI import');
  const importCount = (witText.match(/^\s*import /gm) || []).length;
  const exportCount = (witText.match(/^\s*export /gm) || []).length;
  check(importCount === 1 && exportCount === 1, `world has 1 import and 1 export (${importCount}/${exportCount})`);

  const release = await loadComponent(bytes, 'release');
  check(release.ms < 30000, `jco transpile in ${release.ms} ms`);
  check(release.result.imports.length === 1 && release.result.imports[0] === 'jungle:arena/display',
    `jco imports ${release.result.imports.join(',')}`);
  const fixture = await loadComponent(fixtureBytes, 'fixture');

  const frames = [];
  const game = await instantiate(release, (w, h, rgba) => frames.push({ w, h, n: rgba.length, rgba }));
  const info = game.info();
  check(info.rulesId === 'desktop-jungle-v2', 'rules id');
  check(info.rulesDigest === RULES_DIGEST, 'rules digest');
  check(info.title === 'Grok-4.7 · GrokBuild', 'title');
  check(info.githubUrl.startsWith('https://github.com/'), 'github url');
  check(info.modelPlatform === 'official', 'platform');
  for (const [key, value] of Object.entries(info)) {
    check(String(value).length <= 160, `${key} length`);
    if (key !== 'engineVersion' && key !== 'modelPlatform') {
      check(String(value).length > 0, `${key} non-empty`);
    }
  }

  const ref = new Referee();
  try {
    let rec = await ref.call('NEW ai_ai 1 medium');
    check(rec.ok, 'referee opening');
    let state = rec.data.state;
    const legal = legalWire(state);
    check(legal.length === 24, 'opening has 24 moves');
    check(legal[0].src === 42 && legal[0].dst === 35, 'opening index 0 is 42→35');
    check(legal[23].src === 62 && legal[23].dst === 61, 'opening index 23 is 62→61');

    game.init({ seat: 'red', difficulty: 'medium', seed: 1 });
    const snap = openingSnapshot(state);
    const idx = game.chooseMove(snap, legal.map(({ src, dst }) => ({ src, dst })), 1000);
    check(idx >= 0 && idx < 24, `opening choose-move index ${idx}`);

    frames.length = 0;
    game.render(snap, false, 1000);
    game.render(snap, true, 1000);
    game.render(snap, false, 1000);
    check(frames.length === 3, 'three present calls');
    check(frames.every((f) => f.w === 448 && f.h === 576 && f.n === FRAME), 'frame geometry');
    check(frames[0].rgba.every((b, i) => i % 4 !== 3 || b === 255), 'opaque');
    check(frames[0].rgba.some((b, i) => b !== frames[1].rgba[i]), 'flip changes bytes');
    check(frames[0].rgba.every((b, i) => b === frames[2].rgba[i]), 'unflipped render is stable');

    const again = game.chooseMove(snap, legal.map(({ src, dst }) => ({ src, dst })), 1000);
    check(again === idx, 'render does not change the choice');

    const framesB = [];
    const other = await instantiate(release, (w, h, rgba) => framesB.push(rgba));
    other.init({ seat: 'red', difficulty: 'medium', seed: 1 });
    const isolated = other.chooseMove(snap, legal.map(({ src, dst }) => ({ src, dst })), 1000);
    check(isolated === idx, 'a second instance makes the same choice');

    const easyA = await instantiate(release, () => {});
    const easyB = await instantiate(release, () => {});
    easyA.init({ seat: 'red', difficulty: 'easy', seed: 7 });
    easyB.init({ seat: 'blue', difficulty: 'hard', seed: 99 });
    const easyIdx = easyA.chooseMove(snap, legal.map(({ src, dst }) => ({ src, dst })), 1000);
    const freshEasy = await instantiate(release, () => {});
    freshEasy.init({ seat: 'red', difficulty: 'easy', seed: 7 });
    const freshIdx = freshEasy.chooseMove(snap, legal.map(({ src, dst }) => ({ src, dst })), 1000);
    check(easyIdx === freshIdx, 'instances do not share difficulty');

    check(game.chooseMove(snap, [], 1000) === 0, 'empty legal list returns 0');
    check(game.chooseMove(snap, legal.map(({ src, dst }) => ({ src, dst })), 0) === 0, 'zero budget returns 0');

    const hard = await instantiate(release, () => {});
    hard.init({ seat: 'red', difficulty: 'hard', seed: 1 });
    const hardIdx = hard.chooseMove(snap, legal.map(({ src, dst }) => ({ src, dst })), 1000);
    const fen = state.fen;
    const searched = await ref.call(`NODES 1000 1 1 ${fen}`);
    check(searched.ok, 'shell node search');
    const uci = searched.data.uci;
    const shellIdx = legal.findIndex((m) => m.algSrc + m.algDst === uci.slice(0, 4));
    check(shellIdx === hardIdx, `shell/component parity at 1000 nodes (${uci} vs index ${hardIdx})`);

    const blueOpen = await ref.call('NEW ai_ai 0 medium');
    const blueLegal = legalWire(blueOpen.data.state);
    check(blueLegal[0].src === 0 && blueLegal[0].dst === 1, 'Blue first index 0 is 0→1');

    async function match(releaseSeat) {
      const seatFrames = [];
      const redGame = await instantiate(releaseSeat === 'red' ? release : fixture, (w, h, rgba) => {
        seatFrames.push({ who: 'red', w, h, n: rgba.length });
      });
      const blueGame = await instantiate(releaseSeat === 'blue' ? release : fixture, (w, h, rgba) => {
        seatFrames.push({ who: 'blue', w, h, n: rgba.length });
      });
      redGame.init({ seat: 'red', difficulty: 'medium', seed: 20261120 });
      blueGame.init({ seat: 'blue', difficulty: 'medium', seed: 20261120 });
      let started = await ref.call('NEW ai_ai 1 medium');
      let cur = started.data.state;
      const history = [];
      let ply = 0;
      const players = { red: redGame, blue: blueGame };
      while (!cur.terminal && ply < 240) {
        const moves = legalWire(cur);
        const snapNow = snapshotFromState(cur, history, 'red');
        const side = cur.turn;
        const choice = players[side].chooseMove(
          snapNow,
          moves.map(({ src, dst }) => ({ src, dst })),
          20000,
        );
        if (choice < 0 || choice >= moves.length) {
          return { ok: false, reason: 'index', ply, result: cur.result };
        }
        const mv = moves[choice];
        const applied = await ref.call(`APPLY ${mv.algSrc} ${mv.algDst}`);
        if (!applied.ok) return { ok: false, reason: applied.error, ply, result: null };
        history.push([mv.src, mv.dst]);
        cur = applied.data.state;
        const shown = snapshotFromState(cur, history, 'red');
        redGame.render(shown, false, 1000);
        blueGame.render(shown, true, 1000);
        ply++;
      }
      const good = ['den', 'elimination', 'immobilization', 'repetition', 'no-capture'];
      return {
        ok: cur.terminal && good.includes(cur.outcome),
        result: cur.result,
        outcome: cur.outcome,
        ply,
        frames: seatFrames.length,
      };
    }

    const asRed = await match('red');
    check(asRed.ok, `release as Red ends ${asRed.result} ${asRed.outcome} in ${asRed.ply}`);
    const asBlue = await match('blue');
    check(asBlue.ok, `release as Blue ends ${asBlue.result} ${asBlue.outcome} in ${asBlue.ply}`);
    check(asRed.frames > 0 && asBlue.frames > 0, 'both boards presented frames');
  } finally {
    ref.close();
  }

  if (failures) {
    console.error(`# ${failures} failure(s)`);
    process.exit(1);
  }
  console.log('# conformance passed');
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
