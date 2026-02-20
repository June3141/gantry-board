import { expect, test } from '../fixtures';

test.describe('Authentication Flow', () => {
  test('registers a new user and redirects to board', async ({ page }) => {
    const email = `e2e-reg-${Date.now()}@test.local`;

    await page.goto('/register');
    await page.getByLabel('Name').fill('E2E Test User');
    await page.getByLabel('Email').fill(email);
    await page.getByLabel('Password').fill('Tr0ub4dor&3-e2e-test');
    await page.getByRole('button', { name: /create account/i }).click();

    // Should redirect to board
    await expect(page).toHaveURL('/');
    await expect(page.getByText('Gantry Board')).toBeVisible();
  });

  test('logs in with valid credentials', async ({ page, apiHelper }) => {
    const email = `e2e-login-${Date.now()}@test.local`;
    const password = 'Tr0ub4dor&3-e2e-test';

    // Register first via API (use apiHelper to target backend directly)
    await apiHelper.registerUser(email, 'Login Test', password);

    await page.goto('/login');
    await page.getByLabel('Email').fill(email);
    await page.getByLabel('Password').fill(password);
    await page.getByRole('button', { name: /sign in/i }).click();

    await expect(page).toHaveURL('/');
    await expect(page.getByText('Gantry Board')).toBeVisible();
  });

  test('shows error on invalid credentials', async ({ page }) => {
    await page.goto('/login');
    await page.getByLabel('Email').fill('nonexistent@test.local');
    await page.getByLabel('Password').fill('wrongpassword');
    await page.getByRole('button', { name: /sign in/i }).click();

    await expect(page.getByText(/invalid|incorrect|unauthorized/i)).toBeVisible();
  });

  test('redirects unauthenticated user to login', async ({ page }) => {
    await page.goto('/');
    await expect(page).toHaveURL(/\/login/);
  });

  test('logs out successfully', async ({ authenticatedPage: page }) => {
    await page.goto('/');
    await page.getByRole('button', { name: /logout/i }).click();

    await expect(page).toHaveURL(/\/login/);
  });
});
