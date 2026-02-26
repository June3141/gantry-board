import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import * as membersApi from '@/api/generated/endpoints/project-members/project-members';
import * as usersApi from '@/api/generated/endpoints/users/users';
import type { ProjectMember, User } from '@/api/generated/model';
import { MemberRole } from '@/api/generated/model';
import { useAuthStore } from '@/stores/authStore';
import { useUiStore } from '@/stores/uiStore';
import { ProjectMembersPanel } from './ProjectMembersPanel';

vi.mock('@/api/generated/endpoints/project-members/project-members', () => ({
  useListMembers: vi.fn(),
  useAddMember: vi.fn(),
  useUpdateMember: vi.fn(),
  useRemoveMember: vi.fn(),
  getListMembersQueryKey: vi.fn(() => ['/api/projects/project-1/members']),
}));

vi.mock('@/api/generated/endpoints/users/users', () => ({
  useSearchUsers: vi.fn(),
}));

const mockMembers: ProjectMember[] = [
  {
    user_id: 'user-1',
    user_name: 'Alice',
    user_email: 'alice@test.com',
    role: MemberRole.owner,
    project_id: 'project-1',
    created_at: '2026-01-01T00:00:00Z',
  },
  {
    user_id: 'user-2',
    user_name: 'Bob',
    user_email: 'bob@test.com',
    role: MemberRole.member,
    project_id: 'project-1',
    created_at: '2026-01-01T01:00:00Z',
  },
];

const mockSearchResults: User[] = [
  {
    id: 'user-3',
    name: 'Carol',
    email: 'carol@test.com',
    created_at: '',
    updated_at: '',
  },
];

const createQueryClient = () => new QueryClient({ defaultOptions: { queries: { retry: false } } });

const renderWithProviders = (ui: React.ReactElement) => {
  const queryClient = createQueryClient();
  return render(<QueryClientProvider client={queryClient}>{ui}</QueryClientProvider>);
};

