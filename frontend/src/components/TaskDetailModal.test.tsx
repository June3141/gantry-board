import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import * as agentSessionsApi from '../api/generated/endpoints/agent-sessions/agent-sessions';
import * as commentsApi from '../api/generated/endpoints/task-comments/task-comments';
import * as tasksApi from '../api/generated/endpoints/tasks/tasks';
import * as worktreesApi from '../api/generated/endpoints/worktrees/worktrees';
import type { Task } from '../api/generated/model';
import { TaskPriority, TaskStatus } from '../api/generated/model';
import { useUiStore } from '../stores/uiStore';
import { TaskDetailModal } from './TaskDetailModal';

vi.mock('../api/generated/endpoints/tasks/tasks', () => ({
  useGetTask: vi.fn(),
  useUpdateTask: vi.fn(),
  useDeleteTask: vi.fn(),
}));

vi.mock('../api/generated/endpoints/agent-sessions/agent-sessions', () => ({
  useListAgentSessions: vi.fn(),
  useStartAgentSession: vi.fn(),
  useStopAgentSession: vi.fn(),
  useGetAgentSessionOutputs: vi.fn(),
}));

vi.mock('../api/generated/endpoints/worktrees/worktrees', () => ({
  useListWorktrees: vi.fn(),
  useCreateWorktree: vi.fn(),
  useDeleteWorktree: vi.fn(),
}));

vi.mock('../api/generated/endpoints/task-comments/task-comments', () => ({
  useListComments: vi.fn(),
  useCreateComment: vi.fn(),
  useUpdateComment: vi.fn(),
  useDeleteComment: vi.fn(),
}));

