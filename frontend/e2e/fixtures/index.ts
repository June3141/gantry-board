import { test as base, type Page, request as playwrightRequest } from '@playwright/test';
import { ApiHelper } from '../helpers/api';
import { createTestUser, type TestUser } from '../helpers/auth';

type TestFixtures = {
  apiHelper: ApiHelper;
  authenticatedPage: Page;
};

type WorkerFixtures = {
  testUser: TestUser;
};

export const test = base.extend<TestFixtures, WorkerFixtures>({
  testUser: [
    // biome-ignore lint/correctness/noEmptyPattern: Playwright requires object destructuring for worker fixtures
    async ({}, use) => {
      const apiContext = await playwrightRequest.newContext();
      try {
        const user = await createTestUser(apiContext);
        await use(user);
      } finally {
        await apiContext.dispose();
      }
    },
    { scope: 'worker' },
  ],

  apiHelper: async ({ request }, use) => {
    await use(new ApiHelper(request));
  },

  authenticatedPage: async ({ page, testUser, baseURL }, use) => {
    const base = baseURL ?? 'http://localhost:5173';
    const domain = new URL(base).hostname;
    // Set the register session cookie on the browser context.
    // We intentionally do NOT call login (which rotates sessions) because
    // parallel tests within the same worker share the testUser and login
    // would invalidate other tests' sessions.
    await page.context().addCookies([
      {
        name: testUser.cookie.split('=')[0],
        value: testUser.cookie.split('=')[1],
        domain,
        path: '/',
        sameSite: 'Lax',
      },
    ]);
    // Inject Zustand auth state into sessionStorage before any page script runs.
    // This prevents ProtectedRoute from redirecting to /login on the first navigation.
    const authState = JSON.stringify({
      state: {
        user: { id: testUser.id, email: testUser.email, name: testUser.name },
        isAuthenticated: true,
      },
      version: 0,
    });
    await page.addInitScript((state) => {
      sessionStorage.setItem('auth-storage', state);
    }, authState);
    await use(page);
  },
});

export { expect } from '@playwright/test';
