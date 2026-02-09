import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import * as agentSessionsApi from '../api/generated/endpoints/agent-sessions/agent-sessions';
import { useAgentStore } from '../stores/agentStore';
import { AgentPanel } from './AgentPanel';

vi.mock('../api/generated/endpoints/agent-sessions/agent-sessions', () => ({
  useListAgentSessions: vi.fn(),
  useStartAgentSession: vi.fn(),
  useStopAgentSession: vi.fn(),
}));

vi.mock('../hooks/useAgentEvents', () => ({
  useAgentEvents: vi.fn(),
}));

const createQueryClient = () =>
  new QueryClient({ defaultOptions: { queries: { retry: false } } });

const renderWithProviders = (ui: React.ReactElement) => {
  const queryClient = createQueryClient();
  return render(
    <QueryClientProvider client={queryClient}>{ui}</QueryClientProvider>,
  );
};

describe('AgentPanel', () => {
  const mockStartMutateAsync = vi.fn();
  const mockStopMutateAsync = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    useAgentStore.getState().reset();

    vi.mocked(agentSessionsApi.useListAgentSessions).mockReturnValue({
      data: [],
      isLoading: false,
    } as unknown as ReturnType<typeof agentSessionsApi.useListAgentSessions>);

    vi.mocked(agentSessionsApi.useStartAgentSession).mockReturnValue({
      mutateAsync: mockStartMutateAsync,
      isPending: false,
    } as unknown as ReturnType<typeof agentSessionsApi.useStartAgentSession>);

    vi.mocked(agentSessionsApi.useStopAgentSession).mockReturnValue({
      mutateAsync: mockStopMutateAsync,
      isPending: false,
    } as unknown as ReturnType<typeof agentSessionsApi.useStopAgentSession>);
  });

  it('renders agent type selector', () => {
    renderWithProviders(<AgentPanel taskId="task-1" />);
    expect(screen.getByLabelText(/agent type/i)).toBeInTheDocument();
  });

  it('renders start button when no active session', () => {
    renderWithProviders(<AgentPanel taskId="task-1" />);
    expect(screen.getByRole('button', { name: /start/i })).toBeInTheDocument();
  });

  it('renders stop button when session is running', () => {
    vi.mocked(agentSessionsApi.useListAgentSessions).mockReturnValue({
      data: [
        {
          id: 'session-1',
          task_id: 'task-1',
          agent_type: 'claude_code',
          status: 'running',
          created_at: '2026-01-01T00:00:00Z',
          updated_at: '2026-01-01T00:00:00Z',
        },
      ],
      isLoading: false,
    } as unknown as ReturnType<typeof agentSessionsApi.useListAgentSessions>);

    renderWithProviders(<AgentPanel taskId="task-1" />);
    expect(screen.getByRole('button', { name: /stop/i })).toBeInTheDocument();
  });

  it('shows prompt textarea', () => {
    renderWithProviders(<AgentPanel taskId="task-1" />);
    expect(screen.getByPlaceholderText(/prompt/i)).toBeInTheDocument();
  });

  it('disables start button when prompt is empty', () => {
    renderWithProviders(<AgentPanel taskId="task-1" />);
    expect(screen.getByRole('button', { name: /start/i })).toBeDisabled();
  });

  it('calls start mutation when start is clicked with prompt', async () => {
    mockStartMutateAsync.mockResolvedValue({
      session: { id: 'session-1', status: 'running' },
    });

    const user = userEvent.setup();
    renderWithProviders(<AgentPanel taskId="task-1" />);

    await user.type(screen.getByPlaceholderText(/prompt/i), 'Fix the bug');
    await user.click(screen.getByRole('button', { name: /start/i }));

    expect(mockStartMutateAsync).toHaveBeenCalledWith({
      taskId: 'task-1',
      data: { agent_type: 'claude_code', prompt: 'Fix the bug' },
    });
  });

  it('calls stop mutation when stop is clicked', async () => {
    mockStopMutateAsync.mockResolvedValue({
      id: 'session-1',
      status: 'cancelled',
    });

    vi.mocked(agentSessionsApi.useListAgentSessions).mockReturnValue({
      data: [
        {
          id: 'session-1',
          task_id: 'task-1',
          agent_type: 'claude_code',
          status: 'running',
          created_at: '2026-01-01T00:00:00Z',
          updated_at: '2026-01-01T00:00:00Z',
        },
      ],
      isLoading: false,
    } as unknown as ReturnType<typeof agentSessionsApi.useListAgentSessions>);

    const user = userEvent.setup();
    renderWithProviders(<AgentPanel taskId="task-1" />);

    await user.click(screen.getByRole('button', { name: /stop/i }));

    expect(mockStopMutateAsync).toHaveBeenCalledWith({
      taskId: 'task-1',
      sessionId: 'session-1',
    });
  });

  it('displays session status badge', () => {
    vi.mocked(agentSessionsApi.useListAgentSessions).mockReturnValue({
      data: [
        {
          id: 'session-1',
          task_id: 'task-1',
          agent_type: 'claude_code',
          status: 'running',
          created_at: '2026-01-01T00:00:00Z',
          updated_at: '2026-01-01T00:00:00Z',
        },
      ],
      isLoading: false,
    } as unknown as ReturnType<typeof agentSessionsApi.useListAgentSessions>);

    renderWithProviders(<AgentPanel taskId="task-1" />);
    expect(screen.getByText(/running/i)).toBeInTheDocument();
  });
});
