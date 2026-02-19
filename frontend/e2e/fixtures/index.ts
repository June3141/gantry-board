import { test as base, type Page } from '@playwright/test';
import { ApiHelper } from '../helpers/api';
import { type TestUser, createTestUser } from '../helpers/auth';

type Fixtures = {
  apiHelper: ApiHelper;
  testUser: TestUser;
  authenticatedPage: Page;
};

export const test = base.extend<Fixtures>({
  apiHelper: async ({ request }, use) => {
    await use(new ApiHelper(request));
  },

  testUser: async ({ request }, use) => {
    const user = await createTestUser(request);
    await use(user);
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
