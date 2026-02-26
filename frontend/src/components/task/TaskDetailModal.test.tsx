import { screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import * as tasksApi from '@/api/generated/endpoints/tasks/tasks';
import { useUiStore } from '@/stores/uiStore';
import { TaskDetailModal } from './TaskDetailModal';
import { mockTask, renderWithProviders, setupMocks } from './taskDetailModalSetup';

vi.mock('@/api/generated/endpoints/tasks/tasks', () => ({
  useGetTask: vi.fn(),
  useUpdateTask: vi.fn(),
  useDeleteTask: vi.fn(),
}));

vi.mock('@/api/generated/endpoints/agent-sessions/agent-sessions', () => ({
  useListAgentSessions: vi.fn(),
  useStartAgentSession: vi.fn(),
  useStopAgentSession: vi.fn(),
  useGetAgentSessionOutputs: vi.fn(),
}));

vi.mock('@/api/generated/endpoints/worktrees/worktrees', () => ({
  useListProjectWorktrees: vi.fn(),
  useCreateProjectWorktree: vi.fn(),
  useDeleteProjectWorktree: vi.fn(),
  getListProjectWorktreesQueryKey: vi.fn(() => ['worktrees']),
}));

vi.mock('@/api/generated/endpoints/project-members/project-members', () => ({
  useListMembers: vi.fn(),
}));

vi.mock('@/api/generated/endpoints/task-comments/task-comments', () => ({
  useListComments: vi.fn(),
  useCreateComment: vi.fn(),
  useUpdateComment: vi.fn(),
  useDeleteComment: vi.fn(),
}));

vi.mock('@/hooks/useAgentEvents', () => ({
  useAgentEvents: vi.fn(),
}));

describe('TaskDetailModal', () => {
  beforeEach(() => {
    setupMocks();
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

  describe('activity', () => {
    it('renders activity section when modal is open', () => {
      useUiStore.setState({ selectedTaskId: 'task-1', isTaskDetailOpen: true });
      renderWithProviders(<TaskDetailModal />);
      expect(screen.getByText('Activity')).toBeInTheDocument();
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
});
