import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import * as worktreesApi from '@/api/generated/endpoints/worktrees/worktrees';
import { WorktreePanel } from './WorktreePanel';

vi.mock('@/api/generated/endpoints/worktrees/worktrees', () => ({
  useListProjectWorktrees: vi.fn(),
  useCreateProjectWorktree: vi.fn(),
  useDeleteProjectWorktree: vi.fn(),
  getListProjectWorktreesQueryKey: vi.fn(() => ['worktrees', 'project-1']),
}));

const createQueryClient = () => new QueryClient({ defaultOptions: { queries: { retry: false } } });

const renderWithProviders = (ui: React.ReactElement) => {
  const queryClient = createQueryClient();
  return render(<QueryClientProvider client={queryClient}>{ui}</QueryClientProvider>);
};

const PROJECT_ID = 'test-project-id';

describe('WorktreePanel', () => {
  const mockCreateMutateAsync = vi.fn();
  const mockDeleteMutateAsync = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();

    vi.mocked(worktreesApi.useListProjectWorktrees).mockReturnValue({
      data: [],
      isLoading: false,
    } as unknown as ReturnType<typeof worktreesApi.useListProjectWorktrees>);

    vi.mocked(worktreesApi.useCreateProjectWorktree).mockReturnValue({
      mutateAsync: mockCreateMutateAsync,
      isPending: false,
    } as unknown as ReturnType<typeof worktreesApi.useCreateProjectWorktree>);

    vi.mocked(worktreesApi.useDeleteProjectWorktree).mockReturnValue({
      mutateAsync: mockDeleteMutateAsync,
      isPending: false,
    } as unknown as ReturnType<typeof worktreesApi.useDeleteProjectWorktree>);
  });

  it('shows worktree list', () => {
    vi.mocked(worktreesApi.useListProjectWorktrees).mockReturnValue({
      data: [
        { name: 'wt-task-1', path: '/tmp/wt-task-1', is_valid: true, branch: 'feat/task-1' },
        { name: 'wt-task-2', path: '/tmp/wt-task-2', is_valid: true, branch: 'feat/task-2' },
      ],
      isLoading: false,
    } as unknown as ReturnType<typeof worktreesApi.useListProjectWorktrees>);

    renderWithProviders(<WorktreePanel projectId={PROJECT_ID} />);

    expect(screen.getByText('wt-task-1')).toBeInTheDocument();
    expect(screen.getByText('wt-task-2')).toBeInTheDocument();
    expect(screen.getByText('feat/task-1')).toBeInTheDocument();
    expect(screen.getByText('feat/task-2')).toBeInTheDocument();
  });

  it('shows create form', () => {
    renderWithProviders(<WorktreePanel projectId={PROJECT_ID} />);

    expect(screen.getByLabelText(/worktree name/i)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /create/i })).toBeInTheDocument();
  });

  it('disables create button when name is empty', () => {
    renderWithProviders(<WorktreePanel projectId={PROJECT_ID} />);

    expect(screen.getByRole('button', { name: /create/i })).toBeDisabled();
  });

  it('creates worktree with project scope', async () => {
    mockCreateMutateAsync.mockResolvedValue({
      name: 'new-wt',
      path: '/tmp/new-wt',
      is_valid: true,
      branch: null,
    });

    const user = userEvent.setup();
    renderWithProviders(<WorktreePanel projectId={PROJECT_ID} />);

    await user.type(screen.getByLabelText(/worktree name/i), 'new-wt');
    await user.click(screen.getByRole('button', { name: /create/i }));

    expect(mockCreateMutateAsync).toHaveBeenCalledWith({
      projectId: PROJECT_ID,
      data: { name: 'new-wt' },
    });
  });

  it('clears input after successful creation', async () => {
    mockCreateMutateAsync.mockResolvedValue({
      name: 'new-wt',
      path: '/tmp/new-wt',
      is_valid: true,
      branch: null,
    });

    const user = userEvent.setup();
    renderWithProviders(<WorktreePanel projectId={PROJECT_ID} />);

    await user.type(screen.getByLabelText(/worktree name/i), 'new-wt');
    await user.click(screen.getByRole('button', { name: /create/i }));

    expect((screen.getByLabelText(/worktree name/i) as HTMLInputElement).value).toBe('');
  });

  it('shows error when creation fails', async () => {
    mockCreateMutateAsync.mockRejectedValue(new Error('Already exists'));

    const user = userEvent.setup();
    renderWithProviders(<WorktreePanel projectId={PROJECT_ID} />);

    await user.type(screen.getByLabelText(/worktree name/i), 'existing-wt');
    await user.click(screen.getByRole('button', { name: /create/i }));

    expect(screen.getByText(/failed to create/i)).toBeInTheDocument();
  });

  it('deletes worktree with confirmation', async () => {
    mockDeleteMutateAsync.mockResolvedValue(undefined);

    vi.mocked(worktreesApi.useListProjectWorktrees).mockReturnValue({
      data: [{ name: 'wt-task-1', path: '/tmp/wt-task-1', is_valid: true, branch: 'feat/task-1' }],
      isLoading: false,
    } as unknown as ReturnType<typeof worktreesApi.useListProjectWorktrees>);

    const user = userEvent.setup();
    renderWithProviders(<WorktreePanel projectId={PROJECT_ID} />);

    // Click delete button
    await user.click(screen.getByRole('button', { name: /delete/i }));

    // Confirmation appears
    expect(screen.getByText(/are you sure/i)).toBeInTheDocument();

    // Confirm deletion
    await user.click(screen.getByRole('button', { name: /confirm/i }));

    expect(mockDeleteMutateAsync).toHaveBeenCalledWith({
      projectId: PROJECT_ID,
      name: 'wt-task-1',
    });
  });

  it('cancels deletion when cancel is clicked', async () => {
    vi.mocked(worktreesApi.useListProjectWorktrees).mockReturnValue({
      data: [{ name: 'wt-task-1', path: '/tmp/wt-task-1', is_valid: true, branch: 'feat/task-1' }],
      isLoading: false,
    } as unknown as ReturnType<typeof worktreesApi.useListProjectWorktrees>);

    const user = userEvent.setup();
    renderWithProviders(<WorktreePanel projectId={PROJECT_ID} />);

    await user.click(screen.getByRole('button', { name: /delete/i }));
    await user.click(screen.getByRole('button', { name: /cancel/i }));

    expect(mockDeleteMutateAsync).not.toHaveBeenCalled();
    expect(screen.queryByText(/are you sure/i)).not.toBeInTheDocument();
  });

  it('shows loading state', () => {
    vi.mocked(worktreesApi.useListProjectWorktrees).mockReturnValue({
      data: undefined,
      isLoading: true,
    } as unknown as ReturnType<typeof worktreesApi.useListProjectWorktrees>);

    renderWithProviders(<WorktreePanel projectId={PROJECT_ID} />);

    expect(screen.getByText(/loading/i)).toBeInTheDocument();
  });

  it('shows empty state when no worktrees', () => {
    renderWithProviders(<WorktreePanel projectId={PROJECT_ID} />);

    expect(screen.getByText(/no worktrees/i)).toBeInTheDocument();
  });

  it('shows error state when fetch fails', () => {
    vi.mocked(worktreesApi.useListProjectWorktrees).mockReturnValue({
      data: undefined,
      isLoading: false,
      isError: true,
    } as unknown as ReturnType<typeof worktreesApi.useListProjectWorktrees>);

    renderWithProviders(<WorktreePanel projectId={PROJECT_ID} />);

    expect(screen.getByText(/failed to load/i)).toBeInTheDocument();
  });

  it('shows invalid badge for invalid worktrees', () => {
    vi.mocked(worktreesApi.useListProjectWorktrees).mockReturnValue({
      data: [{ name: 'wt-broken', path: '/tmp/wt-broken', is_valid: false, branch: null }],
      isLoading: false,
    } as unknown as ReturnType<typeof worktreesApi.useListProjectWorktrees>);

    renderWithProviders(<WorktreePanel projectId={PROJECT_ID} />);

    expect(screen.getByText(/invalid/i)).toBeInTheDocument();
  });
});
