// Copy the desktop *rule oracle* (stdlib only) into public/py/jungle/.
// The playing engine is Rust/wasm in engine/; this Python tree is tests-only.
// Usage: node tools/sync-engine.mjs
// Override the desktop root with GROK_JUNGLE if it is not a sibling checkout.

import { mkdir, copyFile, access } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const REPO = join(fileURLToPath(new URL('..', import.meta.url)));
const DESKTOP = process.env.GROK_JUNGLE || join(REPO, '..', 'grok_jungle');
const SRC = join(DESKTOP, 'src', 'jungle');
const DST = join(REPO, 'public', 'py', 'jungle');

const FILES = [
  '__init__.py',
  'engine/__init__.py',
  'engine/types.py',
  'engine/board.py',
  'engine/position.py',
  'engine/apply.py',
  'engine/movegen.py',
  'engine/game.py',
  'engine/zobrist.py',
];

async function main() {
  try {
    await access(join(SRC, 'engine', 'types.py'));
  } catch {
    throw new Error(
      `desktop engine not found at ${SRC}. Set GROK_JUNGLE to the grok_jungle repo root.`,
    );
  }
  for (const rel of FILES) {
    const from = join(SRC, rel);
    const to = join(DST, rel);
    await mkdir(dirname(to), { recursive: true });
    await copyFile(from, to);
    console.log(`copied ${rel}`);
  }
  console.log(`synced ${FILES.length} files from ${SRC}`);
}

main().catch((err) => {
  console.error(err.message || err);
  process.exit(1);
});
