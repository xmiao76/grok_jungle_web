import { test, expect } from '@playwright/test';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';

test.use({ acceptDownloads: true });

test('file:// build plays and the download matches the release hash', async ({ page }) => {
  const html = join(process.cwd(), 'release', 'index.html');
  await page.goto(pathToFileURL(html).href);
  await expect(page.locator('#menu-overlay')).toBeVisible({ timeout: 150_000 });
  await expect(page.locator('#arena-meta')).toContainText('SHA-256');
  const thread = await page.evaluate(() => window.__jungle.thread);
  expect(['worker', 'main']).toContain(thread);

  await page.click('.diff-btn[data-d="easy"]');
  await page.click('#btn-start-hva');
  await expect(page.locator('#status-text')).toContainText('your turn');
  const pos = await page.evaluate((sq) => window.__jungle.squareToPixel(sq), 'g3');
  await page.locator('#board').click({ position: { x: pos.x, y: pos.y } });
  const dst = await page.evaluate((sq) => window.__jungle.squareToPixel(sq), 'g4');
  await page.locator('#board').click({ position: { x: dst.x, y: dst.y } });
  await expect.poll(
    () => page.evaluate(() => window.__jungle.state?.ply),
    { timeout: 30_000 },
  ).toBeGreaterThanOrEqual(2);

  await page.click('#btn-menu');
  await page.locator('#arena-panel > summary').click();
  const [download] = await Promise.all([
    page.waitForEvent('download'),
    page.locator('#btn-download-wasm').click(),
  ]);
  const saved = await download.path();
  const hash = createHash('sha256').update(readFileSync(saved)).digest('hex');
  const expected = readFileSync(join(process.cwd(), 'release', 'SHA256.txt'), 'utf8')
    .split(/\r?\n/)
    .find((line) => line.includes('grok-4-7-grokbuild-2.0.0.wasm'))
    .split(/\s+/)[0];
  expect(hash).toBe(expected);
  expect(download.suggestedFilename()).toBe('grok-4-7-grokbuild-2.0.0.wasm');
  console.log(`file:// engine thread: ${thread}`);
});
