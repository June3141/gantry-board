import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import App from './App';
import * as projectsApi from './api/generated/endpoints/projects/projects';

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
}));

vi.mock('./api/generated/endpoints/tasks/tasks', () => ({
  useListTasks: vi.fn(),
  useUpdateTask: vi.fn(),
  getListTasksQueryKey: vi.fn(() => ['/api/tasks']),
}));

const createQueryClient = () =>
  new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,
      },
    },
  });

const renderWithProviders = (ui: React.ReactElement) => {
  const queryClient = createQueryClient();
  return render(<QueryClientProvider client={queryClient}>{ui}</QueryClientProvider>);
};

describe('App', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders header with title', () => {
    vi.mocked(projectsApi.useListProjects).mockReturnValue({
      data: [],
      isLoading: false,
    } as ReturnType<typeof projectsApi.useListProjects>);

    renderWithProviders(<App />);

    expect(screen.getByText('Gantry Board')).toBeInTheDocument();
  });

  it('renders project selector', () => {
    vi.mocked(projectsApi.useListProjects).mockReturnValue({
      data: [
        { id: 'project-1', name: 'Project One', created_at: '', updated_at: '' },
        { id: 'project-2', name: 'Project Two', created_at: '', updated_at: '' },
      ],
      isLoading: false,
    } as ReturnType<typeof projectsApi.useListProjects>);

    renderWithProviders(<App />);

    expect(screen.getByLabelText('Project:')).toBeInTheDocument();
    expect(screen.getByText('Project One')).toBeInTheDocument();
    expect(screen.getByText('Project Two')).toBeInTheDocument();
  });

  it('shows placeholder when no project is selected', () => {
    vi.mocked(projectsApi.useListProjects).mockReturnValue({
      data: [],
      isLoading: false,
    } as ReturnType<typeof projectsApi.useListProjects>);

    renderWithProviders(<App />);

    expect(screen.getByText('Select a project to view its tasks')).toBeInTheDocument();
  });
});
