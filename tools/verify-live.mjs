// Post-deploy check: load the public URL, wait for Pyodide, play Easy AI-vs-AI.
import { chromium } from '@playwright/test';

const url = process.argv[2] || 'https://grok-jungle.pages.dev/';
const browser = await chromium.launch();
const page = await browser.newPage();
page.on('pageerror', (err) => console.log('PAGEERROR', err.message));
const res = await page.goto(url, { waitUntil: 'domcontentloaded', timeout: 60000 });
console.log('GET', url, res && res.status());
await page.locator('#menu-overlay').waitFor({ state: 'visible', timeout: 150000 });
const info = await page.locator('#engine-info').innerText();
const credits = await page.locator('.credits').innerText();
console.log('engine:', info);
console.log('credits:', credits);
const result = await page.evaluate(async () => {
  const client = window.__jungle.app.client;
  let { state } = await client.newGame({
    mode: 'ai_ai', humanFirst: true, difficulty: 'easy',
  });
  let plies = 0;
  while (!state.terminal && plies < 200) {
    ({ state } = await client.aiMove(250));
    plies++;
  }
  return { terminal: state.terminal, result: state.result, outcome: state.outcome, plies, fen: state.fen };
});
console.log('game', result);
if (!result.terminal) {
  console.error('FAIL: game did not terminate');
  process.exit(1);
}
await browser.close();
console.log('live site verified');
