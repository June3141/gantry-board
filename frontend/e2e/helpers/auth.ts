import type { APIRequestContext } from '@playwright/test';

const API_BASE = process.env.E2E_API_URL ?? 'http://localhost:3000';

export interface TestUser {
  id: string;
  email: string;
  name: string;
  cookie: string;
}

let userCounter = 0;

export async function createTestUser(
  request: APIRequestContext,
  overrides?: { email?: string; name?: string },
): Promise<TestUser> {
  userCounter++;
  const email = overrides?.email ?? `e2e-user-${userCounter}-${Date.now()}@test.local`;
  const name = overrides?.name ?? `E2E User ${userCounter}`;
  const password = 'Tr0ub4dor&3-e2e-test';

  const response = await request.post(`${API_BASE}/api/auth/register`, {
    data: { email, name, password },
    headers: { 'x-requested-with': 'XMLHttpRequest' },
  });

  if (!response.ok()) {
    throw new Error(`createTestUser failed: ${response.status()} ${await response.text()}`);
  }

  const setCookie = response.headers()['set-cookie'] ?? '';
  const cookie = setCookie.split(';')[0];
  const body = await response.json();

  return {
    id: body.user.id,
    email,
    name,
    cookie,
  };
}
