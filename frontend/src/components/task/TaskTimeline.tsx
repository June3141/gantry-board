import { useQueryClient } from '@tanstack/react-query';
import { useCallback, useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  getListAgentSessionsQueryKey,
  useGetAgentSessionOutputs,
  useListAgentSessions,
  useStartAgentSession,
  useStopAgentSession,
} from '@/api/generated/endpoints/agent-sessions/agent-sessions';
import {
  getListCommentsQueryKey,
  useCreateComment,
  useListComments,
} from '@/api/generated/endpoints/task-comments/task-comments';
import type { AgentSession, AgentType } from '@/api/generated/model';
import { Button } from '@/components/ui/button';
import { Textarea } from '@/components/ui/textarea';
import { useAgentEvents } from '@/hooks/useAgentEvents';
import { useAgentStore } from '@/stores/agentStore';
import { useAuthStore } from '@/stores/authStore';
import { useToastStore } from '@/stores/toastStore';
import { AgentOutputViewer } from '@/components/agent/AgentOutputViewer';
import { ActivityFilter, type ActivityFilterValue } from './ActivityFilter';
import { TimelineAgentSessionItem } from './TimelineAgentSessionItem';
import { TimelineCommentItem } from './TimelineCommentItem';
import { AGENT_LABELS, mergeTimeline, STATUS_COLORS, TERMINAL_STATUSES } from './timelineUtils';

export type { TimelineItem } from './timelineUtils';
// Re-export for backward compatibility
export { mergeTimeline } from './timelineUtils';

