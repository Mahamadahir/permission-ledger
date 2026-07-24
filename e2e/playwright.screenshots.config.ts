import { defineConfig, devices } from '@playwright/test';

// Separate from the test suite: this writes README images rather than asserting
// behaviour, and needs a seeded demo account to look like anything.
export default defineConfig({
  testDir: '.',
  testMatch: 'screenshots.spec.ts',
  timeout: 60_000,
  expect: { timeout: 15_000 },
  reporter: 'line',
  use: {
    baseURL: process.env.WEB_URL ?? 'http://localhost:5173'
  },
  projects: [{ name: 'chromium', use: { ...devices['Desktop Chrome'] } }]
});
