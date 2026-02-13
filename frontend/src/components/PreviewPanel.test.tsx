import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import * as previewsApi from '../api/generated/endpoints/previews/previews';
import type { DockerPreview } from '../api/generated/model';
import { PreviewPanel } from './PreviewPanel';

vi.mock('../api/generated/endpoints/previews/previews', () => ({
  useListPreviews: vi.fn(),
  useCreatePreview: vi.fn(),
  useDeletePreview: vi.fn(),
  useStartPreview: vi.fn(),
  useStopPreview: vi.fn(),
  useRestartPreview: vi.fn(),
}));

const createQueryClient = () =>
  new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });

const renderWithProviders = (ui: React.ReactElement) => {
  const queryClient = createQueryClient();
  return render(<QueryClientProvider client={queryClient}>{ui}</QueryClientProvider>);
};

const makePreview = (overrides: Partial<DockerPreview> = {}): DockerPreview => ({
  id: 'preview-1',
  worktree_name: 'wt-feat-1',
  status: 'pending',
  created_at: '2026-02-13T00:00:00Z',
  updated_at: '2026-02-13T00:00:00Z',
  ...overrides,
});

describe('PreviewPanel', () => {
  const mockCreateMutateAsync = vi.fn();
  const mockDeleteMutateAsync = vi.fn();
  const mockStartMutateAsync = vi.fn();
  const mockStopMutateAsync = vi.fn();
  const mockRestartMutateAsync = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();

    vi.mocked(previewsApi.useListPreviews).mockReturnValue({
      data: [],
      isLoading: false,
    } as unknown as ReturnType<typeof previewsApi.useListPreviews>);

    vi.mocked(previewsApi.useCreatePreview).mockReturnValue({
      mutateAsync: mockCreateMutateAsync,
      isPending: false,
    } as unknown as ReturnType<typeof previewsApi.useCreatePreview>);

    vi.mocked(previewsApi.useDeletePreview).mockReturnValue({
      mutateAsync: mockDeleteMutateAsync,
      isPending: false,
    } as unknown as ReturnType<typeof previewsApi.useDeletePreview>);

    vi.mocked(previewsApi.useStartPreview).mockReturnValue({
      mutateAsync: mockStartMutateAsync,
      isPending: false,
    } as unknown as ReturnType<typeof previewsApi.useStartPreview>);

    vi.mocked(previewsApi.useStopPreview).mockReturnValue({
      mutateAsync: mockStopMutateAsync,
      isPending: false,
    } as unknown as ReturnType<typeof previewsApi.useStopPreview>);

    vi.mocked(previewsApi.useRestartPreview).mockReturnValue({
      mutateAsync: mockRestartMutateAsync,
      isPending: false,
    } as unknown as ReturnType<typeof previewsApi.useRestartPreview>);
  });

  it('shows empty state when no previews exist', () => {
    renderWithProviders(<PreviewPanel />);
    expect(screen.getByText(/no previews/i)).toBeInTheDocument();
  });

  it('shows loading state', () => {
    vi.mocked(previewsApi.useListPreviews).mockReturnValue({
      data: undefined,
      isLoading: true,
    } as unknown as ReturnType<typeof previewsApi.useListPreviews>);

    renderWithProviders(<PreviewPanel />);
    expect(screen.getByText(/loading/i)).toBeInTheDocument();
  });

  it('shows error state when fetch fails', () => {
    vi.mocked(previewsApi.useListPreviews).mockReturnValue({
      data: undefined,
      isLoading: false,
      isError: true,
    } as unknown as ReturnType<typeof previewsApi.useListPreviews>);

    renderWithProviders(<PreviewPanel />);
    expect(screen.getByText(/failed to load/i)).toBeInTheDocument();
  });

  it('renders preview list with status badges', () => {
    vi.mocked(previewsApi.useListPreviews).mockReturnValue({
      data: [
        makePreview({ id: 'p1', worktree_name: 'wt-1', status: 'running' }),
        makePreview({ id: 'p2', worktree_name: 'wt-2', status: 'stopped' }),
      ],
      isLoading: false,
    } as unknown as ReturnType<typeof previewsApi.useListPreviews>);

    renderWithProviders(<PreviewPanel />);

    expect(screen.getByText('wt-1')).toBeInTheDocument();
    expect(screen.getByText('wt-2')).toBeInTheDocument();
    expect(screen.getByText('running')).toBeInTheDocument();
    expect(screen.getByText('stopped')).toBeInTheDocument();
  });

  it('shows preview URL link when running', () => {
    vi.mocked(previewsApi.useListPreviews).mockReturnValue({
      data: [
        makePreview({
          status: 'running',
          preview_url: 'http://localhost:8100',
        }),
      ],
      isLoading: false,
    } as unknown as ReturnType<typeof previewsApi.useListPreviews>);

    renderWithProviders(<PreviewPanel />);

    const link = screen.getByRole('link', { name: /http:\/\/localhost:8100/i });
    expect(link).toHaveAttribute('href', 'http://localhost:8100');
    expect(link).toHaveAttribute('target', '_blank');
  });

  it('shows error message when failed', () => {
    vi.mocked(previewsApi.useListPreviews).mockReturnValue({
      data: [
        makePreview({
          status: 'failed',
          error_message: 'build failed: Dockerfile not found',
        }),
      ],
      isLoading: false,
    } as unknown as ReturnType<typeof previewsApi.useListPreviews>);

    renderWithProviders(<PreviewPanel />);

    expect(screen.getByText(/build failed: Dockerfile not found/)).toBeInTheDocument();
  });

  it('shows Start button for pending/stopped/failed previews', () => {
    vi.mocked(previewsApi.useListPreviews).mockReturnValue({
      data: [makePreview({ status: 'stopped' })],
      isLoading: false,
    } as unknown as ReturnType<typeof previewsApi.useListPreviews>);

    renderWithProviders(<PreviewPanel />);

    expect(screen.getByRole('button', { name: /start/i })).toBeInTheDocument();
  });

  it('shows Stop and Restart buttons when running', () => {
    vi.mocked(previewsApi.useListPreviews).mockReturnValue({
      data: [makePreview({ status: 'running' })],
      isLoading: false,
    } as unknown as ReturnType<typeof previewsApi.useListPreviews>);

    renderWithProviders(<PreviewPanel />);

    expect(screen.getByRole('button', { name: /stop/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /restart/i })).toBeInTheDocument();
  });

  it('shows building indicator when building', () => {
    vi.mocked(previewsApi.useListPreviews).mockReturnValue({
      data: [makePreview({ status: 'building' })],
      isLoading: false,
    } as unknown as ReturnType<typeof previewsApi.useListPreviews>);

    renderWithProviders(<PreviewPanel />);

    expect(screen.getByText('building')).toBeInTheDocument();
    // Start/Stop buttons should not be shown
    expect(screen.queryByRole('button', { name: /start/i })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /stop/i })).not.toBeInTheDocument();
  });

  it('calls start mutation when Start button clicked', async () => {
    mockStartMutateAsync.mockResolvedValue(undefined);

    vi.mocked(previewsApi.useListPreviews).mockReturnValue({
      data: [makePreview({ id: 'p1', status: 'stopped' })],
      isLoading: false,
    } as unknown as ReturnType<typeof previewsApi.useListPreviews>);

    const user = userEvent.setup();
    renderWithProviders(<PreviewPanel />);

    await user.click(screen.getByRole('button', { name: /start/i }));

    expect(mockStartMutateAsync).toHaveBeenCalledWith({ id: 'p1' });
  });

  it('calls stop mutation when Stop button clicked', async () => {
    mockStopMutateAsync.mockResolvedValue(undefined);

    vi.mocked(previewsApi.useListPreviews).mockReturnValue({
      data: [makePreview({ id: 'p1', status: 'running' })],
      isLoading: false,
    } as unknown as ReturnType<typeof previewsApi.useListPreviews>);

    const user = userEvent.setup();
    renderWithProviders(<PreviewPanel />);

    await user.click(screen.getByRole('button', { name: /stop/i }));

    expect(mockStopMutateAsync).toHaveBeenCalledWith({ id: 'p1' });
  });

  it('calls delete mutation when Delete button clicked', async () => {
    mockDeleteMutateAsync.mockResolvedValue(undefined);

    vi.mocked(previewsApi.useListPreviews).mockReturnValue({
      data: [makePreview({ id: 'p1', status: 'stopped' })],
      isLoading: false,
    } as unknown as ReturnType<typeof previewsApi.useListPreviews>);

    const user = userEvent.setup();
    renderWithProviders(<PreviewPanel />);

    await user.click(screen.getByRole('button', { name: /delete/i }));

    expect(mockDeleteMutateAsync).toHaveBeenCalledWith({ id: 'p1' });
  });

  it('calls create mutation with worktree name', async () => {
    mockCreateMutateAsync.mockResolvedValue(makePreview());

    const user = userEvent.setup();
    renderWithProviders(<PreviewPanel />);

    await user.type(screen.getByLabelText(/worktree name/i), 'my-worktree');
    await user.click(screen.getByRole('button', { name: /create/i }));

    expect(mockCreateMutateAsync).toHaveBeenCalledWith({
      data: { worktree_name: 'my-worktree' },
    });
  });

  it('clears input after successful creation', async () => {
    mockCreateMutateAsync.mockResolvedValue(makePreview());

    const user = userEvent.setup();
    renderWithProviders(<PreviewPanel />);

    await user.type(screen.getByLabelText(/worktree name/i), 'my-worktree');
    await user.click(screen.getByRole('button', { name: /create/i }));

    expect((screen.getByLabelText(/worktree name/i) as HTMLInputElement).value).toBe('');
  });

  it('disables create button when input is empty', () => {
    renderWithProviders(<PreviewPanel />);

    expect(screen.getByRole('button', { name: /create/i })).toBeDisabled();
  });
});
