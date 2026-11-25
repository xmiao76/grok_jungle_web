// Build the Arena component and the fixture opponent.
// Usage: node tools/build-component.mjs

import { spawnSync } from 'node:child_process';
import { copyFileSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');
const RELEASE_NAME = 'grok-4-7-grokbuild-2.0.0.wasm';
const PREAMBLE = Buffer.from([0x00, 0x61, 0x73, 0x6d, 0x0d, 0x00, 0x01, 0x00]);

function run(cmd, args) {
  const r = spawnSync(cmd, args, { cwd: ROOT, stdio: 'inherit', shell: true });
  if (r.status !== 0) process.exit(r.status ?? 1);
}

function wrap(core, out) {
  mkdirSync(join(ROOT, 'target'), { recursive: true });
  const embedded = join(ROOT, 'target', 'component-embedded.wasm');
  run('wasm-tools', ['component', 'embed', '--world', 'competitor', 'wit', core, '-o', embedded]);
  run('wasm-tools', ['component', 'new', embedded, '-o', out]);
}

run('cargo', [
  'build', '--manifest-path', 'engine/Cargo.toml',
  '--target', 'wasm32-unknown-unknown', '--release',
  '--no-default-features', '--features', 'arena', '--lib',
]);
run('cargo', [
  'build', '--manifest-path', 'fixture/Cargo.toml',
  '--target', 'wasm32-unknown-unknown', '--release',
]);

mkdirSync(join(ROOT, 'release'), { recursive: true });
mkdirSync(join(ROOT, 'public', 'wasm'), { recursive: true });

const releasePath = join(ROOT, 'release', RELEASE_NAME);
wrap(
  join('engine', 'target', 'wasm32-unknown-unknown', 'release', 'jungle_engine.wasm'),
  releasePath,
);
wrap(
  join('fixture', 'target', 'wasm32-unknown-unknown', 'release', 'jungle_fixture.wasm'),
  join(ROOT, 'public', 'wasm', 'fixture.wasm'),
);
copyFileSync(releasePath, join(ROOT, 'public', 'wasm', RELEASE_NAME));

const bytes = readFileSync(releasePath);
if (!bytes.subarray(0, 8).equals(PREAMBLE)) {
  console.error('component preamble mismatch');
  process.exit(1);
}
if (bytes.length > 24 * 1024 * 1024) {
  console.error('component exceeds 24 MiB');
  process.exit(1);
}
const hash = createHash('sha256').update(bytes).digest('hex');
const fixtureHash = createHash('sha256')
  .update(readFileSync(join(ROOT, 'public', 'wasm', 'fixture.wasm')))
  .digest('hex');
writeFileSync(
  join(ROOT, 'release', 'SHA256.txt'),
  `${hash}  ${RELEASE_NAME}\n${fixtureHash}  fixture.wasm\n`,
);
console.log(`${RELEASE_NAME} ${bytes.length} bytes`);
console.log(hash);
