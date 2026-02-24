import { HttpResponse, http } from 'msw';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { server } from '../test/mocks/server';
import { useAuthStore } from './authStore';

describe('authStore', () => {
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

describe('authStore rehydration', () => {
  beforeEach(() => {
    useAuthStore.setState(useAuthStore.getInitialState());
    sessionStorage.clear();
  });

  afterEach(() => {
    server.resetHandlers();
    sessionStorage.clear();
  });

  it('should call GET /api/auth/me on rehydration when isAuthenticated is true', async () => {
    const meHandler = vi.fn();

    server.use(
      http.get('*/api/auth/me', () => {
        meHandler();
        return HttpResponse.json({
          id: 'user-1',
          name: 'Test User',
          email: 'test@test.com',
          created_at: '2024-01-01T00:00:00Z',
          updated_at: '2024-01-01T00:00:00Z',
        });
      }),
    );

    // Simulate stored auth state
    sessionStorage.setItem(
      'auth-storage',
      JSON.stringify({
        state: { user: null, isAuthenticated: true },
        version: 0,
      }),
    );

    // Trigger rehydration by re-creating the store
    useAuthStore.persist.rehydrate();

    // Wait for the async revalidation to complete
    await vi.waitFor(() => {
      expect(meHandler).toHaveBeenCalled();
    });
  });

  it('should clear auth state when server says session is invalid', async () => {
    server.use(
      http.get('*/api/auth/me', () => {
        return HttpResponse.json({ error: 'Unauthorized' }, { status: 401 });
      }),
    );

    // Simulate stored auth state with isAuthenticated = true
    sessionStorage.setItem(
      'auth-storage',
      JSON.stringify({
        state: { user: null, isAuthenticated: true },
        version: 0,
      }),
    );

    // Trigger rehydration
    useAuthStore.persist.rehydrate();

    // Wait for the async revalidation to clear the state
    await vi.waitFor(() => {
      const state = useAuthStore.getState();
      expect(state.isAuthenticated).toBe(false);
      expect(state.user).toBeNull();
    });
  });

  it('should update user data from server on successful revalidation', async () => {
    const serverUser = {
      id: 'user-1',
      name: 'Server User',
      email: 'server@test.com',
      created_at: '2024-01-01T00:00:00Z',
      updated_at: '2024-01-01T00:00:00Z',
    };

    server.use(
      http.get('*/api/auth/me', () => {
        return HttpResponse.json(serverUser);
      }),
    );

    // Simulate stored auth state with stale user data
    sessionStorage.setItem(
      'auth-storage',
      JSON.stringify({
        state: {
          user: { id: 'user-1', name: 'Old Name', email: 'old@test.com' },
          isAuthenticated: true,
        },
        version: 0,
      }),
    );

    useAuthStore.persist.rehydrate();

    await vi.waitFor(() => {
      const state = useAuthStore.getState();
      expect(state.isAuthenticated).toBe(true);
      expect(state.user?.name).toBe('Server User');
      expect(state.user?.email).toBe('server@test.com');
    });
  });

  it('should not call /api/auth/me when isAuthenticated is false', async () => {
    const meHandler = vi.fn();

    server.use(
      http.get('*/api/auth/me', () => {
        meHandler();
        return HttpResponse.json({
          id: 'user-1',
          name: 'Test User',
          email: 'test@test.com',
          created_at: '2024-01-01T00:00:00Z',
          updated_at: '2024-01-01T00:00:00Z',
        });
      }),
    );

    // Simulate stored auth state with isAuthenticated = false
    sessionStorage.setItem(
      'auth-storage',
      JSON.stringify({
        state: { user: null, isAuthenticated: false },
        version: 0,
      }),
    );

    useAuthStore.persist.rehydrate();

    // Give it some time to ensure no call is made
    await new Promise((resolve) => setTimeout(resolve, 100));

    expect(meHandler).not.toHaveBeenCalled();
  });
});
