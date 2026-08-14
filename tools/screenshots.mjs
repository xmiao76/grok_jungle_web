// Capture menu + in-game screenshots for visual review.
import { chromium } from '@playwright/test';
import { mkdir } from 'node:fs/promises';
import { join } from 'node:path';

const OUT = join(process.cwd(), 'test-results', 'shots');
await mkdir(OUT, { recursive: true });

const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1400, height: 900 } });
await page.goto('http://localhost:8788/', { waitUntil: 'domcontentloaded' });
await page.locator('#menu-overlay').waitFor({ state: 'visible', timeout: 150000 });
await page.waitForTimeout(400);
await page.screenshot({ path: join(OUT, 'menu-desktop.png'), fullPage: true });

await page.click('.diff-btn[data-d="easy"]');
await page.click('#btn-start-hva');
await page.locator('#game-area').waitFor({ state: 'visible' });
await page.waitForTimeout(500);
await page.screenshot({ path: join(OUT, 'game-desktop.png') });

const pos = await page.evaluate(() => window.__jungle.squareToPixel('g3'));
await page.locator('#board').click({ position: pos });
await page.waitForTimeout(200);
await page.screenshot({ path: join(OUT, 'game-selected.png') });

const mobile = await browser.newPage({ viewport: { width: 390, height: 844 } });
await mobile.goto('http://localhost:8788/', { waitUntil: 'domcontentloaded' });
await mobile.locator('#menu-overlay').waitFor({ state: 'visible', timeout: 150000 });
await mobile.waitForTimeout(300);
await mobile.screenshot({ path: join(OUT, 'menu-mobile.png'), fullPage: true });

await browser.close();
console.log(`wrote screenshots to ${OUT}`);
