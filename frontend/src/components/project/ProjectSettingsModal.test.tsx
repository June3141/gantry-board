import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import * as membersApi from '@/api/generated/endpoints/project-members/project-members';
import * as projectsApi from '@/api/generated/endpoints/projects/projects';
import type { Project, ProjectMember } from '@/api/generated/model';
import { MemberRole } from '@/api/generated/model';
import { useAuthStore } from '@/stores/authStore';
import { useUiStore } from '@/stores/uiStore';
import { ProjectSettingsModal } from './ProjectSettingsModal';

vi.mock('@/api/generated/endpoints/projects/projects', () => ({
  useGetProject: vi.fn(),
  useUpdateProject: vi.fn(),
  useDeleteProject: vi.fn(),
  getListProjectsQueryKey: vi.fn(() => ['/api/projects']),
  getGetProjectQueryKey: vi.fn((id: string) => ['/api/projects', id]),
}));

vi.mock('@/api/generated/endpoints/project-members/project-members', () => ({
  useListMembers: vi.fn(),
}));

const mockProject: Project = {
  id: 'project-1',
  name: 'Test Project',
  description: 'Test description',
  created_at: '2026-01-01T00:00:00Z',
  updated_at: '2026-01-01T00:00:00Z',
};

const mockMembers: ProjectMember[] = [
  {
    user_id: 'user-1',
    user_name: 'Alice',
    user_email: 'alice@test.com',
    role: MemberRole.owner,
    project_id: 'project-1',
    created_at: '2026-01-01T00:00:00Z',
  },
];

const createQueryClient = () => new QueryClient({ defaultOptions: { queries: { retry: false } } });

const renderWithProviders = (ui: React.ReactElement) => {
  const queryClient = createQueryClient();
  return render(<QueryClientProvider client={queryClient}>{ui}</QueryClientProvider>);
};

