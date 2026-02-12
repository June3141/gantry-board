import { useQueryClient } from '@tanstack/react-query';
import { useCallback, useEffect, useState } from 'react';
import {
  useGetAgentSessionOutputs,
  useListAgentSessions,
  useStartAgentSession,
  useStopAgentSession,
} from '../api/generated/endpoints/agent-sessions/agent-sessions';
import {
  getListCommentsQueryKey,
  useCreateComment,
  useDeleteComment,
  useListComments,
  useUpdateComment,
} from '../api/generated/endpoints/task-comments/task-comments';
import type { AgentSession, AgentSessionStatus, AgentType, TaskComment } from '../api/generated/model';
import { useAgentEvents } from '../hooks/useAgentEvents';
import { useAgentStore } from '../stores/agentStore';
import { useAuthStore } from '../stores/authStore';
import { useToastStore } from '../stores/toastStore';
import { AgentOutputViewer } from './AgentOutputViewer';

export type TimelineItem =
  | { type: 'comment'; data: TaskComment }
  | { type: 'agent_session'; data: AgentSession };

export function mergeTimeline(
  comments: TaskComment[],
  sessions: AgentSession[],
): TimelineItem[] {
  const items: TimelineItem[] = [
    ...comments.map((c) => ({ type: 'comment' as const, data: c })),
    ...sessions.map((s) => ({ type: 'agent_session' as const, data: s })),
  ];
  items.sort(
    (a, b) => new Date(b.data.created_at).getTime() - new Date(a.data.created_at).getTime(),
  );
  return items;
}

const STATUS_COLORS: Record<AgentSessionStatus, string> = {
  pending: 'bg-yellow-100 text-yellow-800',
  running: 'bg-blue-100 text-blue-800',
  completed: 'bg-green-100 text-green-800',
  failed: 'bg-red-100 text-red-800',
  cancelled: 'bg-gray-100 text-gray-800',
};

const AGENT_LABELS: Record<AgentType, string> = {
  claude_code: 'Claude Code',
  gemini_cli: 'Gemini CLI',
};

const TERMINAL_STATUSES: AgentSessionStatus[] = ['completed', 'failed', 'cancelled'];

function timeAgo(dateStr: string): string {
  const now = Date.now();
  const then = new Date(dateStr).getTime();
  const diffSec = Math.floor((now - then) / 1000);
  if (diffSec < 60) return 'just now';
  const diffMin = Math.floor(diffSec / 60);
  if (diffMin < 60) return `${diffMin} min ago`;
  const diffHr = Math.floor(diffMin / 60);
  if (diffHr < 24) return `${diffHr}h ago`;
  const diffDay = Math.floor(diffHr / 24);
  return `${diffDay}d ago`;
}

function getInitials(name: string): string {
  return name
    .split(' ')
    .map((w) => w[0])
    .filter(Boolean)
    .slice(0, 2)
    .join('')
    .toUpperCase();
}

