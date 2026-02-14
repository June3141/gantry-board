import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render } from '@testing-library/react';
import React from 'react';
import { vi } from 'vitest';
import * as agentSessionsApi from '../../api/generated/endpoints/agent-sessions/agent-sessions';
import * as membersApi from '../../api/generated/endpoints/project-members/project-members';
import * as commentsApi from '../../api/generated/endpoints/task-comments/task-comments';
import * as tasksApi from '../../api/generated/endpoints/tasks/tasks';
import * as worktreesApi from '../../api/generated/endpoints/worktrees/worktrees';
import type { ProjectMember, Task } from '../../api/generated/model';
import { MemberRole, TaskPriority, TaskStatus } from '../../api/generated/model';
import { useUiStore } from '../../stores/uiStore';

export const mockMembers: ProjectMember[] = [
  {
    user_id: 'user-1',
    user_name: 'Alice',
    user_email: 'alice@test.com',
    role: MemberRole.owner,
    project_id: 'project-1',
    created_at: '2026-01-01T00:00:00Z',
  },
  {
    user_id: 'user-2',
    user_name: 'Bob',
    user_email: 'bob@test.com',
    role: MemberRole.member,
    project_id: 'project-1',
    created_at: '2026-01-01T00:00:00Z',
  },
];

export const mockTask: Task = {
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

export const renderWithProviders = (ui: React.ReactElement) => {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(React.createElement(QueryClientProvider, { client: queryClient }, ui));
};

export function setupMocks() {
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
  useUiStore.setState({ selectedTaskId: null, isTaskDetailOpen: false });
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
  vi.mocked(membersApi.useListMembers).mockReturnValue({
    data: mockMembers,
    isLoading: false,
  } as unknown as ReturnType<typeof membersApi.useListMembers>);
}
