import { afterEach, describe, expect, it } from 'vitest';

import { useAuthStore } from './authStore';

describe('authStore', () => {
  afterEach(() => {
    useAuthStore.getState().logout();
    sessionStorage.clear();
    localStorage.clear();
  });

  it('uses sessionStorage instead of localStorage', () => {
    useAuthStore.getState().setUser({
      id: 'test-id',
      email: 'test@example.com',
      name: 'Test User',
      created_at: '2024-01-01T00:00:00Z',
      updated_at: '2024-01-01T00:00:00Z',
    });

    expect(sessionStorage.getItem('auth-storage')).not.toBeNull();
    expect(localStorage.getItem('auth-storage')).toBeNull();
  });

  it('persists user data to sessionStorage', () => {
    useAuthStore.getState().setUser({
      id: 'test-id',
      email: 'test@example.com',
      name: 'Test User',
      created_at: '2024-01-01T00:00:00Z',
      updated_at: '2024-01-01T00:00:00Z',
    });

    const stored = JSON.parse(sessionStorage.getItem('auth-storage') ?? '{}');
    expect(stored.state.user.email).toBe('test@example.com');
    expect(stored.state.isAuthenticated).toBe(true);
  });
});
