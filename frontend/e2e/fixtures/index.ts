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
    async (_deps, use) => {
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
    const domain = new URL(baseURL ?? 'http://localhost:5173').hostname;
    await page.context().addCookies([
      {
        name: testUser.cookie.split('=')[0],
        value: testUser.cookie.split('=')[1],
        domain,
        path: '/',
      },
    ]);
    await use(page);
  },
});

export { expect } from '@playwright/test';
