import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: 'tests/e2e',
  timeout: 180_000,
  retries: 1,
  use: {
    baseURL: 'http://localhost:8799',
    trace: 'retain-on-failure',
  },
  webServer: {
    command: 'node tools/serve.mjs 8799',
    url: 'http://localhost:8799',
    reuseExistingServer: false,
  },
});
