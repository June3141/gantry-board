import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { MemoryRouter } from 'react-router-dom';
import { AppRoutes } from './App';
import * as projectsApi from './api/generated/endpoints/projects/projects';
import * as authApi from './api/generated/endpoints/auth/auth';
import { useAuthStore } from './stores/authStore';

// Mock EventSource for SSE
class MockEventSource {
  onmessage: ((event: MessageEvent) => void) | null = null;
  onerror: ((event: Event) => void) | null = null;
  close() {}
  addEventListener() {}
}
vi.stubGlobal('EventSource', MockEventSource);

vi.mock('./api/generated/endpoints/projects/projects', () => ({
  useListProjects: vi.fn(),
}));

vi.mock('./api/generated/endpoints/tasks/tasks', () => ({
  useListTasks: vi.fn(),
  useUpdateTask: vi.fn(),
  getListTasksQueryKey: vi.fn(() => ['/api/tasks']),
}));

vi.mock('./api/generated/endpoints/auth/auth', () => ({
  useLogin: vi.fn(),
  useLogout: vi.fn(),
  useRegister: vi.fn(),
  useMe: vi.fn(),
}));

const createQueryClient = () =>
  new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,
      },
    },
  });

// Helper to render with all providers and custom initial route
const renderWithProviders = (ui: React.ReactElement, { route = '/' } = {}) => {
  const queryClient = createQueryClient();
  return render(
    <QueryClientProvider client={queryClient}>
      <MemoryRouter initialEntries={[route]}>{ui}</MemoryRouter>
    </QueryClientProvider>,
  );
};

describe('App', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // Reset auth store
    useAuthStore.setState({
      user: null,
      isAuthenticated: false,
      isLoading: false,
    });
    // Mock useMe to not fetch by default
    vi.mocked(authApi.useMe).mockReturnValue({
      data: undefined,
      isLoading: false,
      isError: false,
    } as ReturnType<typeof authApi.useMe>);
    // Mock useLogout
    vi.mocked(authApi.useLogout).mockReturnValue({
      mutateAsync: vi.fn(),
      isPending: false,
    } as unknown as ReturnType<typeof authApi.useLogout>);
    // Mock useLogin
    vi.mocked(authApi.useLogin).mockReturnValue({
      mutateAsync: vi.fn(),
      isPending: false,
    } as unknown as ReturnType<typeof authApi.useLogin>);
    // Mock useRegister
    vi.mocked(authApi.useRegister).mockReturnValue({
      mutateAsync: vi.fn(),
      isPending: false,
    } as unknown as ReturnType<typeof authApi.useRegister>);
  });

  describe('when not authenticated', () => {
    it('redirects to login page', () => {
      vi.mocked(projectsApi.useListProjects).mockReturnValue({
        data: [],
        isLoading: false,
      } as ReturnType<typeof projectsApi.useListProjects>);

      renderWithProviders(<AppRoutes />);

      // Should show login page
      expect(screen.getByText('Sign in to Gantry Board')).toBeInTheDocument();
    });
  });

  describe('when authenticated', () => {
    beforeEach(() => {
      useAuthStore.setState({
        user: {
          id: 'user-1',
          name: 'Test User',
          email: 'test@example.com',
          created_at: '',
          updated_at: '',
        },
        isAuthenticated: true,
        isLoading: false,
      });
      vi.mocked(authApi.useMe).mockReturnValue({
        data: {
          id: 'user-1',
          name: 'Test User',
          email: 'test@example.com',
          created_at: '',
          updated_at: '',
        },
        isLoading: false,
        isError: false,
      } as ReturnType<typeof authApi.useMe>);
    });

    it('renders header with title', () => {
      vi.mocked(projectsApi.useListProjects).mockReturnValue({
        data: [],
        isLoading: false,
      } as ReturnType<typeof projectsApi.useListProjects>);

      renderWithProviders(<AppRoutes />);

      expect(screen.getByText('Gantry Board')).toBeInTheDocument();
    });

    it('renders project selector', () => {
      vi.mocked(projectsApi.useListProjects).mockReturnValue({
        data: [
          { id: 'project-1', name: 'Project One', created_at: '', updated_at: '' },
          { id: 'project-2', name: 'Project Two', created_at: '', updated_at: '' },
        ],
        isLoading: false,
      } as ReturnType<typeof projectsApi.useListProjects>);

      renderWithProviders(<AppRoutes />);

      expect(screen.getByLabelText('Project:')).toBeInTheDocument();
      expect(screen.getByText('Project One')).toBeInTheDocument();
      expect(screen.getByText('Project Two')).toBeInTheDocument();
    });

    it('shows placeholder when no project is selected', () => {
      vi.mocked(projectsApi.useListProjects).mockReturnValue({
        data: [],
        isLoading: false,
      } as ReturnType<typeof projectsApi.useListProjects>);

      renderWithProviders(<AppRoutes />);

      expect(screen.getByText('Select a project to view its tasks')).toBeInTheDocument();
    });

    it('shows user name and logout button', () => {
      vi.mocked(projectsApi.useListProjects).mockReturnValue({
        data: [],
        isLoading: false,
      } as ReturnType<typeof projectsApi.useListProjects>);

      renderWithProviders(<AppRoutes />);

      expect(screen.getByText('Test User')).toBeInTheDocument();
      expect(screen.getByText('Logout')).toBeInTheDocument();
    });
  });
});
