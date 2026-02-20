import { test as base, type Page, request as playwrightRequest } from '@playwright/test';
import { ApiHelper } from '../helpers/api';
import { createTestUser, type TestUser } from '../helpers/auth';

const API_BASE = process.env.E2E_API_URL ?? 'http://localhost:3000';

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

  authenticatedPage: async ({ page, testUser }, use) => {
    // Login via page.request so the session cookie is stored in the browser context.
    // This is more reliable than manually parsing Set-Cookie and calling addCookies.
    const loginResponse = await page.request.post(`${API_BASE}/api/auth/login`, {
      data: { email: testUser.email, password: testUser.password },
      headers: { 'x-requested-with': 'XMLHttpRequest' },
    });
    if (!loginResponse.ok()) {
      throw new Error(
        `authenticatedPage login failed: ${loginResponse.status()} ${await loginResponse.text()}`,
      );
    }

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
