import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen } from '@testing-library/react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { describe, expect, it, vi } from 'vitest';
import { useAuthStore } from '@/stores/authStore';
import { AppLayout } from './AppLayout';

// Mock EventSource for SSE
class MockEventSource {
  onmessage: ((event: MessageEvent) => void) | null = null;
  onerror: ((event: Event) => void) | null = null;
  close() {}
  addEventListener() {}
}
vi.stubGlobal('EventSource', MockEventSource);

vi.mock('@/api/generated/endpoints/auth/auth', () => ({
  useLogout: vi.fn(() => ({ mutateAsync: vi.fn(), isPending: false })),
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => {
      const translations: Record<string, string> = {
        'app.title': 'Gantry Board',
        'auth.logout': 'Logout',
      };
      return translations[key] ?? key;
    },
    i18n: { language: 'en', changeLanguage: vi.fn() },
  }),
}));

const createQueryClient = () => new QueryClient({ defaultOptions: { queries: { retry: false } } });

const renderWithProviders = (route = '/') => {
  const queryClient = createQueryClient();
  return render(
    <QueryClientProvider client={queryClient}>
      <MemoryRouter initialEntries={[route]}>
        <Routes>
          <Route element={<AppLayout />}>
            <Route index element={<div>Index Page</div>} />
            <Route path="projects/:projectId" element={<div>Board Page</div>} />
          </Route>
        </Routes>
      </MemoryRouter>
    </QueryClientProvider>,
  );
};

describe('AppLayout', () => {
  it('renders header with logo', () => {
    useAuthStore.setState({
      user: { id: 'u1', name: 'Alice', email: 'a@b.com', is_admin: false, created_at: '', updated_at: '' },
      isAuthenticated: true,
      isLoading: false,
    });

    renderWithProviders();

    expect(screen.getByText('Gantry Board')).toBeInTheDocument();
  });

  it('renders user name', () => {
    useAuthStore.setState({
      user: { id: 'u1', name: 'Alice', email: 'a@b.com', is_admin: false, created_at: '', updated_at: '' },
      isAuthenticated: true,
      isLoading: false,
    });

    renderWithProviders();

    expect(screen.getByText('Alice')).toBeInTheDocument();
  });

  it('renders outlet content', () => {
    useAuthStore.setState({
      user: { id: 'u1', name: 'Alice', email: 'a@b.com', is_admin: false, created_at: '', updated_at: '' },
      isAuthenticated: true,
      isLoading: false,
    });

    renderWithProviders();

    expect(screen.getByText('Index Page')).toBeInTheDocument();
  });

  it('renders logout button', () => {
    useAuthStore.setState({
      user: { id: 'u1', name: 'Alice', email: 'a@b.com', is_admin: false, created_at: '', updated_at: '' },
      isAuthenticated: true,
      isLoading: false,
    });

    renderWithProviders();

    expect(screen.getByRole('button', { name: /logout/i })).toBeInTheDocument();
  });
});
