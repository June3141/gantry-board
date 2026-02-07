import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import * as tasksApi from '../api/generated/endpoints/tasks/tasks';
import type { Task } from '../api/generated/model';
import { TaskPriority, TaskStatus } from '../api/generated/model';
import { useUiStore } from '../stores/uiStore';
import { TaskDetailModal } from './TaskDetailModal';

vi.mock('../api/generated/endpoints/tasks/tasks', () => ({
  useGetTask: vi.fn(),
  useUpdateTask: vi.fn(),
  useDeleteTask: vi.fn(),
}));

const mockTask: Task = {
  id: 'task-1',
  project_id: 'project-1',
  title: 'Test Task',
  description: 'Test description',
  status: TaskStatus.todo,
  priority: TaskPriority.medium,
  position: 0,
  created_at: '2026-01-01T00:00:00Z',
  updated_at: '2026-01-01T00:00:00Z',
};

const createQueryClient = () =>
  new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });

const renderWithProviders = (ui: React.ReactElement) => {
  const queryClient = createQueryClient();
  return render(<QueryClientProvider client={queryClient}>{ui}</QueryClientProvider>);
};

describe('TaskDetailModal', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(tasksApi.useGetTask).mockReturnValue({
      data: mockTask,
      isLoading: false,
      isError: false,
    } as unknown as ReturnType<typeof tasksApi.useGetTask>);
    vi.mocked(tasksApi.useUpdateTask).mockReturnValue({
      mutateAsync: vi.fn().mockResolvedValue({}),
      isPending: false,
    } as unknown as ReturnType<typeof tasksApi.useUpdateTask>);
    vi.mocked(tasksApi.useDeleteTask).mockReturnValue({
      mutateAsync: vi.fn().mockResolvedValue({}),
      isPending: false,
    } as unknown as ReturnType<typeof tasksApi.useDeleteTask>);
    useUiStore.setState({
      selectedTaskId: null,
      isTaskDetailOpen: false,
    });
  });

  it('does not render when modal is closed', () => {
    renderWithProviders(<TaskDetailModal />);
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('renders when modal is open', () => {
    useUiStore.setState({ selectedTaskId: 'task-1', isTaskDetailOpen: true });
    renderWithProviders(<TaskDetailModal />);
    expect(screen.getByRole('dialog')).toBeInTheDocument();
  });

  it('displays task title', () => {
    useUiStore.setState({ selectedTaskId: 'task-1', isTaskDetailOpen: true });
    renderWithProviders(<TaskDetailModal />);
    expect(screen.getByText('Test Task')).toBeInTheDocument();
  });

  it('displays task description', () => {
    useUiStore.setState({ selectedTaskId: 'task-1', isTaskDetailOpen: true });
    renderWithProviders(<TaskDetailModal />);
    expect(screen.getByText('Test description')).toBeInTheDocument();
  });

  it('displays task status', () => {
    useUiStore.setState({ selectedTaskId: 'task-1', isTaskDetailOpen: true });
    renderWithProviders(<TaskDetailModal />);
    expect(screen.getByText('todo')).toBeInTheDocument();
  });

  it('displays task priority', () => {
    useUiStore.setState({ selectedTaskId: 'task-1', isTaskDetailOpen: true });
    renderWithProviders(<TaskDetailModal />);
    expect(screen.getByText('medium')).toBeInTheDocument();
  });

  it('shows loading state', () => {
    vi.mocked(tasksApi.useGetTask).mockReturnValue({
      data: undefined,
      isLoading: true,
      isError: false,
    } as unknown as ReturnType<typeof tasksApi.useGetTask>);
    useUiStore.setState({ selectedTaskId: 'task-1', isTaskDetailOpen: true });
    renderWithProviders(<TaskDetailModal />);
    expect(screen.getByText(/loading/i)).toBeInTheDocument();
  });

  it('closes on ESC key', async () => {
    const user = userEvent.setup();
    useUiStore.setState({ selectedTaskId: 'task-1', isTaskDetailOpen: true });
    renderWithProviders(<TaskDetailModal />);

    await user.keyboard('{Escape}');

    expect(useUiStore.getState().isTaskDetailOpen).toBe(false);
    expect(useUiStore.getState().selectedTaskId).toBeNull();
  });

  it('closes on backdrop click', async () => {
    const user = userEvent.setup();
    useUiStore.setState({ selectedTaskId: 'task-1', isTaskDetailOpen: true });
    renderWithProviders(<TaskDetailModal />);

    const backdrop = screen.getByRole('dialog');
    await user.click(backdrop);

    expect(useUiStore.getState().isTaskDetailOpen).toBe(false);
  });

  it('displays empty description placeholder when no description', () => {
    vi.mocked(tasksApi.useGetTask).mockReturnValue({
      data: { ...mockTask, description: undefined },
      isLoading: false,
      isError: false,
    } as unknown as ReturnType<typeof tasksApi.useGetTask>);
    useUiStore.setState({ selectedTaskId: 'task-1', isTaskDetailOpen: true });
    renderWithProviders(<TaskDetailModal />);
    expect(screen.getByText(/no description/i)).toBeInTheDocument();
  });
});
