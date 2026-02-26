import { useQueryClient } from '@tanstack/react-query';
import { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  getListAgentSessionsQueryKey,
  useGetAgentSessionOutputs,
  useListAgentSessions,
  usePauseAgentSession,
  useResumeAgentSession,
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
  pending: 'bg-warning/15 text-warning',
  running: 'bg-primary/15 text-primary',
  paused: 'bg-warning/25 text-warning',
  completed: 'bg-success/15 text-success',
  failed: 'bg-destructive/15 text-destructive',
  cancelled: 'bg-muted text-muted-foreground',
};

const TERMINAL_STATUSES: AgentSessionStatus[] = ['completed', 'failed', 'cancelled'];

export function AgentPanel({ taskId }: AgentPanelProps) {
  const { t } = useTranslation();
  const [agentType, setAgentType] = useState<AgentType>('claude_code');
  const [prompt, setPrompt] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [viewingSessionId, setViewingSessionId] = useState<string | null>(null);
  const queryClient = useQueryClient();

  const { data: sessions } = useListAgentSessions(taskId);
  const startSession = useStartAgentSession();
  const stopSession = useStopAgentSession();
  const pauseSession = usePauseAgentSession();
  const resumeSession = useResumeAgentSession();

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
    sessions?.find(
      (s) => s.status === 'running' || s.status === 'pending' || s.status === 'paused',
    );

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
      setError(t('agent.startFailed'));
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
      setError(t('agent.stopFailed'));
    }
  };

  const handlePause = async () => {
    if (!activeSession) return;
    setError(null);
    try {
      await pauseSession.mutateAsync({
        taskId,
        sessionId: activeSession.id,
      });
      queryClient.invalidateQueries({ queryKey: getListAgentSessionsQueryKey(taskId) });
    } catch {
      setError(t('agent.pauseFailed'));
    }
  };

  const handleResume = async () => {
    if (!activeSession) return;
    setError(null);
    try {
      await resumeSession.mutateAsync({
        taskId,
        sessionId: activeSession.id,
      });
      queryClient.invalidateQueries({ queryKey: getListAgentSessionsQueryKey(taskId) });
    } catch {
      setError(t('agent.resumeFailed'));
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
      {error && (
        <div className="rounded-md bg-destructive/10 p-3 text-sm text-destructive">{error}</div>
      )}
      {activeSession ? (
        <div className="space-y-3">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <span className="text-sm text-muted-foreground">
                {activeSession.agent_type === 'claude_code'
                  ? t('agent.claudeCode')
                  : t('agent.geminiCli')}
              </span>
              <span
                className={`inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium ${STATUS_COLORS[activeSession.status]}`}
              >
                {t(`agent.status.${activeSession.status}`)}
              </span>
            </div>
            {!TERMINAL_STATUSES.includes(activeSession.status) && (
              <div className="flex items-center gap-2">
                {activeSession.status === 'running' && (
                  <button
                    type="button"
                    onClick={handlePause}
                    disabled={pauseSession.isPending}
                    className="rounded-md bg-warning px-3 py-1.5 text-sm text-white hover:bg-warning/80 disabled:opacity-50"
                  >
                    {t('agent.pause')}
                  </button>
                )}
                {activeSession.status === 'paused' && (
                  <button
                    type="button"
                    onClick={handleResume}
                    disabled={resumeSession.isPending}
                    className="rounded-md bg-success px-3 py-1.5 text-sm text-white hover:bg-success/80 disabled:opacity-50"
                  >
                    {t('agent.resume')}
                  </button>
                )}
                <button
                  type="button"
                  onClick={handleStop}
                  disabled={stopSession.isPending}
                  className="rounded-md bg-destructive px-3 py-1.5 text-sm text-white hover:bg-destructive/80 disabled:opacity-50"
                >
                  {t('common.stop')}
                </button>
              </div>
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
                className="text-sm text-primary hover:text-primary/80"
              >
                {t('common.back')}
              </button>
              <span className="text-sm text-muted-foreground">
                {viewingSession.agent_type === 'claude_code'
                  ? t('agent.claudeCode')
                  : t('agent.geminiCli')}
              </span>
              <span
                className={`inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium ${STATUS_COLORS[viewingSession.status]}`}
              >
                {t(`agent.status.${viewingSession.status}`)}
              </span>
            </div>
          </div>
          <AgentOutputViewer lines={outputLines} isLoading={isLoadingHistory} />
        </div>
      ) : (
        <div className="space-y-3">
          <div>
            <label
              htmlFor="agent-type-select"
              className="block text-sm font-medium text-foreground"
            >
              {t('agent.agentType')}
            </label>
            <select
              id="agent-type-select"
              value={agentType}
              onChange={(e) => setAgentType(e.target.value as AgentType)}
              className="mt-1 block w-full rounded-md border border-input px-3 py-2 text-sm"
            >
              <option value="claude_code">{t('agent.claudeCode')}</option>
              <option value="gemini_cli">{t('agent.geminiCli')}</option>
            </select>
          </div>
          <div>
            <label
              htmlFor="agent-prompt-textarea"
              className="block text-sm font-medium text-foreground"
            >
              {t('agent.prompt')}
            </label>
            <textarea
              id="agent-prompt-textarea"
              value={prompt}
              onChange={(e) => setPrompt(e.target.value)}
              placeholder={t('agent.promptPlaceholder')}
              rows={3}
              className="mt-1 block w-full rounded-md border border-input px-3 py-2 text-sm focus:border-ring focus:outline-none focus:ring-1 focus:ring-ring"
            />
          </div>
          <button
            type="button"
            onClick={handleStart}
            disabled={!prompt.trim() || startSession.isPending}
            className="rounded-md bg-primary px-4 py-2 text-sm text-white hover:bg-primary/80 disabled:opacity-50"
          >
            {t('common.start')}
          </button>
          {pastSessions.length > 0 && (
            <div>
              <h4 className="text-sm font-medium text-foreground">{t('agent.pastSessions')}</h4>
              <div className="mt-1 space-y-1">
                {pastSessions.map((session) => (
                  <button
                    key={session.id}
                    type="button"
                    onClick={() => handleViewSession(session)}
                    className="flex w-full items-center justify-between rounded-md border border-border px-3 py-2 text-left text-sm hover:bg-accent"
                  >
                    <span className="truncate text-muted-foreground">{session.id.slice(0, 8)}</span>
                    <span
                      className={`inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium ${STATUS_COLORS[session.status]}`}
                    >
                      {t(`agent.status.${session.status}`)}
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
