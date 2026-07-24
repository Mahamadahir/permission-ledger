import { expect, test, type Page } from '@playwright/test';

// Regenerates the README screenshots from a seeded demo account. Run with the
// dedicated config so it never runs as part of the normal suite:
//   node ../scripts/seed.mjs <baseUrl>
//   npm run screenshots
const email = process.env.SEED_EMAIL ?? 'demo@permissionledger.test';
const password = process.env.SEED_PASSWORD ?? 'demopassword123';
const outDir = '../docs/images';

async function signIn(page: Page) {
  await page.goto('/');
  await page.waitForLoadState('networkidle');
  await page.getByRole('button', { name: 'Login' }).first().click();
  await page.getByLabel('Email').fill(email);
  await page.getByLabel('Password').fill(password);
  await page.locator('.auth-panel button.primary').click();
  await expect(page.getByRole('button', { name: 'Log out' })).toBeVisible();
}

async function shoot(page: Page, name: string) {
  await page.waitForLoadState('networkidle');
  await page.screenshot({ path: `${outDir}/${name}.png`, fullPage: true });
}

test('capture the dashboard screenshots', async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });

  await page.goto('/');
  await page.waitForLoadState('networkidle');
  await shoot(page, 'login');

  await signIn(page);
  await shoot(page, 'dashboard');

  for (const [label, name] of [
    ['Records', 'records'],
    ['Extension', 'extension'],
    ['Exports', 'exports'],
    ['Settings', 'settings']
  ]) {
    await page.getByRole('link', { name: label, exact: true }).click();
    await shoot(page, name);
  }
});
