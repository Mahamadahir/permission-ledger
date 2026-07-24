import { expect, test, type Page } from '@playwright/test';

function uniqueEmail() {
  return `e2e-${Date.now()}-${Math.floor(Math.random() * 1e6)}@example.com`;
}

async function register(page: Page, email: string) {
  await page.goto('/');
  // Wait for hydration (loadSession fires fetch on mount) so the tab click isn't
  // lost against the server-rendered, not-yet-interactive markup.
  await page.waitForLoadState('networkidle');
  await expect(async () => {
    await page.getByRole('button', { name: 'Register' }).click();
    await expect(page.getByRole('button', { name: 'Create account' })).toBeVisible({ timeout: 1000 });
  }).toPass();
  await page.getByLabel('Email').fill(email);
  await page.getByLabel('Password').fill('correcthorse1');
  await authSubmit(page);
  await expect(page.getByRole('button', { name: 'Log out' })).toBeVisible();
}

// The auth panel submit button is the only primary button in the panel; its
// label changes between "Login" and "Create account" with the active tab.
function authSubmit(page: Page) {
  return page.locator('.auth-panel button.primary').click();
}

async function addRecord(page: Page, service: string) {
  await page.getByRole('button', { name: 'Add record' }).first().click();
  await page.getByLabel('Service name').fill(service);
  await page.getByLabel('Website URL').fill('https://acme.example');
  await page.getByLabel('Consent type').fill('analytics cookies');
  await page.getByRole('button', { name: 'Save record' }).click();
  await expect(page.getByText('Record saved')).toBeVisible();
}

test('register, then reload keeps the session', async ({ page }) => {
  const email = uniqueEmail();
  await register(page, email);
  // The email appears in the user chip (both the name and the sub-label).
  await expect(page.getByText(email).first()).toBeVisible();

  await page.reload();
  await page.waitForLoadState('networkidle');
  // Session cookie round-trips, so the dashboard shows without re-login.
  await expect(page.getByRole('button', { name: 'Log out' })).toBeVisible();
});

test('add a record through the modal and see it in the table', async ({ page }) => {
  await register(page, uniqueEmail());
  await addRecord(page, 'Acme Analytics');

  const row = page.getByRole('row', { name: /Acme Analytics/ });
  await expect(row).toBeVisible();
  await expect(row.getByText('active')).toBeVisible();
});

test('revoke a record flips its status', async ({ page }) => {
  await register(page, uniqueEmail());
  await addRecord(page, 'Acme Analytics');

  await page.getByRole('row', { name: /Acme Analytics/ }).getByTitle('Revoke').click();
  await expect(page.getByRole('row', { name: /Acme Analytics/ }).getByText('revoked')).toBeVisible();
});

test('CSV export responds with the header row', async ({ page, request }) => {
  await register(page, uniqueEmail());
  const response = await page.request.get('http://localhost:3000/api/export.csv');
  expect(response.status()).toBe(200);
  expect(await response.text()).toContain('id,service_name,website_url,category');
});

test('pair an extension device, then revoke it', async ({ page }) => {
  await register(page, uniqueEmail());
  await page.getByLabel('Device name').fill('Work laptop');
  await page.getByRole('button', { name: 'Pair extension' }).click();

  await expect(page.locator('.token-box code')).toBeVisible();
  await expect(page.locator('.device-list')).toContainText('Work laptop');

  await page.getByRole('button', { name: 'Revoke' }).click();
  await expect(page.getByText('No paired devices.')).toBeVisible();
});

test('log out returns to the auth screen', async ({ page }) => {
  await register(page, uniqueEmail());
  await page.getByRole('button', { name: 'Log out' }).click();
  await expect(page.getByRole('heading', { name: /Privacy decisions/ })).toBeVisible();
});

test('wrong password shows an error', async ({ page }) => {
  const email = uniqueEmail();
  await register(page, email);
  await page.getByRole('button', { name: 'Log out' }).click();

  // Logout leaves the panel on whichever tab register used; switch to Login.
  await page.getByRole('button', { name: 'Login' }).first().click();
  await page.getByLabel('Email').fill(email);
  await page.getByLabel('Password').fill('wrongpassword1');
  await authSubmit(page);
  await expect(page.getByText('authentication required')).toBeVisible();
});
