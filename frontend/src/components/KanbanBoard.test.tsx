import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import * as membersApi from '../api/generated/endpoints/project-members/project-members';
import * as tasksApi from '../api/generated/endpoints/tasks/tasks';
import type { Task } from '../api/generated/model';
import { TaskPriority, TaskStatus } from '../api/generated/model';
import { useBoardStore } from '../stores/boardStore';
import { KanbanBoard } from './KanbanBoard';

vi.mock('../api/generated/endpoints/tasks/tasks', () => ({
  useListTasks: vi.fn(),
  useUpdateTask: vi.fn(),
}));

vi.mock('../api/generated/endpoints/project-members/project-members', () => ({
  useListMembers: vi.fn(),
}));

const createMockTask = (overrides: Partial<Task> = {}): Task => ({
  id: 'task-1',
  project_id: 'project-1',
  title: 'Test Task',
  status: TaskStatus.todo,
  priority: TaskPriority.medium,
  position: 0,
  created_at: '2026-01-01T00:00:00Z',
  updated_at: '2026-01-01T00:00:00Z',
  ...overrides,
});

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

describe('KanbanBoard', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useBoardStore.getState().clearFilters();
    vi.mocked(membersApi.useListMembers).mockReturnValue({
      data: [],
      isLoading: false,
    } as unknown as ReturnType<typeof membersApi.useListMembers>);
  });

  it('renders all status columns', () => {
    vi.mocked(tasksApi.useListTasks).mockReturnValue({
      data: { data: [], total: 0, limit: 50, offset: 0 },
      isLoading: false,
      error: null,
    } as ReturnType<typeof tasksApi.useListTasks>);
    vi.mocked(tasksApi.useUpdateTask).mockReturnValue({
      mutate: vi.fn(),
    } as unknown as ReturnType<typeof tasksApi.useUpdateTask>);

    renderWithProviders(<KanbanBoard projectId="project-1" />);

    expect(screen.getByText('Backlog')).toBeInTheDocument();
    expect(screen.getByText('To Do')).toBeInTheDocument();
    expect(screen.getByText('In Progress')).toBeInTheDocument();
    expect(screen.getByText('In Review')).toBeInTheDocument();
    expect(screen.getByText('Done')).toBeInTheDocument();
  });

  it('renders loading state', () => {
    vi.mocked(tasksApi.useListTasks).mockReturnValue({
      data: undefined,
      isLoading: true,
      error: null,
    } as ReturnType<typeof tasksApi.useListTasks>);
    vi.mocked(tasksApi.useUpdateTask).mockReturnValue({
      mutate: vi.fn(),
    } as unknown as ReturnType<typeof tasksApi.useUpdateTask>);

    renderWithProviders(<KanbanBoard projectId="project-1" />);

    expect(screen.getByTestId('kanban-loading')).toBeInTheDocument();
  });

  it('distributes tasks to correct columns', () => {
    const tasks = [
      createMockTask({ id: 'task-1', title: 'Backlog Task', status: TaskStatus.backlog }),
      createMockTask({ id: 'task-2', title: 'Todo Task', status: TaskStatus.todo }),
      createMockTask({ id: 'task-3', title: 'In Progress Task', status: TaskStatus.in_progress }),
    ];

    vi.mocked(tasksApi.useListTasks).mockReturnValue({
      data: { data: tasks, total: tasks.length, limit: 50, offset: 0 },
      isLoading: false,
      error: null,
    } as ReturnType<typeof tasksApi.useListTasks>);
    vi.mocked(tasksApi.useUpdateTask).mockReturnValue({
      mutate: vi.fn(),
    } as unknown as ReturnType<typeof tasksApi.useUpdateTask>);

    renderWithProviders(<KanbanBoard projectId="project-1" />);

    expect(screen.getByText('Backlog Task')).toBeInTheDocument();
    expect(screen.getByText('Todo Task')).toBeInTheDocument();
    expect(screen.getByText('In Progress Task')).toBeInTheDocument();
  });

  it('renders error state', () => {
    vi.mocked(tasksApi.useListTasks).mockReturnValue({
      data: undefined,
      isLoading: false,
      error: new Error('Failed to fetch'),
    } as unknown as ReturnType<typeof tasksApi.useListTasks>);
    vi.mocked(tasksApi.useUpdateTask).mockReturnValue({
      mutate: vi.fn(),
    } as unknown as ReturnType<typeof tasksApi.useUpdateTask>);

    renderWithProviders(<KanbanBoard projectId="project-1" />);

    expect(screen.getByTestId('kanban-error')).toBeInTheDocument();
  });

  describe('filtering', () => {
    const allTasks = [
      createMockTask({
        id: 't1',
        title: 'Fix login bug',
        status: TaskStatus.todo,
        priority: TaskPriority.high,
        assigned_to: 'user-1',
      }),
      createMockTask({
        id: 't2',
        title: 'Add dashboard',
        status: TaskStatus.todo,
        priority: TaskPriority.medium,
        assigned_to: 'user-2',
      }),
      createMockTask({
        id: 't3',
        title: 'Setup CI',
        status: TaskStatus.in_progress,
        priority: TaskPriority.low,
      }),
    ];

    beforeEach(() => {
      vi.mocked(tasksApi.useListTasks).mockReturnValue({
        data: { data: allTasks, total: allTasks.length, limit: 50, offset: 0 },
        isLoading: false,
        error: null,
      } as ReturnType<typeof tasksApi.useListTasks>);
      vi.mocked(tasksApi.useUpdateTask).mockReturnValue({
        mutate: vi.fn(),
      } as unknown as ReturnType<typeof tasksApi.useUpdateTask>);
    });

    it('filters by search text (title match)', () => {
      useBoardStore.getState().setSearchText('login');
      renderWithProviders(<KanbanBoard projectId="project-1" />);

      expect(screen.getByText('Fix login bug')).toBeInTheDocument();
      expect(screen.queryByText('Add dashboard')).not.toBeInTheDocument();
      expect(screen.queryByText('Setup CI')).not.toBeInTheDocument();
    });

    it('filters by assignee', () => {
      useBoardStore.getState().setAssigneeFilter(['user-1']);
      renderWithProviders(<KanbanBoard projectId="project-1" />);

      expect(screen.getByText('Fix login bug')).toBeInTheDocument();
      expect(screen.queryByText('Add dashboard')).not.toBeInTheDocument();
      expect(screen.queryByText('Setup CI')).not.toBeInTheDocument();
    });

    it('filters unassigned tasks', () => {
      useBoardStore.getState().setAssigneeFilter(['unassigned']);
      renderWithProviders(<KanbanBoard projectId="project-1" />);

      expect(screen.queryByText('Fix login bug')).not.toBeInTheDocument();
      expect(screen.queryByText('Add dashboard')).not.toBeInTheDocument();
      expect(screen.getByText('Setup CI')).toBeInTheDocument();
    });

    it('filters by priority', () => {
      useBoardStore.getState().setPriorityFilter([TaskPriority.high]);
      renderWithProviders(<KanbanBoard projectId="project-1" />);

      expect(screen.getByText('Fix login bug')).toBeInTheDocument();
      expect(screen.queryByText('Add dashboard')).not.toBeInTheDocument();
      expect(screen.queryByText('Setup CI')).not.toBeInTheDocument();
    });

    it('applies AND combination of multiple filters', () => {
      useBoardStore.getState().setSearchText('bug');
      useBoardStore.getState().setAssigneeFilter(['user-1']);
      useBoardStore.getState().setPriorityFilter([TaskPriority.high]);
      renderWithProviders(<KanbanBoard projectId="project-1" />);

      expect(screen.getByText('Fix login bug')).toBeInTheDocument();
      expect(screen.queryByText('Add dashboard')).not.toBeInTheDocument();
      expect(screen.queryByText('Setup CI')).not.toBeInTheDocument();
    });

    it('renders TaskFilterBar', () => {
      renderWithProviders(<KanbanBoard projectId="project-1" />);
      expect(screen.getByPlaceholderText(/search/i)).toBeInTheDocument();
    });
  });
});
