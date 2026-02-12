import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { AppRoutes } from './App';
import * as authApi from './api/generated/endpoints/auth/auth';
import * as membersApi from './api/generated/endpoints/project-members/project-members';
import * as projectsApi from './api/generated/endpoints/projects/projects';
import * as tasksApi from './api/generated/endpoints/tasks/tasks';
import * as usersApi from './api/generated/endpoints/users/users';
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
}));

vi.mock('./api/generated/endpoints/project-members/project-members', () => ({
  useListMembers: vi.fn(),
  useAddMember: vi.fn(),
  useUpdateMember: vi.fn(),
  useRemoveMember: vi.fn(),
  getListMembersQueryKey: vi.fn(() => ['/api/projects/members']),
}));

vi.mock('./api/generated/endpoints/users/users', () => ({
  useSearchUsers: vi.fn(),
}));

vi.mock('./api/generated/endpoints/tasks/tasks', () => ({
  useListTasks: vi.fn(),
  useUpdateTask: vi.fn(),
  useCreateTask: vi.fn(),
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
    // Mock useCreateProject
    vi.mocked(projectsApi.useCreateProject).mockReturnValue({
      mutateAsync: vi.fn(),
      isPending: false,
    } as unknown as ReturnType<typeof projectsApi.useCreateProject>);
    // Mock useCreateTask
    vi.mocked(tasksApi.useCreateTask).mockReturnValue({
      mutateAsync: vi.fn(),
      isPending: false,
    } as unknown as ReturnType<typeof tasksApi.useCreateTask>);
    // Mock project detail hooks
    vi.mocked(projectsApi.useGetProject).mockReturnValue({
      data: undefined,
      isLoading: false,
      isError: false,
    } as unknown as ReturnType<typeof projectsApi.useGetProject>);
    vi.mocked(projectsApi.useUpdateProject).mockReturnValue({
      mutateAsync: vi.fn(),
      isPending: false,
    } as unknown as ReturnType<typeof projectsApi.useUpdateProject>);
    vi.mocked(projectsApi.useDeleteProject).mockReturnValue({
      mutateAsync: vi.fn(),
      isPending: false,
    } as unknown as ReturnType<typeof projectsApi.useDeleteProject>);
    // Mock member hooks
    vi.mocked(membersApi.useListMembers).mockReturnValue({
      data: [],
      isLoading: false,
    } as unknown as ReturnType<typeof membersApi.useListMembers>);
    vi.mocked(membersApi.useAddMember).mockReturnValue({
      mutateAsync: vi.fn(),
      isPending: false,
    } as unknown as ReturnType<typeof membersApi.useAddMember>);
    vi.mocked(membersApi.useUpdateMember).mockReturnValue({
      mutateAsync: vi.fn(),
      isPending: false,
    } as unknown as ReturnType<typeof membersApi.useUpdateMember>);
    vi.mocked(membersApi.useRemoveMember).mockReturnValue({
      mutateAsync: vi.fn(),
      isPending: false,
    } as unknown as ReturnType<typeof membersApi.useRemoveMember>);
    // Mock user search
    vi.mocked(usersApi.useSearchUsers).mockReturnValue({
      data: undefined,
      isLoading: false,
    } as unknown as ReturnType<typeof usersApi.useSearchUsers>);
    // Reset ui store
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
        data: { data: [], total: 0, limit: 50, offset: 0 },
        isLoading: false,
      } as ReturnType<typeof projectsApi.useListProjects>);

      renderWithProviders(<AppRoutes />);

      expect(screen.getByText('Gantry Board')).toBeInTheDocument();
    });

    it('renders project selector', () => {
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

      expect(screen.getByLabelText('Project:')).toBeInTheDocument();
      expect(screen.getByText('Project One')).toBeInTheDocument();
      expect(screen.getByText('Project Two')).toBeInTheDocument();
    });

    it('shows placeholder when no project is selected', () => {
      vi.mocked(projectsApi.useListProjects).mockReturnValue({
        data: { data: [], total: 0, limit: 50, offset: 0 },
        isLoading: false,
      } as ReturnType<typeof projectsApi.useListProjects>);

      renderWithProviders(<AppRoutes />);

      expect(screen.getByText('Select a project to view its tasks')).toBeInTheDocument();
    });

    it('shows user name and logout button', () => {
      vi.mocked(projectsApi.useListProjects).mockReturnValue({
        data: { data: [], total: 0, limit: 50, offset: 0 },
        isLoading: false,
      } as ReturnType<typeof projectsApi.useListProjects>);

      renderWithProviders(<AppRoutes />);

      expect(screen.getByText('Test User')).toBeInTheDocument();
      expect(screen.getByText('Logout')).toBeInTheDocument();
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
      await user.click(screen.getByRole('button', { name: /new project/i }));

      expect(useUiStore.getState().isProjectModalOpen).toBe(true);
    });
  });
});