export function TaskTimeline({ taskId }: { taskId: string }) {
  const { t } = useTranslation();
  const { data: comments, isLoading: commentsLoading } = useListComments(taskId);
  const { data: sessions, isLoading: sessionsLoading } = useListAgentSessions(taskId);
  const createComment = useCreateComment();
  const queryClient = useQueryClient();
  const currentUser = useAuthStore((s) => s.user);
  const addToast = useToastStore((s) => s.addToast);

  const [newComment, setNewComment] = useState('');
  const [activityFilter, setActivityFilter] = useState<ActivityFilterValue>('all');

  // Agent start section state
  const [agentType, setAgentType] = useState<AgentType>('claude_code');
  const [prompt, setPrompt] = useState('');
  const [agentError, setAgentError] = useState<string | null>(null);
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

  const isStartingRef = useRef(false);

  const activeSession =
    sessions?.find((s) => s.id === activeSessionId) ??
    sessions?.find((s) => s.status === 'running' || s.status === 'pending');

  useEffect(() => {
    if (activeSession && !activeSessionId) {
      setActiveSession(activeSession.id);
    } else if (!activeSession && activeSessionId && !isStartingRef.current) {
      reset();
    }
  }, [activeSession, activeSessionId, setActiveSession, reset]);

  const handleOutput = useCallback((text: string) => appendOutput(text), [appendOutput]);
  useAgentEvents(activeSessionId, handleOutput);

  // View historical session
  const [viewingSessionId, setViewingSessionId] = useState<string | null>(null);
  const { data: historicalOutputs, isLoading: isLoadingOutputs } = useGetAgentSessionOutputs(
    taskId,
    viewingSessionId ?? '',
    undefined,
    { query: { enabled: !!viewingSessionId } },
  );

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

  const invalidateSessions = () => {
    queryClient.invalidateQueries({ queryKey: getListAgentSessionsQueryKey(taskId) });
  };

  const handleStartAgent = async () => {
    setAgentError(null);
    setViewingSessionId(null);
    isStartingRef.current = true;
    try {
      reset();
      const result = await startSession.mutateAsync({
        taskId,
        data: { agent_type: agentType, prompt },
      });
      setActiveSession(result.session.id);
      setPrompt('');
      invalidateSessions();
    } catch {
      setAgentError(t('agent.startFailed'));
    } finally {
      isStartingRef.current = false;
    }
  };

  const handleStopAgent = async () => {
    if (!activeSession) return;
    try {
      await stopSession.mutateAsync({ taskId, sessionId: activeSession.id });
      reset();
      invalidateSessions();
    } catch {
      setAgentError(t('agent.stopFailed'));
    }
  };

  const handleSubmitComment = async () => {
    const trimmed = newComment.trim();
    if (!trimmed) return;
    try {
      await createComment.mutateAsync({ taskId, data: { content: trimmed } });
      setNewComment('');
      queryClient.invalidateQueries({ queryKey: getListCommentsQueryKey(taskId) });
    } catch {
      addToast('error', t('activity.commentFailed'));
    }
  };

  const handleViewSession = (session: AgentSession) => {
    setViewingSessionId(session.id);
    setActiveSession(null);
    setLoadingHistory(true);
    setOutputLines([]);
  };

  const isLoading = commentsLoading || sessionsLoading;
  const terminalSessions = sessions?.filter((s) => TERMINAL_STATUSES.includes(s.status)) ?? [];
  const allTimeline = mergeTimeline(comments ?? [], terminalSessions);
  const timeline =
    activityFilter === 'comments'
      ? allTimeline.filter((item) => item.type === 'comment')
      : allTimeline;

  return (
    <div className="space-y-4">
      {/* Active session */}
      {activeSession && !TERMINAL_STATUSES.includes(activeSession.status) && (
        <div className="space-y-2 rounded-md border border-primary/20 bg-primary/10 p-3">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <span className="text-sm text-muted-foreground">
                {AGENT_LABELS[activeSession.agent_type] ?? activeSession.agent_type}
              </span>
              <span
                className={`inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium ${STATUS_COLORS[activeSession.status]}`}
              >
                {activeSession.status}
              </span>
            </div>
            <Button
              variant="destructive"
              size="sm"
              onClick={handleStopAgent}
              disabled={stopSession.isPending}
            >
              {t('common.stop')}
            </Button>
          </div>
          <AgentOutputViewer lines={outputLines} isLoading={false} />
        </div>
      )}

      {/* Viewing historical session */}
      {viewingSessionId &&
        (() => {
          const viewingSession = sessions?.find((s) => s.id === viewingSessionId);
          if (!viewingSession) return null;
          return (
            <div className="space-y-2 rounded-md border border-border p-3">
              <div className="flex items-center gap-2">
                <Button
                  variant="link"
                  size="sm"
                  onClick={() => {
                    setViewingSessionId(null);
                    reset();
                  }}
                >
                  {t('common.back')}
                </Button>
                <span className="text-sm text-muted-foreground">
                  {AGENT_LABELS[viewingSession.agent_type] ?? viewingSession.agent_type}
                </span>
                <span
                  className={`inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium ${STATUS_COLORS[viewingSession.status]}`}
                >
                  {viewingSession.status}
                </span>
              </div>
              <AgentOutputViewer lines={outputLines} isLoading={isLoadingHistory} />
            </div>
          );
        })()}

      {/* Agent start section */}
      {!activeSession && !viewingSessionId && (
        <div className="space-y-2 rounded-md border border-border p-3">
          {agentError && (
            <div className="rounded-md bg-destructive/10 p-2 text-sm text-destructive">
              {agentError}
            </div>
          )}
          <div className="grid grid-cols-2 gap-2">
            <div>
              <label
                htmlFor="timeline-agent-type"
                className="block text-sm font-medium text-foreground"
              >
                {t('agent.agentType')}
              </label>
              <select
                id="timeline-agent-type"
                value={agentType}
                onChange={(e) => setAgentType(e.target.value as AgentType)}
                className="mt-1 block w-full rounded-md border border-input px-3 py-2 text-sm"
              >
                <option value="claude_code">{t('agent.claudeCode')}</option>
                <option value="gemini_cli">{t('agent.geminiCli')}</option>
              </select>
            </div>
            <div className="flex items-end">
              <Button
                onClick={handleStartAgent}
                disabled={!prompt.trim() || startSession.isPending}
              >
                {t('common.start')}
              </Button>
            </div>
          </div>
          <Textarea
            value={prompt}
            onChange={(e) => setPrompt(e.target.value)}
            placeholder={t('agent.promptPlaceholder')}
            rows={2}
          />
        </div>
      )}

      {/* Comment input */}
      <div className="flex gap-2">
        <Textarea
          value={newComment}
          onChange={(e) => setNewComment(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
              e.preventDefault();
              handleSubmitComment();
            }
          }}
          placeholder={t('activity.commentPlaceholder')}
          rows={2}
          className="flex-1"
        />
        <Button
          onClick={handleSubmitComment}
          disabled={!newComment.trim() || createComment.isPending}
          className="self-end"
        >
          {t('common.post')}
        </Button>
      </div>

      {/* Activity filter */}
      <ActivityFilter value={activityFilter} onChange={setActivityFilter} />

      {/* Timeline */}
      {isLoading ? (
        <p className="text-sm text-muted-foreground">{t('activity.loadingActivity')}</p>
      ) : timeline.length === 0 ? (
        <p className="text-sm text-muted-foreground">{t('activity.noActivity')}</p>
      ) : (
        <div className="divide-y">
          {timeline.map((item) =>
            item.type === 'comment' ? (
              <TimelineCommentItem
                key={item.data.id}
                comment={item.data}
                taskId={taskId}
                isOwner={currentUser?.id === item.data.user_id}
              />
            ) : (
              <TimelineAgentSessionItem
                key={item.data.id}
                session={item.data}
                onView={handleViewSession}
              />
            ),
          )}
        </div>
      )}
    </div>
  );
}
