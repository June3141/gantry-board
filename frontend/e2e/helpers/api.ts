import type { APIRequestContext, APIResponse } from '@playwright/test';

const API_BASE = process.env.E2E_API_URL ?? 'http://localhost:3000';

async function assertOk(response: APIResponse, context: string): Promise<APIResponse> {
  if (!response.ok()) {
    throw new Error(`${context} failed: ${response.status()} ${await response.text()}`);
  }
  return response;
}

export class ApiHelper {
  constructor(private request: APIRequestContext) {}

  async registerUser(email: string, name: string, password: string) {
    const response = await this.request.post(`${API_BASE}/api/auth/register`, {
      data: { email, name, password },
      headers: { 'x-requested-with': 'XMLHttpRequest' },
    });
    return (await assertOk(response, 'registerUser')).json();
  }

  async createProject(cookie: string, name: string) {
    const response = await this.request.post(`${API_BASE}/api/projects`, {
      data: { name },
      headers: {
        cookie,
        'x-requested-with': 'XMLHttpRequest',
      },
    });
    return (await assertOk(response, 'createProject')).json();
  }

  async healthCheck() {
    const response = await this.request.get(`${API_BASE}/health`);
    return response;
  }
}
