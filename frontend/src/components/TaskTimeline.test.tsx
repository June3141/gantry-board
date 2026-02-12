import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import * as agentSessionsApi from '../api/generated/endpoints/agent-sessions/agent-sessions';
import * as commentsApi from '../api/generated/endpoints/task-comments/task-comments';
import type { AgentSession, TaskComment } from '../api/generated/model';
import { AgentSessionStatus, AgentType } from '../api/generated/model';
import { useAuthStore } from '../stores/authStore';
import { mergeTimeline, TaskTimeline, type TimelineItem } from './TaskTimeline';

vi.mock('../api/generated/endpoints/task-comments/task-comments', () => ({
  useListComments: vi.fn(),
  useCreateComment: vi.fn(),
  useUpdateComment: vi.fn(),
  useDeleteComment: vi.fn(),
  getListCommentsQueryKey: vi.fn(() => ['comments']),
}));

vi.mock('../api/generated/endpoints/agent-sessions/agent-sessions', () => ({
  useListAgentSessions: vi.fn(),
  useStartAgentSession: vi.fn(),
  useStopAgentSession: vi.fn(),
  useGetAgentSessionOutputs: vi.fn(),
  getListAgentSessionsQueryKey: vi.fn(() => ['agent-sessions']),
}));

vi.mock('../hooks/useAgentEvents', () => ({
  useAgentEvents: vi.fn(),
}));

const createComment = (overrides: Partial<TaskComment> = {}): TaskComment => ({
  id: 'comment-1',
  task_id: 'task-1',
  user_id: 'user-1',
  user_name: 'Alice',
  content: 'Test comment',
  created_at: '2026-01-01T12:00:00Z',
  updated_at: '2026-01-01T12:00:00Z',
  ...overrides,
});

const createSession = (overrides: Partial<AgentSession> = {}): AgentSession => ({
  id: 'session-1',
  task_id: 'task-1',
  agent_type: AgentType.claude_code,
  status: AgentSessionStatus.completed,
  created_at: '2026-01-01T10:00:00Z',
  updated_at: '2026-01-01T10:00:00Z',
  ...overrides,
});

describe('mergeTimeline', () => {
  it('merges comments and sessions sorted by created_at descending', () => {
    const comments = [
      createComment({ id: 'c1', created_at: '2026-01-01T12:00:00Z' }),
      createComment({ id: 'c2', created_at: '2026-01-01T08:00:00Z' }),
    ];
    const sessions = [createSession({ id: 's1', created_at: '2026-01-01T10:00:00Z' })];

    const result = mergeTimeline(comments, sessions);

    expect(result).toHaveLength(3);
    expect(result[0]).toEqual({ type: 'comment', data: comments[0] });
    expect(result[1]).toEqual({ type: 'agent_session', data: sessions[0] });
    expect(result[2]).toEqual({ type: 'comment', data: comments[1] });
  });

  it('returns only sessions when comments are empty', () => {
    const sessions = [createSession({ id: 's1' })];
    const result = mergeTimeline([], sessions);

    expect(result).toHaveLength(1);
    expect(result[0].type).toBe('agent_session');
  });

  it('returns only comments when sessions are empty', () => {
    const comments = [createComment({ id: 'c1' })];
    const result = mergeTimeline(comments, []);

    expect(result).toHaveLength(1);
    expect(result[0].type).toBe('comment');
  });

  it('returns empty array when both are empty', () => {
    expect(mergeTimeline([], [])).toEqual([]);
  });

  it('has correct discriminator types', () => {
    const comments = [createComment({ id: 'c1', created_at: '2026-01-01T12:00:00Z' })];
    const sessions = [createSession({ id: 's1', created_at: '2026-01-01T10:00:00Z' })];

    const result = mergeTimeline(comments, sessions);

    const commentItem = result.find(
      (item): item is TimelineItem & { type: 'comment' } => item.type === 'comment',
    );
    const sessionItem = result.find(
      (item): item is TimelineItem & { type: 'agent_session' } => item.type === 'agent_session',
    );

    expect(commentItem?.data.content).toBe('Test comment');
    expect(sessionItem?.data.agent_type).toBe(AgentType.claude_code);
  });
});

const createQueryClient = () => new QueryClient({ defaultOptions: { queries: { retry: false } } });