describe('ProjectMembersPanel', () => {
  const mockAddMutateAsync = vi.fn();
  const mockUpdateMutateAsync = vi.fn();
  const mockRemoveMutateAsync = vi.fn();

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

    useUiStore.setState({ isProjectMembersOpen: false });

    vi.mocked(membersApi.useListMembers).mockReturnValue({
      data: mockMembers,
      isLoading: false,
    } as unknown as ReturnType<typeof membersApi.useListMembers>);

    vi.mocked(membersApi.useAddMember).mockReturnValue({
      mutateAsync: mockAddMutateAsync,
      isPending: false,
    } as unknown as ReturnType<typeof membersApi.useAddMember>);

    vi.mocked(membersApi.useUpdateMember).mockReturnValue({
      mutateAsync: mockUpdateMutateAsync,
      isPending: false,
    } as unknown as ReturnType<typeof membersApi.useUpdateMember>);

    vi.mocked(membersApi.useRemoveMember).mockReturnValue({
      mutateAsync: mockRemoveMutateAsync,
      isPending: false,
    } as unknown as ReturnType<typeof membersApi.useRemoveMember>);

    vi.mocked(usersApi.useSearchUsers).mockReturnValue({
      data: undefined,
      isLoading: false,
    } as unknown as ReturnType<typeof usersApi.useSearchUsers>);
  });

  it('does not render when panel is closed', () => {
    renderWithProviders(<ProjectMembersPanel projectId="project-1" />);
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('renders member list when open', () => {
    useUiStore.setState({ isProjectMembersOpen: true });
    renderWithProviders(<ProjectMembersPanel projectId="project-1" />);
    expect(screen.getByText('Alice')).toBeInTheDocument();
    expect(screen.getByText('Bob')).toBeInTheDocument();
  });

  it('displays member emails', () => {
    useUiStore.setState({ isProjectMembersOpen: true });
    renderWithProviders(<ProjectMembersPanel projectId="project-1" />);
    expect(screen.getByText('alice@test.com')).toBeInTheDocument();
    expect(screen.getByText('bob@test.com')).toBeInTheDocument();
  });

  it('displays role badges', () => {
    useUiStore.setState({ isProjectMembersOpen: true });
    renderWithProviders(<ProjectMembersPanel projectId="project-1" />);
    // Alice (self/owner) shows a static badge, Bob has a role select
    // Check that role text exists somewhere in the document
    expect(screen.getAllByText('Owner').length).toBeGreaterThanOrEqual(1);
    expect(screen.getAllByText('Member').length).toBeGreaterThanOrEqual(1);
  });

  describe('owner permissions', () => {
    it('shows invite section for owner', () => {
      useUiStore.setState({ isProjectMembersOpen: true });
      renderWithProviders(<ProjectMembersPanel projectId="project-1" />);
      expect(screen.getByPlaceholderText(/search users/i)).toBeInTheDocument();
    });

    it('shows remove button for other members', () => {
      useUiStore.setState({ isProjectMembersOpen: true });
      renderWithProviders(<ProjectMembersPanel projectId="project-1" />);
      const removeButtons = screen.getAllByRole('button', {
        name: /remove/i,
      });
      expect(removeButtons).toHaveLength(1); // Only Bob, not self
    });

    it('shows role select for other members', () => {
      useUiStore.setState({ isProjectMembersOpen: true });
      renderWithProviders(<ProjectMembersPanel projectId="project-1" />);
      const roleSelects = screen.getAllByRole('combobox');
      // 1 for Bob's role change + 1 for invite role select
      expect(roleSelects).toHaveLength(2);
    });
  });

  describe('member permissions (read-only)', () => {
    beforeEach(() => {
      // Current user is user-2 (member role)
      useAuthStore.setState({
        user: {
          id: 'user-2',
          email: 'bob@test.com',
          name: 'Bob',
          created_at: '',
          updated_at: '',
        },
        isAuthenticated: true,
        isLoading: false,
      });
    });

    it('does not show invite section for member role', () => {
      useUiStore.setState({ isProjectMembersOpen: true });
      renderWithProviders(<ProjectMembersPanel projectId="project-1" />);
      expect(screen.queryByPlaceholderText(/search users/i)).not.toBeInTheDocument();
    });

    it('does not show remove buttons for member role', () => {
      useUiStore.setState({ isProjectMembersOpen: true });
      renderWithProviders(<ProjectMembersPanel projectId="project-1" />);
      expect(screen.queryByRole('button', { name: /remove/i })).not.toBeInTheDocument();
    });

    it('does not show role selects for member role', () => {
      useUiStore.setState({ isProjectMembersOpen: true });
      renderWithProviders(<ProjectMembersPanel projectId="project-1" />);
      expect(screen.queryByRole('combobox')).not.toBeInTheDocument();
    });
  });

  describe('invite flow', () => {
    it('searches users when typing', async () => {
      const user = userEvent.setup();
      vi.mocked(usersApi.useSearchUsers).mockReturnValue({
        data: mockSearchResults,
        isLoading: false,
      } as unknown as ReturnType<typeof usersApi.useSearchUsers>);
      useUiStore.setState({ isProjectMembersOpen: true });
      renderWithProviders(<ProjectMembersPanel projectId="project-1" />);

      const searchInput = screen.getByPlaceholderText(/search users/i);
      await user.type(searchInput, 'Carol');

      expect(screen.getByText('Carol')).toBeInTheDocument();
      expect(screen.getByText('carol@test.com')).toBeInTheDocument();
    });

    it('adds member when search result is clicked and add pressed', async () => {
      const user = userEvent.setup();
      mockAddMutateAsync.mockResolvedValue({});
      vi.mocked(usersApi.useSearchUsers).mockReturnValue({
        data: mockSearchResults,
        isLoading: false,
      } as unknown as ReturnType<typeof usersApi.useSearchUsers>);
      useUiStore.setState({ isProjectMembersOpen: true });
      renderWithProviders(<ProjectMembersPanel projectId="project-1" />);

      const searchInput = screen.getByPlaceholderText(/search users/i);
      await user.type(searchInput, 'Carol');

      // Click on search result to select
      const resultButtons = screen.getAllByTestId('search-result');
      await user.click(resultButtons[0]);

      // Click Add button
      await user.click(screen.getByRole('button', { name: /^add$/i }));

      expect(mockAddMutateAsync).toHaveBeenCalledWith({
        projectId: 'project-1',
        data: { user_id: 'user-3', role: MemberRole.member },
      });
    });
  });

  describe('role change', () => {
    it('changes member role via select', async () => {
      const user = userEvent.setup();
      mockUpdateMutateAsync.mockResolvedValue({});
      useUiStore.setState({ isProjectMembersOpen: true });
      renderWithProviders(<ProjectMembersPanel projectId="project-1" />);

      const roleSelect = screen.getAllByRole('combobox')[0];
      await user.selectOptions(roleSelect, 'admin');

      expect(mockUpdateMutateAsync).toHaveBeenCalledWith({
        projectId: 'project-1',
        userId: 'user-2',
        data: { role: 'admin' },
      });
    });
  });

  describe('remove member', () => {
    it('removes member with confirmation', async () => {
      const user = userEvent.setup();
      mockRemoveMutateAsync.mockResolvedValue({});
      useUiStore.setState({ isProjectMembersOpen: true });
      renderWithProviders(<ProjectMembersPanel projectId="project-1" />);

      await user.click(screen.getByRole('button', { name: /remove/i }));
      await user.click(screen.getByRole('button', { name: /confirm/i }));

      expect(mockRemoveMutateAsync).toHaveBeenCalledWith({
        projectId: 'project-1',
        userId: 'user-2',
      });
    });

    it('cancels remove', async () => {
      const user = userEvent.setup();
      useUiStore.setState({ isProjectMembersOpen: true });
      renderWithProviders(<ProjectMembersPanel projectId="project-1" />);

      await user.click(screen.getByRole('button', { name: /remove/i }));
      await user.click(screen.getByRole('button', { name: /cancel/i }));

      expect(mockRemoveMutateAsync).not.toHaveBeenCalled();
    });
  });

  it('closes on Escape key', async () => {
    const user = userEvent.setup();
    useUiStore.setState({ isProjectMembersOpen: true });
    renderWithProviders(<ProjectMembersPanel projectId="project-1" />);

    await user.keyboard('{Escape}');
    expect(useUiStore.getState().isProjectMembersOpen).toBe(false);
  });

  it('closes on backdrop click', async () => {
    const user = userEvent.setup();
    useUiStore.setState({ isProjectMembersOpen: true });
    renderWithProviders(<ProjectMembersPanel projectId="project-1" />);

    await user.click(screen.getByRole('dialog'));
    expect(useUiStore.getState().isProjectMembersOpen).toBe(false);
  });
});
