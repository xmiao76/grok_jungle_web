import { test, expect } from '@playwright/test';

async function waitForMenu(page) {
  await page.goto('/');
  await expect(page.locator('#menu-overlay')).toBeVisible({ timeout: 150_000 });
}

async function clickSquare(page, name) {
  const pos = await page.evaluate((sq) => window.__jungle.squareToPixel(sq), name);
  expect(pos).toBeTruthy();
  await page.locator('#board').click({ position: { x: pos.x, y: pos.y } });
}

test('loads and initializes the Rust/wasm engine', async ({ page }) => {
  await waitForMenu(page);
  await expect(page.locator('#engine-info')).toContainText('Rust');
  await expect(page.locator('#identity-line')).toContainText('Grok-4.7');
  await expect(page.locator('#identity-line')).toContainText('GrokBuild');
  await expect(page.locator('#identity-line')).toContainText('official');
  await expect(page.locator('#arena-meta')).toContainText('desktop-jungle-v2');
  await expect(page.locator('#arena-meta')).toContainText('SHA-256');
  await expect(page.locator('#error-overlay')).toBeHidden();
});

test('human move via clicks gets an AI reply', async ({ page }) => {
  await waitForMenu(page);
  await page.click('.diff-btn[data-d="easy"]');
  await page.click('#btn-start-hva');
  await expect(page.locator('#game-area')).toBeVisible();
  await expect(page.locator('#status-text')).toContainText('your turn');

  await clickSquare(page, 'g3');
  await clickSquare(page, 'g4');

  await expect.poll(
    () => page.evaluate(() => window.__jungle.state?.ply),
    { timeout: 30_000 },
  ).toBeGreaterThanOrEqual(2);
  await expect(page.locator('#history-list li')).toHaveCount(
    await page.evaluate(() => window.__jungle.state.ply),
  );
  await expect(page.locator('#error-overlay')).toBeHidden();
});

test('undo returns the game to the human turn', async ({ page }) => {
  await waitForMenu(page);
  await page.click('.diff-btn[data-d="easy"]');
  await page.click('#btn-start-hva');
  await clickSquare(page, 'g3');
  await clickSquare(page, 'g4');
  await expect.poll(
    () => page.evaluate(() => window.__jungle.state?.ply),
    { timeout: 30_000 },
  ).toBeGreaterThanOrEqual(2);

  await expect(page.locator('#btn-undo')).toBeEnabled();
  await page.click('#btn-undo');
  await expect.poll(
    () => page.evaluate(() => window.__jungle.state?.ply),
  ).toBe(0);
  await expect(page.locator('#status-text')).toContainText('your turn');
});

test('flip is view-only and clicks still map', async ({ page }) => {
  await waitForMenu(page);
  await page.click('.diff-btn[data-d="easy"]');
  await page.click('#btn-start-hva');
  const fenBefore = await page.evaluate(() => window.__jungle.state.fen);
  await page.click('#btn-flip');
  const fenAfter = await page.evaluate(() => window.__jungle.state.fen);
  expect(fenAfter).toBe(fenBefore);
  await clickSquare(page, 'g3');
  await clickSquare(page, 'g4');
  await expect.poll(
    () => page.evaluate(() => window.__jungle.state?.ply),
    { timeout: 30_000 },
  ).toBeGreaterThanOrEqual(1);
});

test('AI first undo re-kicks Blue', async ({ page }) => {
  await waitForMenu(page);
  await page.click('.diff-btn[data-d="easy"]');
  await page.locator('.switch').click();
  await page.click('#btn-start-hva');
  await expect(page.locator('#game-area')).toBeVisible();
  await expect.poll(
    () => page.evaluate(() => window.__jungle.state?.humanFirst),
  ).toBe(false);
  await expect.poll(
    () => page.evaluate(() => window.__jungle.state?.ply),
    { timeout: 30_000 },
  ).toBeGreaterThanOrEqual(1);
  await expect.poll(
    () => page.evaluate(() => window.__jungle.state?.turn),
  ).toBe('red');
  await page.click('#btn-undo');
  await expect.poll(
    () => page.evaluate(() => window.__jungle.state?.ply),
    { timeout: 30_000 },
  ).toBeGreaterThanOrEqual(1);
  await expect(page.locator('#error-overlay')).toBeHidden();
});

test('full AI-vs-AI game reaches a valid terminal state in-browser', async ({ page }) => {
  await waitForMenu(page);
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
    return { terminal: state.terminal, result: state.result, outcome: state.outcome, plies };
  });
  expect(result.terminal).toBe(true);
  expect(['red_wins', 'blue_wins', 'draw']).toContain(result.result);
  expect(['den', 'elimination', 'immobilization', 'repetition', 'no-capture']).toContain(result.outcome);
  expect(result.plies).toBeGreaterThan(4);
});

test('undo during a Hard think drops the in-flight reply', async ({ page }) => {
  await waitForMenu(page);
  await page.click('.diff-btn[data-d="hard"]');
  await page.click('#btn-start-hva');
  await clickSquare(page, 'g3');
  await clickSquare(page, 'g4');
  await expect(page.locator('#btn-undo')).toBeEnabled();
  await page.click('#btn-undo');
  await expect.poll(
    () => page.evaluate(() => window.__jungle.state?.ply),
  ).toBe(0);
  await page.waitForTimeout(1200);
  expect(await page.evaluate(() => window.__jungle.state?.ply)).toBe(0);
  await expect(page.locator('#status-text')).toContainText('your turn');
});

test('a Hard reply reaches the board within one second', async ({ page }) => {
  await waitForMenu(page);
  await page.click('.diff-btn[data-d="hard"]');
  await page.click('#btn-start-hva');
  const started = Date.now();
  await clickSquare(page, 'g3');
  await clickSquare(page, 'g4');
  await expect.poll(
    () => page.evaluate(() => window.__jungle.state?.ply),
    { timeout: 5_000 },
  ).toBe(2);
  expect(Date.now() - started).toBeLessThan(1000);
});
