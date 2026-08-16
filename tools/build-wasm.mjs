// Build the Rust engine to public/wasm for the static site.
// Requires: rustup target add wasm32-unknown-unknown, wasm-pack.
// Usage: node tools/build-wasm.mjs

import { spawnSync } from 'node:child_process';
import { rmSync, existsSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');
const ENGINE = join(ROOT, 'engine');
const OUT = join(ROOT, 'public', 'wasm');

const r = spawnSync(
  'wasm-pack',
  ['build', '--target', 'web', '--out-dir', OUT, '--release', '--features', 'wasm'],
  { cwd: ENGINE, stdio: 'inherit', shell: true },
);
if (r.status !== 0) {
  process.exit(r.status ?? 1);
}

for (const extra of ['.gitignore', 'package.json']) {
  const p = join(OUT, extra);
  if (existsSync(p)) rmSync(p);
}
console.log('wrote', OUT);