vi.mock('../hooks/useAgentEvents', () => ({
  useAgentEvents: vi.fn(),
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

    vi.mocked(agentSessionsApi.useListAgentSessions).mockReturnValue({
      data: [],
      isLoading: false,
    } as unknown as ReturnType<typeof agentSessionsApi.useListAgentSessions>);

    vi.mocked(agentSessionsApi.useStartAgentSession).mockReturnValue({
      mutateAsync: vi.fn(),
      isPending: false,
    } as unknown as ReturnType<typeof agentSessionsApi.useStartAgentSession>);

    vi.mocked(agentSessionsApi.useStopAgentSession).mockReturnValue({
      mutateAsync: vi.fn(),
      isPending: false,
    } as unknown as ReturnType<typeof agentSessionsApi.useStopAgentSession>);

    vi.mocked(agentSessionsApi.useGetAgentSessionOutputs).mockReturnValue({
      data: undefined,
      isLoading: false,
    } as unknown as ReturnType<typeof agentSessionsApi.useGetAgentSessionOutputs>);

    vi.mocked(worktreesApi.useListWorktrees).mockReturnValue({
      data: [],
      isLoading: false,
    } as unknown as ReturnType<typeof worktreesApi.useListWorktrees>);

    vi.mocked(worktreesApi.useCreateWorktree).mockReturnValue({
      mutateAsync: vi.fn(),
      isPending: false,
    } as unknown as ReturnType<typeof worktreesApi.useCreateWorktree>);

    vi.mocked(worktreesApi.useDeleteWorktree).mockReturnValue({
      mutateAsync: vi.fn(),
      isPending: false,
    } as unknown as ReturnType<typeof worktreesApi.useDeleteWorktree>);

    vi.mocked(commentsApi.useListComments).mockReturnValue({
      data: [],
      isLoading: false,
    } as unknown as ReturnType<typeof commentsApi.useListComments>);

    vi.mocked(commentsApi.useCreateComment).mockReturnValue({
      mutateAsync: vi.fn(),
      isPending: false,
    } as unknown as ReturnType<typeof commentsApi.useCreateComment>);

    vi.mocked(commentsApi.useUpdateComment).mockReturnValue({
      mutateAsync: vi.fn(),
      isPending: false,
    } as unknown as ReturnType<typeof commentsApi.useUpdateComment>);

    vi.mocked(commentsApi.useDeleteComment).mockReturnValue({
      mutateAsync: vi.fn(),
      isPending: false,
    } as unknown as ReturnType<typeof commentsApi.useDeleteComment>);
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

  it('displays task status as select', () => {
    useUiStore.setState({ selectedTaskId: 'task-1', isTaskDetailOpen: true });
    renderWithProviders(<TaskDetailModal />);
    const statusSelect = screen.getByLabelText(/status/i) as HTMLSelectElement;
    expect(statusSelect.value).toBe('todo');
  });

  it('displays task priority as select', () => {
    useUiStore.setState({ selectedTaskId: 'task-1', isTaskDetailOpen: true });
    renderWithProviders(<TaskDetailModal />);
    const prioritySelect = screen.getByLabelText(/priority/i) as HTMLSelectElement;
    expect(prioritySelect.value).toBe('medium');
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

  describe('inline editing', () => {
    it('enters title edit mode on click', async () => {
      const user = userEvent.setup();
      useUiStore.setState({ selectedTaskId: 'task-1', isTaskDetailOpen: true });
      renderWithProviders(<TaskDetailModal />);

      await user.click(screen.getByText('Test Task'));

      expect(screen.getByDisplayValue('Test Task')).toBeInTheDocument();
    });

    it('saves title on blur', async () => {
      const mockMutateAsync = vi.fn().mockResolvedValue({});
      vi.mocked(tasksApi.useUpdateTask).mockReturnValue({
        mutateAsync: mockMutateAsync,
        isPending: false,
      } as unknown as ReturnType<typeof tasksApi.useUpdateTask>);

      const user = userEvent.setup();
      useUiStore.setState({ selectedTaskId: 'task-1', isTaskDetailOpen: true });
      renderWithProviders(<TaskDetailModal />);

      await user.click(screen.getByText('Test Task'));
      const input = screen.getByDisplayValue('Test Task');
      await user.clear(input);
      await user.type(input, 'Updated Title');
      await user.tab();

      expect(mockMutateAsync).toHaveBeenCalledWith({
        id: 'task-1',
        data: { title: 'Updated Title' },
      });
    });

    it('does not save empty title', async () => {
      const mockMutateAsync = vi.fn().mockResolvedValue({});
      vi.mocked(tasksApi.useUpdateTask).mockReturnValue({
        mutateAsync: mockMutateAsync,
        isPending: false,
      } as unknown as ReturnType<typeof tasksApi.useUpdateTask>);

      const user = userEvent.setup();
      useUiStore.setState({ selectedTaskId: 'task-1', isTaskDetailOpen: true });
      renderWithProviders(<TaskDetailModal />);

      await user.click(screen.getByText('Test Task'));
      const input = screen.getByDisplayValue('Test Task');
      await user.clear(input);
      await user.tab();

      expect(mockMutateAsync).not.toHaveBeenCalled();
    });

    it('enters description edit mode on click', async () => {
      const user = userEvent.setup();
      useUiStore.setState({ selectedTaskId: 'task-1', isTaskDetailOpen: true });
      renderWithProviders(<TaskDetailModal />);

      await user.click(screen.getByText('Test description'));

      expect(screen.getByDisplayValue('Test description')).toBeInTheDocument();
    });

    it('saves description on blur', async () => {
      const mockMutateAsync = vi.fn().mockResolvedValue({});
      vi.mocked(tasksApi.useUpdateTask).mockReturnValue({
        mutateAsync: mockMutateAsync,
        isPending: false,
      } as unknown as ReturnType<typeof tasksApi.useUpdateTask>);

      const user = userEvent.setup();
      useUiStore.setState({ selectedTaskId: 'task-1', isTaskDetailOpen: true });
      renderWithProviders(<TaskDetailModal />);

      await user.click(screen.getByText('Test description'));
      const textarea = screen.getByDisplayValue('Test description');
      await user.clear(textarea);
      await user.type(textarea, 'Updated description');
      await user.tab();

      expect(mockMutateAsync).toHaveBeenCalledWith({
        id: 'task-1',
        data: { description: 'Updated description' },
      });
    });

    it('updates status via select', async () => {
      const mockMutateAsync = vi.fn().mockResolvedValue({});
      vi.mocked(tasksApi.useUpdateTask).mockReturnValue({
        mutateAsync: mockMutateAsync,
        isPending: false,
      } as unknown as ReturnType<typeof tasksApi.useUpdateTask>);

      const user = userEvent.setup();
      useUiStore.setState({ selectedTaskId: 'task-1', isTaskDetailOpen: true });
      renderWithProviders(<TaskDetailModal />);

      await user.selectOptions(screen.getByLabelText(/status/i), 'in_progress');

      expect(mockMutateAsync).toHaveBeenCalledWith({
        id: 'task-1',
        data: { status: 'in_progress' },
      });
    });

    it('updates priority via select', async () => {
      const mockMutateAsync = vi.fn().mockResolvedValue({});
      vi.mocked(tasksApi.useUpdateTask).mockReturnValue({
        mutateAsync: mockMutateAsync,
        isPending: false,
      } as unknown as ReturnType<typeof tasksApi.useUpdateTask>);

      const user = userEvent.setup();
      useUiStore.setState({ selectedTaskId: 'task-1', isTaskDetailOpen: true });
      renderWithProviders(<TaskDetailModal />);

      await user.selectOptions(screen.getByLabelText(/priority/i), 'high');

      expect(mockMutateAsync).toHaveBeenCalledWith({
        id: 'task-1',
        data: { priority: 'high' },
      });
    });
  });

  describe('agent panel', () => {
    it('renders agent panel section when modal is open', () => {
      useUiStore.setState({ selectedTaskId: 'task-1', isTaskDetailOpen: true });
      renderWithProviders(<TaskDetailModal />);
      expect(screen.getByText('Agent')).toBeInTheDocument();
      expect(screen.getByLabelText(/agent type/i)).toBeInTheDocument();
    });
  });

  describe('worktree panel', () => {
    it('renders worktree panel section when modal is open', () => {
      useUiStore.setState({ selectedTaskId: 'task-1', isTaskDetailOpen: true });
      renderWithProviders(<TaskDetailModal />);
      expect(screen.getByText('Worktrees')).toBeInTheDocument();
    });
  });

  describe('delete', () => {
    it('shows delete button', () => {
      useUiStore.setState({ selectedTaskId: 'task-1', isTaskDetailOpen: true });
      renderWithProviders(<TaskDetailModal />);
      expect(screen.getByRole('button', { name: /delete/i })).toBeInTheDocument();
    });

    it('shows confirmation dialog when delete is clicked', async () => {
      const user = userEvent.setup();
      useUiStore.setState({ selectedTaskId: 'task-1', isTaskDetailOpen: true });
      renderWithProviders(<TaskDetailModal />);

      await user.click(screen.getByRole('button', { name: /delete/i }));

      expect(screen.getByText(/are you sure/i)).toBeInTheDocument();
    });

    it('cancels deletion when cancel is clicked in confirmation', async () => {
      const mockDeleteMutateAsync = vi.fn().mockResolvedValue({});
      vi.mocked(tasksApi.useDeleteTask).mockReturnValue({
        mutateAsync: mockDeleteMutateAsync,
        isPending: false,
      } as unknown as ReturnType<typeof tasksApi.useDeleteTask>);

      const user = userEvent.setup();
      useUiStore.setState({ selectedTaskId: 'task-1', isTaskDetailOpen: true });
      renderWithProviders(<TaskDetailModal />);

      await user.click(screen.getByRole('button', { name: /delete/i }));
      await user.click(screen.getByRole('button', { name: /cancel/i }));

      expect(mockDeleteMutateAsync).not.toHaveBeenCalled();
      expect(screen.queryByText(/are you sure/i)).not.toBeInTheDocument();
    });

    it('deletes task and closes modal on confirm', async () => {
      const mockDeleteMutateAsync = vi.fn().mockResolvedValue({});
      vi.mocked(tasksApi.useDeleteTask).mockReturnValue({
        mutateAsync: mockDeleteMutateAsync,
        isPending: false,
      } as unknown as ReturnType<typeof tasksApi.useDeleteTask>);

      const user = userEvent.setup();
      useUiStore.setState({ selectedTaskId: 'task-1', isTaskDetailOpen: true });
      renderWithProviders(<TaskDetailModal />);

      await user.click(screen.getByRole('button', { name: /delete/i }));
      await user.click(screen.getByRole('button', { name: /confirm/i }));

      expect(mockDeleteMutateAsync).toHaveBeenCalledWith({ id: 'task-1' });
      expect(useUiStore.getState().isTaskDetailOpen).toBe(false);
    });
  });
});
