import { screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import * as tasksApi from '@/api/generated/endpoints/tasks/tasks';
import { useUiStore } from '@/stores/uiStore';
import { mockTask, renderWithProviders, setupMocks } from './taskDetailModalSetup';
import { TaskDetailModal } from './TaskDetailModal';

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
  useListWorktrees: vi.fn(),
  useCreateWorktree: vi.fn(),
  useDeleteWorktree: vi.fn(),
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

  describe('assignee', () => {
    it('displays assignee select', () => {
      useUiStore.setState({ selectedTaskId: 'task-1', isTaskDetailOpen: true });
      renderWithProviders(<TaskDetailModal />);
      expect(screen.getByLabelText(/assignee/i)).toBeInTheDocument();
    });

    it('shows current assignee in select', () => {
      vi.mocked(tasksApi.useGetTask).mockReturnValue({
        data: { ...mockTask, assigned_to: 'user-1' },
        isLoading: false,
        isError: false,
      } as unknown as ReturnType<typeof tasksApi.useGetTask>);
      useUiStore.setState({ selectedTaskId: 'task-1', isTaskDetailOpen: true });
      renderWithProviders(<TaskDetailModal />);
      const select = screen.getByLabelText(/assignee/i) as HTMLSelectElement;
      expect(select.value).toBe('user-1');
    });

    it('has Unassigned option', () => {
      useUiStore.setState({ selectedTaskId: 'task-1', isTaskDetailOpen: true });
      renderWithProviders(<TaskDetailModal />);
      const select = screen.getByLabelText(/assignee/i) as HTMLSelectElement;
      const options = Array.from(select.options);
      expect(options.some((o) => o.text === 'Unassigned')).toBe(true);
    });

    it('lists all project members', () => {
      useUiStore.setState({ selectedTaskId: 'task-1', isTaskDetailOpen: true });
      renderWithProviders(<TaskDetailModal />);
      const select = screen.getByLabelText(/assignee/i) as HTMLSelectElement;
      const options = Array.from(select.options);
      expect(options.some((o) => o.text === 'Alice')).toBe(true);
      expect(options.some((o) => o.text === 'Bob')).toBe(true);
    });

    it('calls updateTask when assignee is changed', async () => {
      const mockMutateAsync = vi.fn().mockResolvedValue({});
      vi.mocked(tasksApi.useUpdateTask).mockReturnValue({
        mutateAsync: mockMutateAsync,
        isPending: false,
      } as unknown as ReturnType<typeof tasksApi.useUpdateTask>);

      const user = userEvent.setup();
      useUiStore.setState({ selectedTaskId: 'task-1', isTaskDetailOpen: true });
      renderWithProviders(<TaskDetailModal />);
      await user.selectOptions(screen.getByLabelText(/assignee/i), 'user-2');

      expect(mockMutateAsync).toHaveBeenCalledWith({
        id: 'task-1',
        data: { assigned_to: 'user-2' },
      });
    });

    it('sends null when Unassigned is selected', async () => {
      const mockMutateAsync = vi.fn().mockResolvedValue({});
      vi.mocked(tasksApi.useUpdateTask).mockReturnValue({
        mutateAsync: mockMutateAsync,
        isPending: false,
      } as unknown as ReturnType<typeof tasksApi.useUpdateTask>);
      vi.mocked(tasksApi.useGetTask).mockReturnValue({
        data: { ...mockTask, assigned_to: 'user-1' },
        isLoading: false,
        isError: false,
      } as unknown as ReturnType<typeof tasksApi.useGetTask>);

      const user = userEvent.setup();
      useUiStore.setState({ selectedTaskId: 'task-1', isTaskDetailOpen: true });
      renderWithProviders(<TaskDetailModal />);
      await user.selectOptions(screen.getByLabelText(/assignee/i), '');

      expect(mockMutateAsync).toHaveBeenCalledWith({
        id: 'task-1',
        data: { assigned_to: null },
      });
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
