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
    await page.context().addCookies([
      {
        name: testUser.cookie.split('=')[0],
        value: testUser.cookie.split('=')[1],
        domain,
        path: '/',
      },
    ]);
    // Populate sessionStorage with Zustand auth state so ProtectedRoute
    // doesn't redirect to /login before the app can mount.
    await page.goto(base);
    await page.evaluate(
      (user) => {
        sessionStorage.setItem(
          'auth-storage',
          JSON.stringify({
            state: { user, isAuthenticated: true },
            version: 0,
          }),
        );
      },
      { id: testUser.id, email: testUser.email, name: testUser.name },
    );
    await use(page);
  },
});

export { expect } from '@playwright/test';
