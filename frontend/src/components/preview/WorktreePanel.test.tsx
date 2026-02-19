import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import * as worktreesApi from '@/api/generated/endpoints/worktrees/worktrees';
import { WorktreePanel } from './WorktreePanel';

vi.mock('@/api/generated/endpoints/worktrees/worktrees', () => ({
  useListWorktrees: vi.fn(),
  useCreateWorktree: vi.fn(),
  useDeleteWorktree: vi.fn(),
}));

const createQueryClient = () => new QueryClient({ defaultOptions: { queries: { retry: false } } });

const renderWithProviders = (ui: React.ReactElement) => {
  const queryClient = createQueryClient();
  return render(<QueryClientProvider client={queryClient}>{ui}</QueryClientProvider>);
};

describe('WorktreePanel', () => {
  const mockCreateMutateAsync = vi.fn();
  const mockDeleteMutateAsync = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();

    vi.mocked(worktreesApi.useListWorktrees).mockReturnValue({
      data: [],
      isLoading: false,
    } as unknown as ReturnType<typeof worktreesApi.useListWorktrees>);

    vi.mocked(worktreesApi.useCreateWorktree).mockReturnValue({
      mutateAsync: mockCreateMutateAsync,
      isPending: false,
    } as unknown as ReturnType<typeof worktreesApi.useCreateWorktree>);

    vi.mocked(worktreesApi.useDeleteWorktree).mockReturnValue({
      mutateAsync: mockDeleteMutateAsync,
      isPending: false,
    } as unknown as ReturnType<typeof worktreesApi.useDeleteWorktree>);
  });

  it('shows worktree list', () => {
    vi.mocked(worktreesApi.useListWorktrees).mockReturnValue({
      data: [
        { name: 'wt-task-1', path: '/tmp/wt-task-1', is_valid: true, branch: 'feat/task-1' },
        { name: 'wt-task-2', path: '/tmp/wt-task-2', is_valid: true, branch: 'feat/task-2' },
      ],
      isLoading: false,
    } as unknown as ReturnType<typeof worktreesApi.useListWorktrees>);

    renderWithProviders(<WorktreePanel />);

    expect(screen.getByText('wt-task-1')).toBeInTheDocument();
    expect(screen.getByText('wt-task-2')).toBeInTheDocument();
    expect(screen.getByText('feat/task-1')).toBeInTheDocument();
    expect(screen.getByText('feat/task-2')).toBeInTheDocument();
  });

  it('shows create form', () => {
    renderWithProviders(<WorktreePanel />);

    expect(screen.getByLabelText(/worktree name/i)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /create/i })).toBeInTheDocument();
  });

  it('disables create button when name is empty', () => {
    renderWithProviders(<WorktreePanel />);

    expect(screen.getByRole('button', { name: /create/i })).toBeDisabled();
  });

  it('creates worktree when form is submitted', async () => {
    mockCreateMutateAsync.mockResolvedValue({
      name: 'new-wt',
      path: '/tmp/new-wt',
      is_valid: true,
      branch: null,
    });

    const user = userEvent.setup();
    renderWithProviders(<WorktreePanel />);

    await user.type(screen.getByLabelText(/worktree name/i), 'new-wt');
    await user.click(screen.getByRole('button', { name: /create/i }));

    expect(mockCreateMutateAsync).toHaveBeenCalledWith({ data: { name: 'new-wt' } });
  });

  it('clears input after successful creation', async () => {
    mockCreateMutateAsync.mockResolvedValue({
      name: 'new-wt',
      path: '/tmp/new-wt',
      is_valid: true,
      branch: null,
    });

    const user = userEvent.setup();
    renderWithProviders(<WorktreePanel />);

    await user.type(screen.getByLabelText(/worktree name/i), 'new-wt');
    await user.click(screen.getByRole('button', { name: /create/i }));

    expect((screen.getByLabelText(/worktree name/i) as HTMLInputElement).value).toBe('');
  });

  it('shows error when creation fails', async () => {
    mockCreateMutateAsync.mockRejectedValue(new Error('Already exists'));

    const user = userEvent.setup();
    renderWithProviders(<WorktreePanel />);

    await user.type(screen.getByLabelText(/worktree name/i), 'existing-wt');
    await user.click(screen.getByRole('button', { name: /create/i }));

    expect(screen.getByText(/failed to create/i)).toBeInTheDocument();
  });

  it('deletes worktree with confirmation', async () => {
    mockDeleteMutateAsync.mockResolvedValue(undefined);

    vi.mocked(worktreesApi.useListWorktrees).mockReturnValue({
      data: [{ name: 'wt-task-1', path: '/tmp/wt-task-1', is_valid: true, branch: 'feat/task-1' }],
      isLoading: false,
    } as unknown as ReturnType<typeof worktreesApi.useListWorktrees>);

    const user = userEvent.setup();
    renderWithProviders(<WorktreePanel />);

    // Click delete button
    await user.click(screen.getByRole('button', { name: /delete/i }));

    // Confirmation appears
    expect(screen.getByText(/are you sure/i)).toBeInTheDocument();

    // Confirm deletion
    await user.click(screen.getByRole('button', { name: /confirm/i }));

    expect(mockDeleteMutateAsync).toHaveBeenCalledWith({ name: 'wt-task-1' });
  });

  it('cancels deletion when cancel is clicked', async () => {
    vi.mocked(worktreesApi.useListWorktrees).mockReturnValue({
      data: [{ name: 'wt-task-1', path: '/tmp/wt-task-1', is_valid: true, branch: 'feat/task-1' }],
      isLoading: false,
    } as unknown as ReturnType<typeof worktreesApi.useListWorktrees>);

    const user = userEvent.setup();
    renderWithProviders(<WorktreePanel />);

    await user.click(screen.getByRole('button', { name: /delete/i }));
    await user.click(screen.getByRole('button', { name: /cancel/i }));

    expect(mockDeleteMutateAsync).not.toHaveBeenCalled();
    expect(screen.queryByText(/are you sure/i)).not.toBeInTheDocument();
  });

  it('shows loading state', () => {
    vi.mocked(worktreesApi.useListWorktrees).mockReturnValue({
      data: undefined,
      isLoading: true,
    } as unknown as ReturnType<typeof worktreesApi.useListWorktrees>);

    renderWithProviders(<WorktreePanel />);

    expect(screen.getByText(/loading/i)).toBeInTheDocument();
  });

  it('shows empty state when no worktrees', () => {
    renderWithProviders(<WorktreePanel />);

    expect(screen.getByText(/no worktrees/i)).toBeInTheDocument();
  });

  it('shows error state when fetch fails', () => {
    vi.mocked(worktreesApi.useListWorktrees).mockReturnValue({
      data: undefined,
      isLoading: false,
      isError: true,
    } as unknown as ReturnType<typeof worktreesApi.useListWorktrees>);

    renderWithProviders(<WorktreePanel />);

    expect(screen.getByText(/failed to load/i)).toBeInTheDocument();
  });

  it('shows invalid badge for invalid worktrees', () => {
    vi.mocked(worktreesApi.useListWorktrees).mockReturnValue({
      data: [{ name: 'wt-broken', path: '/tmp/wt-broken', is_valid: false, branch: null }],
      isLoading: false,
    } as unknown as ReturnType<typeof worktreesApi.useListWorktrees>);

    renderWithProviders(<WorktreePanel />);

    expect(screen.getByText(/invalid/i)).toBeInTheDocument();
  });
});
