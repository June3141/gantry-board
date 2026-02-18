import { useQueryClient } from '@tanstack/react-query';
import { useCallback, useEffect, useRef, useState } from 'react';
import {
  getListAgentSessionsQueryKey,
  useGetAgentSessionOutputs,
  useListAgentSessions,
  useStartAgentSession,
  useStopAgentSession,
} from '../api/generated/endpoints/agent-sessions/agent-sessions';
import {
  getListCommentsQueryKey,
  useCreateComment,
  useListComments,
} from '../api/generated/endpoints/task-comments/task-comments';
import type { AgentSession, AgentType } from '../api/generated/model';
import { useAgentEvents } from '../hooks/useAgentEvents';
import { useAgentStore } from '../stores/agentStore';
import { useAuthStore } from '../stores/authStore';
import { useToastStore } from '../stores/toastStore';
import { AgentOutputViewer } from './AgentOutputViewer';
import { TimelineAgentSessionItem } from './TimelineAgentSessionItem';
import { TimelineCommentItem } from './TimelineCommentItem';
import { AGENT_LABELS, mergeTimeline, STATUS_COLORS, TERMINAL_STATUSES } from './timelineUtils';

export type { TimelineItem } from './timelineUtils';
// Re-export for backward compatibility
export { mergeTimeline } from './timelineUtils';

export function TaskTimeline({ taskId }: { taskId: string }) {
  const { data: comments, isLoading: commentsLoading } = useListComments(taskId);
  const { data: sessions, isLoading: sessionsLoading } = useListAgentSessions(taskId);
  const createComment = useCreateComment();
  const queryClient = useQueryClient();
  const currentUser = useAuthStore((s) => s.user);
  const addToast = useToastStore((s) => s.addToast);

  const [newComment, setNewComment] = useState('');

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
      setAgentError('Failed to start agent session.');
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
      setAgentError('Failed to stop agent session.');
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
      addToast('error', 'Failed to post comment.');
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
  const timeline = mergeTimeline(comments ?? [], terminalSessions);

  return (
    <div className="space-y-4">
      {/* Active session */}
      {activeSession && !TERMINAL_STATUSES.includes(activeSession.status) && (
        <div className="space-y-2 rounded-md border border-blue-200 bg-blue-50 p-3">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <span className="text-sm text-gray-600">
                {AGENT_LABELS[activeSession.agent_type] ?? activeSession.agent_type}
              </span>
              <span
                className={`inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium ${STATUS_COLORS[activeSession.status]}`}
              >
                {activeSession.status}
              </span>
            </div>
            <button
              type="button"
              onClick={handleStopAgent}
              disabled={stopSession.isPending}
              className="rounded-md bg-red-600 px-3 py-1.5 text-sm text-white hover:bg-red-700 disabled:opacity-50"
            >
              Stop
            </button>
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
            <div className="space-y-2 rounded-md border border-gray-200 p-3">
              <div className="flex items-center gap-2">
                <button
                  type="button"
                  onClick={() => {
                    setViewingSessionId(null);
                    reset();
                  }}
                  className="text-sm text-blue-600 hover:text-blue-800"
                >
                  Back
                </button>
                <span className="text-sm text-gray-600">
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
        <div className="space-y-2 rounded-md border border-gray-200 p-3">
          {agentError && (
            <div className="rounded-md bg-red-50 p-2 text-sm text-red-700">{agentError}</div>
          )}
          <div className="grid grid-cols-2 gap-2">
            <div>
              <label
                htmlFor="timeline-agent-type"
                className="block text-sm font-medium text-gray-700"
              >
                Agent Type
              </label>
              <select
                id="timeline-agent-type"
                value={agentType}
                onChange={(e) => setAgentType(e.target.value as AgentType)}
                className="mt-1 block w-full rounded-md border border-gray-300 px-3 py-2 text-sm"
              >
                <option value="claude_code">Claude Code</option>
                <option value="gemini_cli">Gemini CLI</option>
              </select>
            </div>
            <div className="flex items-end">
              <button
                type="button"
                onClick={handleStartAgent}
                disabled={!prompt.trim() || startSession.isPending}
                className="rounded-md bg-blue-600 px-4 py-2 text-sm text-white hover:bg-blue-700 disabled:opacity-50"
              >
                Start
              </button>
            </div>
          </div>
          <textarea
            value={prompt}
            onChange={(e) => setPrompt(e.target.value)}
            placeholder="Enter prompt for the agent..."
            rows={2}
            className="block w-full rounded-md border border-gray-300 px-3 py-2 text-sm focus:border-blue-500 focus:ring-1 focus:ring-blue-500"
          />
        </div>
      )}

      {/* Comment input */}
      <div className="flex gap-2">
        <textarea
          value={newComment}
          onChange={(e) => setNewComment(e.target.value)}
          placeholder="Add a comment..."
          rows={2}
          className="flex-1 rounded border border-gray-300 px-3 py-2 text-sm focus:border-blue-500 focus:ring-1 focus:ring-blue-500"
        />
        <button
          type="button"
          onClick={handleSubmitComment}
          disabled={!newComment.trim() || createComment.isPending}
          className="self-end rounded bg-blue-600 px-3 py-2 text-sm text-white hover:bg-blue-700 disabled:opacity-50"
        >
          Post
        </button>
      </div>

      {/* Timeline */}
      {isLoading ? (
        <p className="text-sm text-gray-500">Loading activity...</p>
      ) : timeline.length === 0 ? (
        <p className="text-sm text-gray-500">No activity yet.</p>
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