function TimelineCommentItem({
  comment,
  taskId,
  isOwner,
}: {
  comment: TaskComment;
  taskId: string;
  isOwner: boolean;
}) {
  const [editing, setEditing] = useState(false);
  const [editValue, setEditValue] = useState('');
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);
  const queryClient = useQueryClient();
  const updateComment = useUpdateComment();
  const deleteComment = useDeleteComment();
  const addToast = useToastStore((s) => s.addToast);

  const invalidateComments = () => {
    queryClient.invalidateQueries({ queryKey: getListCommentsQueryKey(taskId) });
  };

  const handleSave = async () => {
    const trimmed = editValue.trim();
    if (!trimmed) return;
    try {
      await updateComment.mutateAsync({
        taskId,
        commentId: comment.id,
        data: { content: trimmed },
      });
      setEditing(false);
      invalidateComments();
    } catch {
      addToast('error', 'Failed to update comment.');
    }
  };

  const handleDelete = async () => {
    try {
      await deleteComment.mutateAsync({ taskId, commentId: comment.id });
      setShowDeleteConfirm(false);
      invalidateComments();
    } catch {
      addToast('error', 'Failed to delete comment.');
    }
  };

  return (
    <div data-testid="timeline-comment" className="flex gap-3 py-2">
      <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-gray-200 text-xs font-medium text-gray-600">
        {getInitials(comment.user_name)}
      </div>
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <span className="text-sm font-medium text-gray-900">{comment.user_name}</span>
          <span className="text-xs text-gray-500">{timeAgo(comment.created_at)}</span>
          {isOwner && !editing && !showDeleteConfirm && (
            <div className="ml-auto flex gap-1">
              <button
                type="button"
                aria-label="Edit"
                onClick={() => {
                  setEditing(true);
                  setEditValue(comment.content);
                }}
                className="text-xs text-gray-400 hover:text-gray-600"
              >
                Edit
              </button>
              <button
                type="button"
                aria-label="Delete"
                onClick={() => setShowDeleteConfirm(true)}
                className="text-xs text-gray-400 hover:text-red-600"
              >
                Delete
              </button>
            </div>
          )}
        </div>
        {editing ? (
          <div className="mt-1 space-y-1">
            <textarea
              value={editValue}
              onChange={(e) => setEditValue(e.target.value)}
              rows={2}
              className="block w-full rounded border border-gray-300 px-2 py-1 text-sm focus:border-blue-500 focus:ring-1 focus:ring-blue-500"
            />
            <div className="flex gap-1">
              <button
                type="button"
                onClick={handleSave}
                className="rounded bg-blue-600 px-2 py-0.5 text-xs text-white hover:bg-blue-700"
              >
                Save
              </button>
              <button
                type="button"
                onClick={() => setEditing(false)}
                className="rounded border border-gray-300 px-2 py-0.5 text-xs text-gray-700 hover:bg-gray-50"
              >
                Cancel
              </button>
            </div>
          </div>
        ) : showDeleteConfirm ? (
          <div className="mt-1 flex items-center gap-2 rounded bg-red-50 px-2 py-1">
            <span className="text-xs text-red-700">Delete this comment?</span>
            <button
              type="button"
              onClick={handleDelete}
              className="rounded bg-red-600 px-2 py-0.5 text-xs text-white hover:bg-red-700"
            >
              Confirm
            </button>
            <button
              type="button"
              onClick={() => setShowDeleteConfirm(false)}
              className="rounded border border-gray-300 px-2 py-0.5 text-xs text-gray-700 hover:bg-gray-50"
            >
              Cancel
            </button>
          </div>
        ) : (
          <p className="mt-0.5 text-sm text-gray-700">{comment.content}</p>
        )}
      </div>
    </div>
  );
}

function TimelineAgentSessionItem({ session }: { session: AgentSession }) {
  return (
    <div data-testid="timeline-session" className="flex items-center gap-3 rounded-md bg-gray-50 px-3 py-2">
      <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-purple-100 text-xs font-medium text-purple-800">
        AI
      </div>
      <div className="flex flex-1 items-center gap-2">
        <span className="text-sm font-medium text-gray-900">
          {AGENT_LABELS[session.agent_type] ?? session.agent_type}
        </span>
        <span
          className={`inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium ${STATUS_COLORS[session.status]}`}
        >
          {session.status}
        </span>
        <span className="text-xs text-gray-500">{timeAgo(session.created_at)}</span>
      </div>
    </div>
  );
}

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

  const activeSession =
    sessions?.find((s) => s.id === activeSessionId) ??
    sessions?.find((s) => s.status === 'running' || s.status === 'pending');

  useEffect(() => {
    if (activeSession && !activeSessionId) {
      setActiveSession(activeSession.id);
    } else if (!activeSession && activeSessionId) {
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

  const handleStartAgent = async () => {
    setAgentError(null);
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
      setAgentError('Failed to start agent session.');
    }
  };

  const handleStopAgent = async () => {
    if (!activeSession) return;
    try {
      await stopSession.mutateAsync({ taskId, sessionId: activeSession.id });
      reset();
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
      {viewingSessionId && (() => {
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
              />
            ),
          )}
        </div>
      )}
    </div>
  );
}
