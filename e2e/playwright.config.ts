import { defineConfig, devices } from '@playwright/test';

// The stack (postgres + backend + web) is expected to be running already, via
// `docker compose up` locally and in CI. The web app is served on :5173 and
// talks to the backend on :3000 with credentialed CORS, which is how the real
// cookie/CSRF flow gets exercised.
export default defineConfig({
  testDir: '.',
  timeout: 30_000,
  expect: { timeout: 10_000 },
  fullyParallel: false,
  retries: process.env.CI ? 1 : 0,
  reporter: process.env.CI ? 'list' : 'line',
  use: {
    baseURL: process.env.WEB_URL ?? 'http://localhost:5173',
    trace: 'on-first-retry'
  },
  projects: [{ name: 'chromium', use: { ...devices['Desktop Chrome'] } }]
});
