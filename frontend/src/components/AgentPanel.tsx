import { useCallback, useState } from 'react';
import {
  useListAgentSessions,
  useStartAgentSession,
  useStopAgentSession,
} from '../api/generated/endpoints/agent-sessions/agent-sessions';
import type { AgentType } from '../api/generated/model';
import { useAgentEvents } from '../hooks/useAgentEvents';
import { useAgentStore } from '../stores/agentStore';
import { AgentOutputViewer } from './AgentOutputViewer';

interface AgentPanelProps {
  taskId: string;
}

const STATUS_COLORS: Record<string, string> = {
  pending: 'bg-yellow-100 text-yellow-800',
  running: 'bg-blue-100 text-blue-800',
  completed: 'bg-green-100 text-green-800',
  failed: 'bg-red-100 text-red-800',
  cancelled: 'bg-gray-100 text-gray-800',
};

export function AgentPanel({ taskId }: AgentPanelProps) {
  const [agentType, setAgentType] = useState<AgentType>('claude_code');
  const [prompt, setPrompt] = useState('');

  const { data: sessions } = useListAgentSessions(taskId);
  const startSession = useStartAgentSession();
  const stopSession = useStopAgentSession();

  const activeSession = sessions?.find(
    (s) => s.status === 'running' || s.status === 'pending',
  );

  const { activeSessionId, outputLines, appendOutput, setActiveSession, reset } =
    useAgentStore();

  const handleOutput = useCallback(
    (text: string) => appendOutput(text),
    [appendOutput],
  );
  useAgentEvents(activeSessionId, handleOutput);

  const handleStart = async () => {
    try {
      const result = await startSession.mutateAsync({
        taskId,
        data: { agent_type: agentType, prompt },
      });
      setActiveSession(result.session.id);
      setPrompt('');
    } catch {
      // Error handled by mutation state
    }
  };

  const handleStop = async () => {
    if (!activeSession) return;
    try {
      await stopSession.mutateAsync({
        taskId,
        sessionId: activeSession.id,
      });
      reset();
    } catch {
      // Error handled by mutation state
    }
  };

  return (
    <div className="space-y-3">
      {activeSession ? (
        <div className="space-y-3">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <span className="text-sm text-gray-600">
                {activeSession.agent_type === 'claude_code' ? 'Claude Code' : 'Gemini CLI'}
              </span>
              <span
                className={`inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium ${STATUS_COLORS[activeSession.status] ?? ''}`}
              >
                {activeSession.status}
              </span>
            </div>
            <button
              type="button"
              onClick={handleStop}
              disabled={stopSession.isPending}
              className="rounded-md bg-red-600 px-3 py-1.5 text-sm text-white hover:bg-red-700 disabled:opacity-50"
            >
              Stop
            </button>
          </div>
          <AgentOutputViewer lines={outputLines} />
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
            <textarea
              value={prompt}
              onChange={(e) => setPrompt(e.target.value)}
              placeholder="Enter prompt for the agent..."
              rows={3}
              className="block w-full rounded-md border border-gray-300 px-3 py-2 text-sm focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500"
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
        </div>
      )}
    </div>
  );
}
