import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import * as githubLinksApi from '@/api/generated/endpoints/github-links/github-links';
import type { GitHubLink } from '@/api/generated/model';
import { GitHubLinkSettings } from './GitHubLinkSettings';

vi.mock('@/api/generated/endpoints/github-links/github-links', () => ({
  useGetGithubLink: vi.fn(),
  useCreateGithubLink: vi.fn(),
  useDeleteGithubLink: vi.fn(),
  useSyncGithubLink: vi.fn(),
  getGetGithubLinkQueryKey: vi.fn((id: string) => [`/api/projects/${id}/github-link`]),
}));

const mockLink: GitHubLink = {
  id: 'link-1',
  project_id: 'project-1',
  repo_owner: 'myorg',
  repo_name: 'myrepo',
  sync_enabled: true,
  last_synced_at: '2026-01-15T10:00:00Z',
  created_at: '2026-01-01T00:00:00Z',
  updated_at: '2026-01-15T10:00:00Z',
};

const createQueryClient = () => new QueryClient({ defaultOptions: { queries: { retry: false } } });

const renderWithProviders = (ui: React.ReactElement) => {
  const queryClient = createQueryClient();
  return render(<QueryClientProvider client={queryClient}>{ui}</QueryClientProvider>);
};

describe('GitHubLinkSettings', () => {
  const mockCreateMutateAsync = vi.fn();
  const mockDeleteMutateAsync = vi.fn();
  const mockSyncMutateAsync = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();

    vi.mocked(githubLinksApi.useCreateGithubLink).mockReturnValue({
      mutateAsync: mockCreateMutateAsync,
      isPending: false,
    } as unknown as ReturnType<typeof githubLinksApi.useCreateGithubLink>);

    vi.mocked(githubLinksApi.useDeleteGithubLink).mockReturnValue({
      mutateAsync: mockDeleteMutateAsync,
      isPending: false,
    } as unknown as ReturnType<typeof githubLinksApi.useDeleteGithubLink>);

    vi.mocked(githubLinksApi.useSyncGithubLink).mockReturnValue({
      mutateAsync: mockSyncMutateAsync,
      isPending: false,
    } as unknown as ReturnType<typeof githubLinksApi.useSyncGithubLink>);
  });

  it('shows loading state', () => {
    vi.mocked(githubLinksApi.useGetGithubLink).mockReturnValue({
      data: undefined,
      isLoading: true,
      isError: false,
    } as unknown as ReturnType<typeof githubLinksApi.useGetGithubLink>);

    renderWithProviders(<GitHubLinkSettings projectId="project-1" />);
    expect(screen.getByText('Loading...')).toBeInTheDocument();
  });

  describe('when no link exists', () => {
    beforeEach(() => {
      vi.mocked(githubLinksApi.useGetGithubLink).mockReturnValue({
        data: undefined,
        isLoading: false,
        isError: true,
      } as unknown as ReturnType<typeof githubLinksApi.useGetGithubLink>);
    });

    it('shows create form', () => {
      renderWithProviders(<GitHubLinkSettings projectId="project-1" />);
      expect(screen.getByLabelText('Owner')).toBeInTheDocument();
      expect(screen.getByLabelText('Repository')).toBeInTheDocument();
      expect(screen.getByRole('button', { name: /link/i })).toBeInTheDocument();
    });

    it('creates a link with owner and repo', async () => {
      const user = userEvent.setup();
      mockCreateMutateAsync.mockResolvedValue(mockLink);

      renderWithProviders(<GitHubLinkSettings projectId="project-1" />);

      await user.type(screen.getByLabelText('Owner'), 'myorg');
      await user.type(screen.getByLabelText('Repository'), 'myrepo');
      await user.click(screen.getByRole('button', { name: /link/i }));

      expect(mockCreateMutateAsync).toHaveBeenCalledWith({
        projectId: 'project-1',
        data: { repo_owner: 'myorg', repo_name: 'myrepo' },
      });
    });

    it('disables link button when fields are empty', () => {
      renderWithProviders(<GitHubLinkSettings projectId="project-1" />);
      expect(screen.getByRole('button', { name: /link/i })).toBeDisabled();
    });
  });

  describe('when link exists', () => {
    beforeEach(() => {
      vi.mocked(githubLinksApi.useGetGithubLink).mockReturnValue({
        data: mockLink,
        isLoading: false,
        isError: false,
      } as unknown as ReturnType<typeof githubLinksApi.useGetGithubLink>);
    });

    it('displays linked repository info', () => {
      renderWithProviders(<GitHubLinkSettings projectId="project-1" />);
      expect(screen.getByText('myorg/myrepo')).toBeInTheDocument();
    });

    it('shows sync button', () => {
      renderWithProviders(<GitHubLinkSettings projectId="project-1" />);
      expect(screen.getByRole('button', { name: /sync/i })).toBeInTheDocument();
    });

    it('triggers sync on button click', async () => {
      const user = userEvent.setup();
      mockSyncMutateAsync.mockResolvedValue({ project_id: 'project-1', pushed: 1, pulled: 2 });

      renderWithProviders(<GitHubLinkSettings projectId="project-1" />);
      await user.click(screen.getByRole('button', { name: /sync/i }));

      expect(mockSyncMutateAsync).toHaveBeenCalledWith({ projectId: 'project-1' });
    });

    it('shows unlink button', () => {
      renderWithProviders(<GitHubLinkSettings projectId="project-1" />);
      expect(screen.getByRole('button', { name: /unlink/i })).toBeInTheDocument();
    });

    it('confirms before unlinking', async () => {
      const user = userEvent.setup();

      renderWithProviders(<GitHubLinkSettings projectId="project-1" />);
      await user.click(screen.getByRole('button', { name: /unlink/i }));

      expect(screen.getByText(/are you sure/i)).toBeInTheDocument();
      expect(screen.getByRole('button', { name: /confirm/i })).toBeInTheDocument();
    });

    it('deletes link on confirm', async () => {
      const user = userEvent.setup();
      mockDeleteMutateAsync.mockResolvedValue(undefined);

      renderWithProviders(<GitHubLinkSettings projectId="project-1" />);
      await user.click(screen.getByRole('button', { name: /unlink/i }));
      await user.click(screen.getByRole('button', { name: /confirm/i }));

      expect(mockDeleteMutateAsync).toHaveBeenCalledWith({ projectId: 'project-1' });
    });

    it('cancels unlink', async () => {
      const user = userEvent.setup();

      renderWithProviders(<GitHubLinkSettings projectId="project-1" />);
      await user.click(screen.getByRole('button', { name: /unlink/i }));
      await user.click(screen.getByRole('button', { name: /cancel/i }));

      expect(mockDeleteMutateAsync).not.toHaveBeenCalled();
      expect(screen.queryByText(/are you sure/i)).not.toBeInTheDocument();
    });
  });
});
