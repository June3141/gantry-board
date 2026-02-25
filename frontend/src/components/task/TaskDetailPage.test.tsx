import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen } from '@testing-library/react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { describe, expect, it, vi } from 'vitest';
import * as membersApi from '@/api/generated/endpoints/project-members/project-members';
import * as tasksApi from '@/api/generated/endpoints/tasks/tasks';
import { TaskPriority, TaskStatus } from '@/api/generated/model';
import { TaskDetailPage } from './TaskDetailPage';

vi.mock('@/api/generated/endpoints/tasks/tasks', () => ({
  useGetTask: vi.fn(),
  useUpdateTask: vi.fn(() => ({ mutateAsync: vi.fn(), isPending: false })),
  useDeleteTask: vi.fn(() => ({ mutateAsync: vi.fn(), isPending: false })),
  getListTasksQueryKey: vi.fn(() => ['/api/tasks']),
}));

vi.mock('@/api/generated/endpoints/project-members/project-members', () => ({
  useListMembers: vi.fn(() => ({ data: [] })),
}));

vi.mock('@/api/generated/endpoints/agent-sessions/agent-sessions', () => ({
  useListAgentSessions: vi.fn(() => ({ data: undefined })),
  useStartAgentSession: vi.fn(() => ({ mutateAsync: vi.fn(), isPending: false })),
}));

vi.mock('@/api/generated/endpoints/task-comments/task-comments', () => ({
  useListComments: vi.fn(() => ({ data: undefined })),
  useCreateComment: vi.fn(() => ({ mutateAsync: vi.fn(), isPending: false })),
  getListCommentsQueryKey: vi.fn(() => ['/api/comments']),
}));

vi.mock('@/api/generated/endpoints/pull-requests/pull-requests', () => ({
  useListPullRequests: vi.fn(() => ({ data: undefined, isLoading: false })),
}));

vi.mock('@/api/generated/endpoints/worktrees/worktrees', () => ({
  useListWorktrees: vi.fn(() => ({ data: undefined, isLoading: false })),
  useCreateWorktree: vi.fn(() => ({ mutateAsync: vi.fn(), isPending: false })),
  useDeleteWorktree: vi.fn(() => ({ mutateAsync: vi.fn(), isPending: false })),
  getListWorktreesQueryKey: vi.fn(() => ['/api/worktrees']),
}));

const createQueryClient = () =>
  new QueryClient({ defaultOptions: { queries: { retry: false } } });

const renderWithProviders = (taskId = 'task-1', projectId = 'proj-1') =>
  render(
    <QueryClientProvider client={createQueryClient()}>
      <MemoryRouter initialEntries={[`/projects/${projectId}/tasks/${taskId}`]}>
        <Routes>
          <Route path="projects/:projectId/tasks/:taskId" element={<TaskDetailPage />} />
          <Route path="projects/:projectId" element={<div>Board Page</div>} />
        </Routes>
      </MemoryRouter>
    </QueryClientProvider>,
  );

describe('TaskDetailPage', () => {
  it('shows loading state', () => {
    vi.mocked(tasksApi.useGetTask).mockReturnValue({
      data: undefined,
      isLoading: true,
      isError: false,
    } as ReturnType<typeof tasksApi.useGetTask>);

    renderWithProviders();

    expect(screen.getByText(/loading/i)).toBeInTheDocument();
  });

  it('shows error state', () => {
    vi.mocked(tasksApi.useGetTask).mockReturnValue({
      data: undefined,
      isLoading: false,
      isError: true,
    } as ReturnType<typeof tasksApi.useGetTask>);

    renderWithProviders();

    expect(screen.getByText(/failed to load/i)).toBeInTheDocument();
  });

  it('renders task title when loaded', () => {
    vi.mocked(tasksApi.useGetTask).mockReturnValue({
      data: {
        id: 'task-1',
        project_id: 'proj-1',
        title: 'Fix the bug',
        description: 'Detailed desc',
        status: TaskStatus.todo,
        priority: TaskPriority.high,
        position: 0,
        created_at: '2026-01-01T00:00:00Z',
        updated_at: '2026-01-01T00:00:00Z',
      },
      isLoading: false,
      isError: false,
    } as ReturnType<typeof tasksApi.useGetTask>);

    renderWithProviders();

    expect(screen.getByText('Fix the bug')).toBeInTheDocument();
  });

  it('has a back to board link', () => {
    vi.mocked(tasksApi.useGetTask).mockReturnValue({
      data: {
        id: 'task-1',
        project_id: 'proj-1',
        title: 'Fix the bug',
        status: TaskStatus.todo,
        priority: TaskPriority.high,
        position: 0,
        created_at: '2026-01-01T00:00:00Z',
        updated_at: '2026-01-01T00:00:00Z',
      },
      isLoading: false,
      isError: false,
    } as ReturnType<typeof tasksApi.useGetTask>);

    renderWithProviders();

    const backLink = screen.getByRole('link', { name: /back to board/i });
    expect(backLink).toHaveAttribute('href', '/projects/proj-1');
  });
});
