import { useCallback, useEffect, useState } from 'react';
import {
  useGetAgentSessionOutputs,
  useListAgentSessions,
  useStartAgentSession,
  useStopAgentSession,
} from '@/api/generated/endpoints/agent-sessions/agent-sessions';
import type { AgentSession, AgentSessionStatus, AgentType } from '@/api/generated/model';
import { useAgentEvents } from '@/hooks/useAgentEvents';
import { useAgentStore } from '@/stores/agentStore';
import { AgentOutputViewer } from './AgentOutputViewer';

interface AgentPanelProps {
  taskId: string;
}

const STATUS_COLORS: Record<AgentSessionStatus, string> = {
  pending: 'bg-yellow-100 text-yellow-800',
  running: 'bg-blue-100 text-blue-800',
  completed: 'bg-green-100 text-green-800',
  failed: 'bg-red-100 text-red-800',
  cancelled: 'bg-gray-100 text-gray-800',
};

const TERMINAL_STATUSES: AgentSessionStatus[] = ['completed', 'failed', 'cancelled'];

export function AgentPanel({ taskId }: AgentPanelProps) {
  const [agentType, setAgentType] = useState<AgentType>('claude_code');
  const [prompt, setPrompt] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [viewingSessionId, setViewingSessionId] = useState<string | null>(null);

  const { data: sessions } = useListAgentSessions(taskId);
  const startSession = useStartAgentSession();
  const stopSession = useStopAgentSession();

  const {
    activeSessionId,
    outputLines,
    appendOutput,
    setActiveSession,
    setOutputLines,
    setLoadingHistory,
    isLoadingHistory,
    reset,
  } = useAgentStore();

  // Derive active session from both store and server data
  const activeSession =
    sessions?.find((s) => s.id === activeSessionId) ??
    sessions?.find((s) => s.status === 'running' || s.status === 'pending');

  // Past (terminal) sessions
  const pastSessions = sessions?.filter((s) => TERMINAL_STATUSES.includes(s.status)) ?? [];

  // Fetch historical outputs for viewed session
  const { data: historicalOutputs, isLoading: isLoadingOutputs } = useGetAgentSessionOutputs(
    taskId,
    viewingSessionId ?? '',
    undefined,
    { query: { enabled: !!viewingSessionId } },
  );

  // Load historical outputs into store when data arrives
  useEffect(() => {
    if (historicalOutputs && viewingSessionId) {
      const lines = historicalOutputs.map((o) => o.content);
      setOutputLines(lines);
      setLoadingHistory(false);
    }
  }, [historicalOutputs, viewingSessionId, setOutputLines, setLoadingHistory]);

  useEffect(() => {
    setLoadingHistory(isLoadingOutputs);
  }, [isLoadingOutputs, setLoadingHistory]);

  // Sync activeSessionId from server data on mount/refresh
  useEffect(() => {
    if (activeSession && !activeSessionId) {
      setActiveSession(activeSession.id);
    } else if (!activeSession && activeSessionId) {
      reset();
    }
  }, [activeSession, activeSessionId, setActiveSession, reset]);

  const handleOutput = useCallback((text: string) => appendOutput(text), [appendOutput]);
  useAgentEvents(activeSessionId, handleOutput);

  const handleStart = async () => {
    setError(null);
    setViewingSessionId(null);
    try {
      reset();
      const result = await startSession.mutateAsync({
        taskId,
        data: { agent_type: agentType, prompt },
      });
      setActiveSession(result.session.id);
      setPrompt('');
    } catch {
      setError('Failed to start agent session. Please try again.');
    }
  };

  const handleStop = async () => {
    if (!activeSession) return;
    setError(null);
    try {
      await stopSession.mutateAsync({
        taskId,
        sessionId: activeSession.id,
      });
      reset();
    } catch {
      setError('Failed to stop agent session. Please try again.');
    }
  };

  const handleViewSession = (session: AgentSession) => {
    setViewingSessionId(session.id);
    setActiveSession(null);
    setLoadingHistory(true);
    setOutputLines([]);
  };

  const handleBackToInput = () => {
    setViewingSessionId(null);
    reset();
  };

  const isViewingHistory = !!viewingSessionId;
  const viewingSession = sessions?.find((s) => s.id === viewingSessionId);

  return (
    <div className="space-y-3">
      {error && <div className="rounded-md bg-red-50 p-3 text-sm text-red-700">{error}</div>}
      {activeSession ? (
        <div className="space-y-3">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <span className="text-sm text-gray-600">
                {activeSession.agent_type === 'claude_code' ? 'Claude Code' : 'Gemini CLI'}
              </span>
              <span
                className={`inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium ${STATUS_COLORS[activeSession.status]}`}
              >
                {activeSession.status}
              </span>
            </div>
            {!TERMINAL_STATUSES.includes(activeSession.status) && (
              <button
                type="button"
                onClick={handleStop}
                disabled={stopSession.isPending}
                className="rounded-md bg-red-600 px-3 py-1.5 text-sm text-white hover:bg-red-700 disabled:opacity-50"
              >
                Stop
              </button>
            )}
          </div>
          <AgentOutputViewer lines={outputLines} isLoading={false} />
        </div>
      ) : isViewingHistory && viewingSession ? (
        <div className="space-y-3">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <button
                type="button"
                onClick={handleBackToInput}
                className="text-sm text-blue-600 hover:text-blue-800"
              >
                Back
              </button>
              <span className="text-sm text-gray-600">
                {viewingSession.agent_type === 'claude_code' ? 'Claude Code' : 'Gemini CLI'}
              </span>
              <span
                className={`inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium ${STATUS_COLORS[viewingSession.status]}`}
              >
                {viewingSession.status}
              </span>
            </div>
          </div>
          <AgentOutputViewer lines={outputLines} isLoading={isLoadingHistory} />
        </div>
      ) : (
        <div className="space-y-3">
          <div>
            <label htmlFor="agent-type-select" className="block text-sm font-medium text-gray-700">
              Agent Type
            </label>
            <select
              id="agent-type-select"
              value={agentType}
              onChange={(e) => setAgentType(e.target.value as AgentType)}
              className="mt-1 block w-full rounded-md border border-gray-300 px-3 py-2 text-sm"
            >
              <option value="claude_code">Claude Code</option>
              <option value="gemini_cli">Gemini CLI</option>
            </select>
          </div>
          <div>
            <label
              htmlFor="agent-prompt-textarea"
              className="block text-sm font-medium text-gray-700"
            >
              Prompt
            </label>
            <textarea
              id="agent-prompt-textarea"
              value={prompt}
              onChange={(e) => setPrompt(e.target.value)}
              placeholder="Enter prompt for the agent..."
              rows={3}
              className="mt-1 block w-full rounded-md border border-gray-300 px-3 py-2 text-sm focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500"
            />
          </div>
          <button
            type="button"
            onClick={handleStart}
            disabled={!prompt.trim() || startSession.isPending}
            className="rounded-md bg-blue-600 px-4 py-2 text-sm text-white hover:bg-blue-700 disabled:opacity-50"
          >
            Start
          </button>
          {pastSessions.length > 0 && (
            <div>
              <h4 className="text-sm font-medium text-gray-700">Past Sessions</h4>
              <div className="mt-1 space-y-1">
                {pastSessions.map((session) => (
                  <button
                    key={session.id}
                    type="button"
                    onClick={() => handleViewSession(session)}
                    className="flex w-full items-center justify-between rounded-md border border-gray-200 px-3 py-2 text-left text-sm hover:bg-gray-50"
                  >
                    <span className="truncate text-gray-600">{session.id.slice(0, 8)}</span>
                    <span
                      className={`inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium ${STATUS_COLORS[session.status]}`}
                    >
                      {session.status}
                    </span>
                  </button>
                ))}
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