describe('ProjectSettingsModal', () => {
  const mockOnProjectDeleted = vi.fn();
  const mockUpdateMutateAsync = vi.fn();
  const mockDeleteMutateAsync = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();

    useAuthStore.setState({
      user: {
        id: 'user-1',
        email: 'alice@test.com',
        name: 'Alice',
        created_at: '',
        updated_at: '',
      },
      isAuthenticated: true,
      isLoading: false,
    });

    useUiStore.setState({
      isProjectSettingsOpen: false,
    });

    vi.mocked(projectsApi.useGetProject).mockReturnValue({
      data: mockProject,
      isLoading: false,
      isError: false,
    } as unknown as ReturnType<typeof projectsApi.useGetProject>);

    vi.mocked(projectsApi.useUpdateProject).mockReturnValue({
      mutateAsync: mockUpdateMutateAsync,
      isPending: false,
    } as unknown as ReturnType<typeof projectsApi.useUpdateProject>);

    vi.mocked(projectsApi.useDeleteProject).mockReturnValue({
      mutateAsync: mockDeleteMutateAsync,
      isPending: false,
    } as unknown as ReturnType<typeof projectsApi.useDeleteProject>);

    vi.mocked(membersApi.useListMembers).mockReturnValue({
      data: mockMembers,
      isLoading: false,
    } as unknown as ReturnType<typeof membersApi.useListMembers>);
  });

  it('does not render when modal is closed', () => {
    renderWithProviders(
      <ProjectSettingsModal projectId="project-1" onProjectDeleted={mockOnProjectDeleted} />,
    );
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('renders when modal is open', () => {
    useUiStore.setState({ isProjectSettingsOpen: true });
    renderWithProviders(
      <ProjectSettingsModal projectId="project-1" onProjectDeleted={mockOnProjectDeleted} />,
    );
    expect(screen.getByRole('dialog')).toBeInTheDocument();
  });

  it('displays project name', () => {
    useUiStore.setState({ isProjectSettingsOpen: true });
    renderWithProviders(
      <ProjectSettingsModal projectId="project-1" onProjectDeleted={mockOnProjectDeleted} />,
    );
    expect(screen.getByText('Test Project')).toBeInTheDocument();
  });

  it('displays project description', () => {
    useUiStore.setState({ isProjectSettingsOpen: true });
    renderWithProviders(
      <ProjectSettingsModal projectId="project-1" onProjectDeleted={mockOnProjectDeleted} />,
    );
    expect(screen.getByText('Test description')).toBeInTheDocument();
  });

  it('enters name edit mode on click', async () => {
    const user = userEvent.setup();
    useUiStore.setState({ isProjectSettingsOpen: true });
    renderWithProviders(
      <ProjectSettingsModal projectId="project-1" onProjectDeleted={mockOnProjectDeleted} />,
    );

    await user.click(screen.getByText('Test Project'));
    expect(screen.getByDisplayValue('Test Project')).toBeInTheDocument();
  });

  it('saves name on blur', async () => {
    const user = userEvent.setup();
    mockUpdateMutateAsync.mockResolvedValue({});
    useUiStore.setState({ isProjectSettingsOpen: true });
    renderWithProviders(
      <ProjectSettingsModal projectId="project-1" onProjectDeleted={mockOnProjectDeleted} />,
    );

    await user.click(screen.getByText('Test Project'));
    const input = screen.getByDisplayValue('Test Project');
    await user.clear(input);
    await user.type(input, 'Updated Name');
    await user.tab();

    expect(mockUpdateMutateAsync).toHaveBeenCalledWith({
      id: 'project-1',
      data: { name: 'Updated Name' },
    });
  });

  it('does not save empty name', async () => {
    const user = userEvent.setup();
    useUiStore.setState({ isProjectSettingsOpen: true });
    renderWithProviders(
      <ProjectSettingsModal projectId="project-1" onProjectDeleted={mockOnProjectDeleted} />,
    );

    await user.click(screen.getByText('Test Project'));
    const input = screen.getByDisplayValue('Test Project');
    await user.clear(input);
    await user.tab();

    expect(mockUpdateMutateAsync).not.toHaveBeenCalled();
  });

  it('saves description on blur', async () => {
    const user = userEvent.setup();
    mockUpdateMutateAsync.mockResolvedValue({});
    useUiStore.setState({ isProjectSettingsOpen: true });
    renderWithProviders(
      <ProjectSettingsModal projectId="project-1" onProjectDeleted={mockOnProjectDeleted} />,
    );

    await user.click(screen.getByText('Test description'));
    const textarea = screen.getByDisplayValue('Test description');
    await user.clear(textarea);
    await user.type(textarea, 'Updated desc');
    await user.tab();

    expect(mockUpdateMutateAsync).toHaveBeenCalledWith({
      id: 'project-1',
      data: { description: 'Updated desc' },
    });
  });

  it('shows delete button for owner', () => {
    useUiStore.setState({ isProjectSettingsOpen: true });
    renderWithProviders(
      <ProjectSettingsModal projectId="project-1" onProjectDeleted={mockOnProjectDeleted} />,
    );
    expect(screen.getByRole('button', { name: /delete project/i })).toBeInTheDocument();
  });

  it('does not show delete button for non-owner', () => {
    vi.mocked(membersApi.useListMembers).mockReturnValue({
      data: [{ ...mockMembers[0], role: MemberRole.member }],
      isLoading: false,
    } as unknown as ReturnType<typeof membersApi.useListMembers>);
    useUiStore.setState({ isProjectSettingsOpen: true });
    renderWithProviders(
      <ProjectSettingsModal projectId="project-1" onProjectDeleted={mockOnProjectDeleted} />,
    );
    expect(screen.queryByRole('button', { name: /delete project/i })).not.toBeInTheDocument();
  });

  it('deletes project on confirm', async () => {
    const user = userEvent.setup();
    mockDeleteMutateAsync.mockResolvedValue({});
    useUiStore.setState({ isProjectSettingsOpen: true });
    renderWithProviders(
      <ProjectSettingsModal projectId="project-1" onProjectDeleted={mockOnProjectDeleted} />,
    );

    await user.click(screen.getByRole('button', { name: /delete project/i }));
    await user.click(screen.getByRole('button', { name: /confirm/i }));

    expect(mockDeleteMutateAsync).toHaveBeenCalledWith({ id: 'project-1' });
    expect(mockOnProjectDeleted).toHaveBeenCalled();
  });

  it('cancels delete', async () => {
    const user = userEvent.setup();
    useUiStore.setState({ isProjectSettingsOpen: true });
    renderWithProviders(
      <ProjectSettingsModal projectId="project-1" onProjectDeleted={mockOnProjectDeleted} />,
    );

    await user.click(screen.getByRole('button', { name: /delete project/i }));
    await user.click(screen.getByRole('button', { name: /cancel/i }));

    expect(mockDeleteMutateAsync).not.toHaveBeenCalled();
  });

  describe('member permissions (read-only)', () => {
    beforeEach(() => {
      vi.mocked(membersApi.useListMembers).mockReturnValue({
        data: [{ ...mockMembers[0], role: MemberRole.member }],
        isLoading: false,
      } as unknown as ReturnType<typeof membersApi.useListMembers>);
    });

    it('does not allow editing name for member role', () => {
      useUiStore.setState({ isProjectSettingsOpen: true });
      renderWithProviders(
        <ProjectSettingsModal projectId="project-1" onProjectDeleted={mockOnProjectDeleted} />,
      );
      // Name should be displayed as static text, not a clickable button
      expect(screen.getByText('Test Project')).toBeInTheDocument();
      expect(screen.getByText('Test Project').tagName).toBe('P');
    });

    it('does not allow editing description for member role', () => {
      useUiStore.setState({ isProjectSettingsOpen: true });
      renderWithProviders(
        <ProjectSettingsModal projectId="project-1" onProjectDeleted={mockOnProjectDeleted} />,
      );
      expect(screen.getByText('Test description')).toBeInTheDocument();
      expect(screen.getByText('Test description').tagName).toBe('P');
    });
  });

  it('closes on Escape key', async () => {
    const user = userEvent.setup();
    useUiStore.setState({ isProjectSettingsOpen: true });
    renderWithProviders(
      <ProjectSettingsModal projectId="project-1" onProjectDeleted={mockOnProjectDeleted} />,
    );

    await user.keyboard('{Escape}');
    expect(useUiStore.getState().isProjectSettingsOpen).toBe(false);
  });

  it('closes on backdrop click', async () => {
    const user = userEvent.setup();
    useUiStore.setState({ isProjectSettingsOpen: true });
    renderWithProviders(
      <ProjectSettingsModal projectId="project-1" onProjectDeleted={mockOnProjectDeleted} />,
    );

    const overlay = document.querySelector('[data-slot="dialog-overlay"]') as HTMLElement;
    await user.click(overlay);
    expect(useUiStore.getState().isProjectSettingsOpen).toBe(false);
  });
});
