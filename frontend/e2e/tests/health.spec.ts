import { test, expect } from '../fixtures';

test.describe('Health Check', () => {
  test('backend health endpoint returns ok', async ({ apiHelper }) => {
    const response = await apiHelper.healthCheck();
    expect(response.ok()).toBeTruthy();
  });
});
