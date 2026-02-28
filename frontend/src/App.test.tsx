import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter } from 'react-router';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { AppRoutes } from './App';
import * as authApi from './api/generated/endpoints/auth/auth';
import * as projectsApi from './api/generated/endpoints/projects/projects';
import { useAuthStore } from './stores/authStore';
import { useUiStore } from './stores/uiStore';

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
  useCreateProject: vi.fn(),
  useGetProject: vi.fn(),
  useUpdateProject: vi.fn(),
  useDeleteProject: vi.fn(),
  getListProjectsQueryKey: vi.fn(() => ['/api/projects']),
  getGetProjectQueryKey: vi.fn(() => ['/api/projects']),
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
    useAuthStore.setState({
      user: null,
      isAuthenticated: false,
      isLoading: false,
    });
    vi.mocked(authApi.useMe).mockReturnValue({
      data: undefined,
      isLoading: false,
      isError: false,
    } as ReturnType<typeof authApi.useMe>);
    vi.mocked(authApi.useLogout).mockReturnValue({
      mutateAsync: vi.fn(),
      isPending: false,
    } as unknown as ReturnType<typeof authApi.useLogout>);
    vi.mocked(authApi.useLogin).mockReturnValue({
      mutateAsync: vi.fn(),
      isPending: false,
    } as unknown as ReturnType<typeof authApi.useLogin>);
    vi.mocked(authApi.useRegister).mockReturnValue({
      mutateAsync: vi.fn(),
      isPending: false,
    } as unknown as ReturnType<typeof authApi.useRegister>);
    vi.mocked(projectsApi.useCreateProject).mockReturnValue({
      mutateAsync: vi.fn(),
      isPending: false,
    } as unknown as ReturnType<typeof projectsApi.useCreateProject>);
    useUiStore.setState({
      isTaskModalOpen: false,
      defaultStatus: null,
      isProjectModalOpen: false,
      isProjectSettingsOpen: false,
      isProjectMembersOpen: false,
    });
  });

  describe('when not authenticated', () => {
    it('redirects to login page', () => {
      vi.mocked(projectsApi.useListProjects).mockReturnValue({
        data: { data: [], total: 0, limit: 50, offset: 0 },
        isLoading: false,
      } as ReturnType<typeof projectsApi.useListProjects>);

      renderWithProviders(<AppRoutes />);

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
          is_admin: false,
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
        data: { data: [], total: 0, limit: 50, offset: 0 },
        isLoading: false,
      } as ReturnType<typeof projectsApi.useListProjects>);

      renderWithProviders(<AppRoutes />);

      expect(screen.getByText('Gantry Board')).toBeInTheDocument();
    });

    it('renders project list page with project cards', () => {
      vi.mocked(projectsApi.useListProjects).mockReturnValue({
        data: {
          data: [
            { id: 'project-1', name: 'Project One', created_at: '', updated_at: '' },
            { id: 'project-2', name: 'Project Two', created_at: '', updated_at: '' },
          ],
          total: 2,
          limit: 50,
          offset: 0,
        },
        isLoading: false,
      } as ReturnType<typeof projectsApi.useListProjects>);

      renderWithProviders(<AppRoutes />);

      expect(screen.getByText('Project One')).toBeInTheDocument();
      expect(screen.getByText('Project Two')).toBeInTheDocument();
    });

    it('shows empty state when no projects', () => {
      vi.mocked(projectsApi.useListProjects).mockReturnValue({
        data: { data: [], total: 0, limit: 50, offset: 0 },
        isLoading: false,
      } as ReturnType<typeof projectsApi.useListProjects>);

      renderWithProviders(<AppRoutes />);

      expect(screen.getByText(/no projects/i)).toBeInTheDocument();
    });

    it('shows user name and logout button', () => {
      vi.mocked(projectsApi.useListProjects).mockReturnValue({
        data: { data: [], total: 0, limit: 50, offset: 0 },
        isLoading: false,
      } as ReturnType<typeof projectsApi.useListProjects>);

      renderWithProviders(<AppRoutes />);

      expect(screen.getByText('Test User')).toBeInTheDocument();
      expect(screen.getByRole('button', { name: /logout/i })).toBeInTheDocument();
    });

    it('renders new project button', () => {
      vi.mocked(projectsApi.useListProjects).mockReturnValue({
        data: { data: [], total: 0, limit: 50, offset: 0 },
        isLoading: false,
      } as ReturnType<typeof projectsApi.useListProjects>);

      renderWithProviders(<AppRoutes />);

      expect(screen.getByRole('button', { name: /new project/i })).toBeInTheDocument();
    });

    it('opens project modal on new project button click', async () => {
      const user = userEvent.setup();
      vi.mocked(projectsApi.useListProjects).mockReturnValue({
        data: { data: [], total: 0, limit: 50, offset: 0 },
        isLoading: false,
      } as ReturnType<typeof projectsApi.useListProjects>);

      renderWithProviders(<AppRoutes />);
      await user.click(screen.getAllByRole('button', { name: /new project/i })[0]);

      expect(useUiStore.getState().isProjectModalOpen).toBe(true);
    });
  });
});
