import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import * as pullRequestsApi from '../api/generated/endpoints/pull-requests/pull-requests';
import type { GitHubPullRequest } from '../api/generated/model';
import { PrState } from '../api/generated/model';
import { PullRequestList } from './PullRequestList';

vi.mock('../api/generated/endpoints/pull-requests/pull-requests', () => ({
  useListPullRequests: vi.fn(),
}));

const mockPullRequests: GitHubPullRequest[] = [
  {
    id: 'pr-1',
    task_id: 'task-1',
    github_link_id: 'link-1',
    pr_number: 42,
    title: 'feat: add login form',
    url: 'https://github.com/owner/repo/pull/42',
    state: PrState.open,
    is_merged: false,
    author: 'alice',
    created_at: '2026-01-15T10:00:00Z',
    updated_at: '2026-01-15T10:00:00Z',
  },
  {
    id: 'pr-2',
    task_id: 'task-1',
    github_link_id: 'link-1',
    pr_number: 38,
    title: 'fix: resolve auth bug',
    url: 'https://github.com/owner/repo/pull/38',
    state: PrState.closed,
    is_merged: true,
    author: 'bob',
    created_at: '2026-01-10T10:00:00Z',
    updated_at: '2026-01-12T10:00:00Z',
  },
  {
    id: 'pr-3',
    task_id: 'task-1',
    github_link_id: 'link-1',
    pr_number: 35,
    title: 'chore: cleanup old code',
    url: 'https://github.com/owner/repo/pull/35',
    state: PrState.closed,
    is_merged: false,
    author: null,
    created_at: '2026-01-08T10:00:00Z',
    updated_at: '2026-01-09T10:00:00Z',
  },
];

const createQueryClient = () => new QueryClient({ defaultOptions: { queries: { retry: false } } });

const renderWithProviders = (ui: React.ReactElement) => {
  const queryClient = createQueryClient();
  return render(<QueryClientProvider client={queryClient}>{ui}</QueryClientProvider>);
};

describe('PullRequestList', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('shows loading state', () => {
    vi.mocked(pullRequestsApi.useListPullRequests).mockReturnValue({
      data: undefined,
      isLoading: true,
      isError: false,
    } as unknown as ReturnType<typeof pullRequestsApi.useListPullRequests>);

    renderWithProviders(<PullRequestList taskId="task-1" />);
    expect(screen.getByText('Loading pull requests...')).toBeInTheDocument();
  });

  it('shows empty state when no pull requests', () => {
    vi.mocked(pullRequestsApi.useListPullRequests).mockReturnValue({
      data: [],
      isLoading: false,
      isError: false,
    } as unknown as ReturnType<typeof pullRequestsApi.useListPullRequests>);

    renderWithProviders(<PullRequestList taskId="task-1" />);
    expect(screen.getByText('No pull requests')).toBeInTheDocument();
  });

  it('shows error state', () => {
    vi.mocked(pullRequestsApi.useListPullRequests).mockReturnValue({
      data: undefined,
      isLoading: false,
      isError: true,
    } as unknown as ReturnType<typeof pullRequestsApi.useListPullRequests>);

    renderWithProviders(<PullRequestList taskId="task-1" />);
    expect(screen.getByText('Failed to load pull requests.')).toBeInTheDocument();
  });

  it('renders pull request list with titles and numbers', () => {
    vi.mocked(pullRequestsApi.useListPullRequests).mockReturnValue({
      data: mockPullRequests,
      isLoading: false,
      isError: false,
    } as unknown as ReturnType<typeof pullRequestsApi.useListPullRequests>);

    renderWithProviders(<PullRequestList taskId="task-1" />);

    expect(screen.getByText('#42')).toBeInTheDocument();
    expect(screen.getByText('feat: add login form')).toBeInTheDocument();
    expect(screen.getByText('#38')).toBeInTheDocument();
    expect(screen.getByText('fix: resolve auth bug')).toBeInTheDocument();
    expect(screen.getByText('#35')).toBeInTheDocument();
    expect(screen.getByText('chore: cleanup old code')).toBeInTheDocument();
  });

  it('renders open PR with green badge', () => {
    vi.mocked(pullRequestsApi.useListPullRequests).mockReturnValue({
      data: [mockPullRequests[0]],
      isLoading: false,
      isError: false,
    } as unknown as ReturnType<typeof pullRequestsApi.useListPullRequests>);

    renderWithProviders(<PullRequestList taskId="task-1" />);
    expect(screen.getByText('open')).toBeInTheDocument();
  });

  it('renders merged PR with purple badge', () => {
    vi.mocked(pullRequestsApi.useListPullRequests).mockReturnValue({
      data: [mockPullRequests[1]],
      isLoading: false,
      isError: false,
    } as unknown as ReturnType<typeof pullRequestsApi.useListPullRequests>);

    renderWithProviders(<PullRequestList taskId="task-1" />);
    expect(screen.getByText('merged')).toBeInTheDocument();
  });

  it('renders closed (not merged) PR with red badge', () => {
    vi.mocked(pullRequestsApi.useListPullRequests).mockReturnValue({
      data: [mockPullRequests[2]],
      isLoading: false,
      isError: false,
    } as unknown as ReturnType<typeof pullRequestsApi.useListPullRequests>);

    renderWithProviders(<PullRequestList taskId="task-1" />);
    expect(screen.getByText('closed')).toBeInTheDocument();
  });

  it('renders PR links pointing to GitHub', () => {
    vi.mocked(pullRequestsApi.useListPullRequests).mockReturnValue({
      data: [mockPullRequests[0]],
      isLoading: false,
      isError: false,
    } as unknown as ReturnType<typeof pullRequestsApi.useListPullRequests>);

    renderWithProviders(<PullRequestList taskId="task-1" />);
    const link = screen.getByRole('link', { name: /feat: add login form/ });
    expect(link).toHaveAttribute('href', 'https://github.com/owner/repo/pull/42');
    expect(link).toHaveAttribute('target', '_blank');
  });

  it('displays author name when available', () => {
    vi.mocked(pullRequestsApi.useListPullRequests).mockReturnValue({
      data: [mockPullRequests[0]],
      isLoading: false,
      isError: false,
    } as unknown as ReturnType<typeof pullRequestsApi.useListPullRequests>);

    renderWithProviders(<PullRequestList taskId="task-1" />);
    expect(screen.getByText('alice')).toBeInTheDocument();
  });
});
