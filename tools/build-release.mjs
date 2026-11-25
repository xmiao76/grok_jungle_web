// Package release/index.html: one file, playable from file://, with the
// shell engine and the exact Arena component bytes embedded.
// Usage: node tools/build-release.mjs
// Requires public/wasm (npm run build:wasm) and release/<name>.wasm.

import esbuild from 'esbuild';
import { readFileSync, writeFileSync, mkdirSync, rmSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');
const RELEASE_NAME = 'grok-4-7-grokbuild-2.0.0.wasm';
const ANIMALS = ['rat', 'cat', 'dog', 'wolf', 'leopard', 'tiger', 'lion', 'elephant'];

function b64(buf) {
  return Buffer.from(buf).toString('base64');
}

function dataUri(bytes, mime) {
  return `data:${mime};base64,${b64(bytes)}`;
}

const entryPath = join(ROOT, 'tools', '_release_entry.mjs');
writeFileSync(entryPath, `
import * as engine from '../public/wasm/jungle_engine.js';
globalThis.JUNGLE_ENGINE_MODULE = engine;
import '../public/js/main.js';
`);

let bundled;
let worker;
try {
  [bundled, worker] = await Promise.all([
    esbuild.build({
      absWorkingDir: ROOT,
      entryPoints: [entryPath],
      bundle: true,
      format: 'esm',
      platform: 'browser',
      write: false,
      legalComments: 'none',
    }),
    esbuild.build({
      absWorkingDir: ROOT,
      entryPoints: [join(ROOT, 'public/js/engine-worker.js')],
      bundle: true,
      format: 'esm',
      platform: 'browser',
      write: false,
      legalComments: 'none',
    }),
  ]);
} finally {
  rmSync(entryPath, { force: true });
}

const appJs = bundled.outputFiles[0].text.replaceAll('</script>', '<\\/script>');
const workerJs = worker.outputFiles[0].text.replaceAll('</', '<\\/');

const engineBytes = readFileSync(join(ROOT, 'public', 'wasm', 'jungle_engine_bg.wasm'));
const arenaBytes = readFileSync(join(ROOT, 'release', RELEASE_NAME));
const arenaHash = createHash('sha256').update(arenaBytes).digest('hex');

const assets = {};
for (const name of ['den', 'land', 'trap', 'water']) {
  const bytes = readFileSync(join(ROOT, 'public', 'assets', 'terrain', `${name}.png`));
  assets[`assets/terrain/${name}.png`] = dataUri(bytes, 'image/png');
}
for (const side of ['gold', 'scarlet']) {
  for (const animal of ANIMALS) {
    const bytes = readFileSync(join(ROOT, 'public', 'assets', 'pieces', side, `${animal}.png`));
    assets[`assets/pieces/${side}/${animal}.png`] = dataUri(bytes, 'image/png');
  }
}

const font = readFileSync(join(ROOT, 'public', 'assets', 'fonts', 'Cinzel.ttf'));
let css = readFileSync(join(ROOT, 'public', 'css', 'style.css'), 'utf8');
css = css.replace(
  'url("../assets/fonts/Cinzel.ttf")',
  `url("${dataUri(font, 'font/ttf')}")`,
);

let html = readFileSync(join(ROOT, 'public', 'index.html'), 'utf8');
const lion = readFileSync(join(ROOT, 'public', 'assets', 'pieces', 'gold', 'lion.png'));
html = html.replace(
  'href="assets/pieces/gold/lion.png"',
  `href="${dataUri(lion, 'image/png')}"`,
);
html = html.replace(
  '<link rel="stylesheet" href="css/style.css">',
  `<style>\n${css}\n</style>`,
);

const boot = `
const ENGINE_B64 = ${JSON.stringify(b64(engineBytes))};
const ARENA_B64 = ${JSON.stringify(b64(arenaBytes))};
const WORKER_JS = ${JSON.stringify(workerJs)};
function b64ToU8(s) {
  const bin = atob(s);
  const out = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
  return out;
}
globalThis.JUNGLE_ENGINE_BYTES = b64ToU8(ENGINE_B64);
globalThis.JUNGLE_ARENA_BYTES = b64ToU8(ARENA_B64);
globalThis.JUNGLE_ARENA_SHA256 = ${JSON.stringify(arenaHash)};
globalThis.JUNGLE_ASSETS = ${JSON.stringify(assets)};
const header = 'globalThis.JUNGLE_ENGINE_BYTES = Uint8Array.from(atob('
  + JSON.stringify(ENGINE_B64) + '), c => c.charCodeAt(0));\\n';
globalThis.JUNGLE_WORKER_URL = URL.createObjectURL(new Blob(
  [header, WORKER_JS],
  { type: 'text/javascript' },
));
`;

const moduleSrc = `${boot}\n${appJs}`;
html = html.replace(
  '<script type="module" src="js/main.js"></script>',
  `<script type="module">\n${moduleSrc}\n</script>`,
);

mkdirSync(join(ROOT, 'release'), { recursive: true });
const out = join(ROOT, 'release', 'index.html');
writeFileSync(out, html);
const htmlHash = createHash('sha256').update(html).digest('hex');
const shaPath = join(ROOT, 'release', 'SHA256.txt');
const previous = readFileSync(shaPath, 'utf8').split(/\r?\n/).filter((line) => line && !line.includes('index.html'));
writeFileSync(shaPath, `${previous.join('\n')}\n${htmlHash}  index.html\n`);
console.log(`release/index.html ${html.length} bytes`);
console.log(`arena ${arenaHash}`);
console.log(`html  ${htmlHash}`);
