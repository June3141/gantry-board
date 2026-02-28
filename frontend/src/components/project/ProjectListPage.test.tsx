import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router';
import { describe, expect, it, vi } from 'vitest';
import * as projectsApi from '@/api/generated/endpoints/projects/projects';
import { ProjectListPage } from './ProjectListPage';

vi.mock('@/api/generated/endpoints/projects/projects', () => ({
  useListProjects: vi.fn(),
  useCreateProject: vi.fn(() => ({ mutateAsync: vi.fn(), isPending: false })),
  getListProjectsQueryKey: vi.fn(() => ['/api/projects']),
}));

const createQueryClient = () => new QueryClient({ defaultOptions: { queries: { retry: false } } });

const renderWithProviders = (ui: React.ReactElement) =>
  render(
    <QueryClientProvider client={createQueryClient()}>
      <MemoryRouter>{ui}</MemoryRouter>
    </QueryClientProvider>,
  );

describe('ProjectListPage', () => {
  it('shows loading state', () => {
    vi.mocked(projectsApi.useListProjects).mockReturnValue({
      data: undefined,
      isLoading: true,
    } as ReturnType<typeof projectsApi.useListProjects>);

    renderWithProviders(<ProjectListPage />);

    expect(screen.getByText(/loading/i)).toBeInTheDocument();
  });

  it('shows empty state when no projects', () => {
    vi.mocked(projectsApi.useListProjects).mockReturnValue({
      data: { data: [], total: 0, limit: 50, offset: 0 },
      isLoading: false,
    } as ReturnType<typeof projectsApi.useListProjects>);

    renderWithProviders(<ProjectListPage />);

    expect(screen.getByText(/no projects/i)).toBeInTheDocument();
  });

  it('renders project cards', () => {
    vi.mocked(projectsApi.useListProjects).mockReturnValue({
      data: {
        data: [
          {
            id: 'p1',
            name: 'Project Alpha',
            description: 'Desc A',
            created_at: '',
            updated_at: '',
          },
          { id: 'p2', name: 'Project Beta', created_at: '', updated_at: '' },
        ],
        total: 2,
        limit: 50,
        offset: 0,
      },
      isLoading: false,
    } as ReturnType<typeof projectsApi.useListProjects>);

    renderWithProviders(<ProjectListPage />);

    expect(screen.getByText('Project Alpha')).toBeInTheDocument();
    expect(screen.getByText('Project Beta')).toBeInTheDocument();
  });

  it('has a new project button', () => {
    vi.mocked(projectsApi.useListProjects).mockReturnValue({
      data: { data: [], total: 0, limit: 50, offset: 0 },
      isLoading: false,
    } as ReturnType<typeof projectsApi.useListProjects>);

    renderWithProviders(<ProjectListPage />);

    expect(screen.getByRole('button', { name: /new project/i })).toBeInTheDocument();
  });
});
