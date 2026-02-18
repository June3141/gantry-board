import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import * as agentSessionsApi from '@/api/generated/endpoints/agent-sessions/agent-sessions';
import { useAgentStore } from '@/stores/agentStore';
import { AgentPanel } from './AgentPanel';

vi.mock('@/api/generated/endpoints/agent-sessions/agent-sessions', () => ({
  useListAgentSessions: vi.fn(),
  useStartAgentSession: vi.fn(),
  useStopAgentSession: vi.fn(),
  useGetAgentSessionOutputs: vi.fn(),
}));

vi.mock('@/hooks/useAgentEvents', () => ({
  useAgentEvents: vi.fn(),
}));

const createQueryClient = () => new QueryClient({ defaultOptions: { queries: { retry: false } } });

const renderWithProviders = (ui: React.ReactElement) => {
  const queryClient = createQueryClient();
  return render(<QueryClientProvider client={queryClient}>{ui}</QueryClientProvider>);
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

    vi.mocked(agentSessionsApi.useGetAgentSessionOutputs).mockReturnValue({
      data: undefined,
      isLoading: false,
    } as unknown as ReturnType<typeof agentSessionsApi.useGetAgentSessionOutputs>);
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

  it('shows prompt textarea with label', () => {
    renderWithProviders(<AgentPanel taskId="task-1" />);
    expect(screen.getByLabelText(/prompt/i)).toBeInTheDocument();
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

    await user.type(screen.getByLabelText(/prompt/i), 'Fix the bug');
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

  it('shows error message when start fails', async () => {
    mockStartMutateAsync.mockRejectedValue(new Error('Network error'));

    const user = userEvent.setup();
    renderWithProviders(<AgentPanel taskId="task-1" />);

    await user.type(screen.getByLabelText(/prompt/i), 'Fix the bug');
    await user.click(screen.getByRole('button', { name: /start/i }));

    expect(screen.getByText(/failed to start/i)).toBeInTheDocument();
  });

  it('shows past sessions list', () => {
    vi.mocked(agentSessionsApi.useListAgentSessions).mockReturnValue({
      data: [
        {
          id: 'session-1',
          task_id: 'task-1',
          agent_type: 'claude_code',
          status: 'completed',
          created_at: '2026-01-01T00:00:00Z',
          updated_at: '2026-01-01T00:00:00Z',
        },
        {
          id: 'session-2',
          task_id: 'task-1',
          agent_type: 'gemini_cli',
          status: 'failed',
          created_at: '2026-01-02T00:00:00Z',
          updated_at: '2026-01-02T00:00:00Z',
        },
      ],
      isLoading: false,
    } as unknown as ReturnType<typeof agentSessionsApi.useListAgentSessions>);

    renderWithProviders(<AgentPanel taskId="task-1" />);
    expect(screen.getByText(/past sessions/i)).toBeInTheDocument();
  });

  it('loads historical output when clicking past session', async () => {
    vi.mocked(agentSessionsApi.useListAgentSessions).mockReturnValue({
      data: [
        {
          id: 'session-1',
          task_id: 'task-1',
          agent_type: 'claude_code',
          status: 'completed',
          created_at: '2026-01-01T00:00:00Z',
          updated_at: '2026-01-01T00:00:00Z',
        },
      ],
      isLoading: false,
    } as unknown as ReturnType<typeof agentSessionsApi.useListAgentSessions>);

    vi.mocked(agentSessionsApi.useGetAgentSessionOutputs).mockReturnValue({
      data: [
        {
          id: 1,
          session_id: 'session-1',
          sequence: 0,
          content: 'output line 1',
          created_at: '2026-01-01T00:00:00Z',
        },
        {
          id: 2,
          session_id: 'session-1',
          sequence: 1,
          content: 'output line 2',
          created_at: '2026-01-01T00:00:01Z',
        },
      ],
      isLoading: false,
    } as unknown as ReturnType<typeof agentSessionsApi.useGetAgentSessionOutputs>);

    const user = userEvent.setup();
    renderWithProviders(<AgentPanel taskId="task-1" />);

    // Click the past session to view its output (session ID is truncated to 8 chars)
    await user.click(screen.getByText('session-').closest('button')!);

    // Output viewer should be visible with loaded history content
    expect(screen.getByTestId('agent-output-container')).toBeInTheDocument();
    expect(screen.getByText('output line 1')).toBeInTheDocument();
    expect(screen.getByText('output line 2')).toBeInTheDocument();
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