const renderWithProviders = (ui: React.ReactElement) => {
  const queryClient = createQueryClient();
  return render(<QueryClientProvider client={queryClient}>{ui}</QueryClientProvider>);
};

describe('TaskTimeline component', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useAuthStore.setState({
      user: {
        id: 'user-1',
        name: 'Alice',
        email: 'alice@test.com',
        created_at: '2026-01-01T00:00:00Z',
        updated_at: '2026-01-01T00:00:00Z',
      },
    });

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
  });

  it('displays comment with user name and content', () => {
    const comments = [createComment({ id: 'c1', user_name: 'Alice', content: 'Hello world' })];
    vi.mocked(commentsApi.useListComments).mockReturnValue({
      data: comments,
      isLoading: false,
    } as unknown as ReturnType<typeof commentsApi.useListComments>);
    vi.mocked(agentSessionsApi.useListAgentSessions).mockReturnValue({
      data: [],
      isLoading: false,
    } as unknown as ReturnType<typeof agentSessionsApi.useListAgentSessions>);

    renderWithProviders(<TaskTimeline taskId="task-1" />);

    expect(screen.getByText('Alice')).toBeInTheDocument();
    expect(screen.getByText('Hello world')).toBeInTheDocument();
  });

  it('displays agent session with type and status', () => {
    vi.mocked(commentsApi.useListComments).mockReturnValue({
      data: [],
      isLoading: false,
    } as unknown as ReturnType<typeof commentsApi.useListComments>);
    vi.mocked(agentSessionsApi.useListAgentSessions).mockReturnValue({
      data: [
        createSession({
          id: 's1',
          agent_type: AgentType.claude_code,
          status: AgentSessionStatus.completed,
        }),
      ],
      isLoading: false,
    } as unknown as ReturnType<typeof agentSessionsApi.useListAgentSessions>);

    renderWithProviders(<TaskTimeline taskId="task-1" />);

    const sessionItem = screen.getByTestId('timeline-session');
    expect(sessionItem).toHaveTextContent('Claude Code');
    expect(sessionItem).toHaveTextContent('completed');
  });

  it('shows edit/delete for own comments', () => {
    vi.mocked(commentsApi.useListComments).mockReturnValue({
      data: [createComment({ id: 'c1', user_id: 'user-1' })],
      isLoading: false,
    } as unknown as ReturnType<typeof commentsApi.useListComments>);
    vi.mocked(agentSessionsApi.useListAgentSessions).mockReturnValue({
      data: [],
      isLoading: false,
    } as unknown as ReturnType<typeof agentSessionsApi.useListAgentSessions>);

    renderWithProviders(<TaskTimeline taskId="task-1" />);

    expect(screen.getByLabelText('Edit')).toBeInTheDocument();
    expect(screen.getByLabelText('Delete')).toBeInTheDocument();
  });

  it('hides edit/delete for other users comments', () => {
    vi.mocked(commentsApi.useListComments).mockReturnValue({
      data: [createComment({ id: 'c1', user_id: 'user-2', user_name: 'Bob' })],
      isLoading: false,
    } as unknown as ReturnType<typeof commentsApi.useListComments>);
    vi.mocked(agentSessionsApi.useListAgentSessions).mockReturnValue({
      data: [],
      isLoading: false,
    } as unknown as ReturnType<typeof agentSessionsApi.useListAgentSessions>);

    renderWithProviders(<TaskTimeline taskId="task-1" />);

    expect(screen.queryByLabelText('Edit')).not.toBeInTheDocument();
    expect(screen.queryByLabelText('Delete')).not.toBeInTheDocument();
  });

  it('displays comment input form', () => {
    vi.mocked(commentsApi.useListComments).mockReturnValue({
      data: [],
      isLoading: false,
    } as unknown as ReturnType<typeof commentsApi.useListComments>);
    vi.mocked(agentSessionsApi.useListAgentSessions).mockReturnValue({
      data: [],
      isLoading: false,
    } as unknown as ReturnType<typeof agentSessionsApi.useListAgentSessions>);

    renderWithProviders(<TaskTimeline taskId="task-1" />);

    expect(screen.getByPlaceholderText(/add a comment/i)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /post/i })).toBeInTheDocument();
  });

  it('displays agent start section', () => {
    vi.mocked(commentsApi.useListComments).mockReturnValue({
      data: [],
      isLoading: false,
    } as unknown as ReturnType<typeof commentsApi.useListComments>);
    vi.mocked(agentSessionsApi.useListAgentSessions).mockReturnValue({
      data: [],
      isLoading: false,
    } as unknown as ReturnType<typeof agentSessionsApi.useListAgentSessions>);

    renderWithProviders(<TaskTimeline taskId="task-1" />);

    expect(screen.getByLabelText(/agent type/i)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /start/i })).toBeInTheDocument();
  });

  it('shows empty state when no items', () => {
    vi.mocked(commentsApi.useListComments).mockReturnValue({
      data: [],
      isLoading: false,
    } as unknown as ReturnType<typeof commentsApi.useListComments>);
    vi.mocked(agentSessionsApi.useListAgentSessions).mockReturnValue({
      data: [],
      isLoading: false,
    } as unknown as ReturnType<typeof agentSessionsApi.useListAgentSessions>);

    renderWithProviders(<TaskTimeline taskId="task-1" />);

    expect(screen.getByText(/no activity/i)).toBeInTheDocument();
  });

  it('submits a new comment', async () => {
    const mockCreate = vi.fn().mockResolvedValue({});
    vi.mocked(commentsApi.useCreateComment).mockReturnValue({
      mutateAsync: mockCreate,
      isPending: false,
    } as unknown as ReturnType<typeof commentsApi.useCreateComment>);
    vi.mocked(commentsApi.useListComments).mockReturnValue({
      data: [],
      isLoading: false,
    } as unknown as ReturnType<typeof commentsApi.useListComments>);
    vi.mocked(agentSessionsApi.useListAgentSessions).mockReturnValue({
      data: [],
      isLoading: false,
    } as unknown as ReturnType<typeof agentSessionsApi.useListAgentSessions>);

    const user = userEvent.setup();
    renderWithProviders(<TaskTimeline taskId="task-1" />);

    await user.type(screen.getByPlaceholderText(/add a comment/i), 'New comment');
    await user.click(screen.getByRole('button', { name: /post/i }));

    expect(mockCreate).toHaveBeenCalledWith({
      taskId: 'task-1',
      data: { content: 'New comment' },
    });
  });

  it('starts agent session when Start is clicked', async () => {
    const mockStart = vi.fn().mockResolvedValue({ session: { id: 'new-session' } });
    vi.mocked(agentSessionsApi.useStartAgentSession).mockReturnValue({
      mutateAsync: mockStart,
      isPending: false,
    } as unknown as ReturnType<typeof agentSessionsApi.useStartAgentSession>);
    vi.mocked(commentsApi.useListComments).mockReturnValue({
      data: [],
      isLoading: false,
    } as unknown as ReturnType<typeof commentsApi.useListComments>);
    vi.mocked(agentSessionsApi.useListAgentSessions).mockReturnValue({
      data: [],
      isLoading: false,
    } as unknown as ReturnType<typeof agentSessionsApi.useListAgentSessions>);

    const user = userEvent.setup();
    renderWithProviders(<TaskTimeline taskId="task-1" />);

    await user.type(screen.getByPlaceholderText(/enter prompt/i), 'Fix the bug');
    await user.click(screen.getByRole('button', { name: /start/i }));

    expect(mockStart).toHaveBeenCalledWith({
      taskId: 'task-1',
      data: { agent_type: 'claude_code', prompt: 'Fix the bug' },
    });
  });

  it('session items are clickable to view outputs', async () => {
    vi.mocked(commentsApi.useListComments).mockReturnValue({
      data: [],
      isLoading: false,
    } as unknown as ReturnType<typeof commentsApi.useListComments>);
    vi.mocked(agentSessionsApi.useListAgentSessions).mockReturnValue({
      data: [
        createSession({
          id: 's1',
          agent_type: AgentType.claude_code,
          status: AgentSessionStatus.completed,
        }),
      ],
      isLoading: false,
    } as unknown as ReturnType<typeof agentSessionsApi.useListAgentSessions>);

    const user = userEvent.setup();
    renderWithProviders(<TaskTimeline taskId="task-1" />);

    const sessionItem = screen.getByTestId('timeline-session');
    expect(sessionItem.tagName).toBe('BUTTON');

    await user.click(sessionItem);

    // After clicking, the historical viewer should be shown with a Back button
    expect(screen.getByText('Back')).toBeInTheDocument();
  });
});
