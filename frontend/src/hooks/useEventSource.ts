import type { QueryClient } from '@tanstack/react-query';
import type {
  AgentSession,
  DockerPreview,
  SyncResult,
  Task,
  TaskComment,
} from '../api/generated/model';
import { logger } from '../lib/logger';
import { createRealtimeTransport } from '../lib/realtimeTransport';
import {
  invalidateComments,
  invalidatePreviews,
  invalidateSessions,
  invalidateTasks,
} from '../services/queryInvalidation';

const API_BASE_URL = import.meta.env.VITE_API_BASE_URL ?? 'http://localhost:3000';
const sseLog = logger.child({ module: 'sse' });

export type SseEvent =
  | { type: 'TaskCreated'; task: Task }
  | { type: 'TaskUpdated'; task: Task }
  | { type: 'TaskDeleted'; task_id: string }
  | { type: 'AgentOutput'; session_id: string; text: string }
  | { type: 'AgentSessionStatusChanged'; session: AgentSession }
  | { type: 'CommentCreated'; comment: TaskComment }
  | { type: 'CommentUpdated'; comment: TaskComment }
  | { type: 'CommentDeleted'; comment_id: string; task_id: string }
  | { type: 'PreviewStatusChanged'; preview: DockerPreview }
  | { type: 'PreviewDeleted'; preview_id: string }
  | { type: 'GitHubSyncCompleted'; result: SyncResult }
  | { type: 'GitHubSyncFailed'; project_id: string; error: string };

export function connectEventSource(queryClient: QueryClient): () => void {
  const eventSource = createRealtimeTransport(`${API_BASE_URL}/api/events`);

  const handleSseMessage = (event: MessageEvent) => {
    try {
      const parsed = JSON.parse(event.data) as SseEvent;
      if (parsed.type) {
        invalidateTasks(queryClient);
      }
    } catch {
      sseLog.error({ data: event.data }, 'failed to parse SSE event');
    }
  };

  eventSource.addEventListener('task_created', handleSseMessage);
  eventSource.addEventListener('task_updated', handleSseMessage);
  eventSource.addEventListener('task_deleted', handleSseMessage);

  const handleAgentSessionEvent = (event: MessageEvent) => {
    try {
      const parsed = JSON.parse(event.data) as SseEvent;
      if (parsed.type !== 'AgentSessionStatusChanged') {
        return;
      }
      const { task_id: taskId } = parsed.session;
      if (taskId) {
        invalidateSessions(queryClient, taskId);
      }
      invalidateTasks(queryClient);
    } catch {
      sseLog.error({ data: event.data }, 'failed to parse SSE event');
    }
  };
  eventSource.addEventListener('agent_session_status_changed', handleAgentSessionEvent);

  const handleCommentEvent = (event: MessageEvent) => {
    try {
      const parsed = JSON.parse(event.data) as SseEvent;
      const taskId =
        'comment' in parsed ? parsed.comment.task_id : 'task_id' in parsed ? parsed.task_id : null;
      if (taskId) {
        invalidateComments(queryClient, taskId);
      }
    } catch {
      sseLog.error({ data: event.data }, 'failed to parse SSE event');
    }
  };
  eventSource.addEventListener('comment_created', handleCommentEvent);
  eventSource.addEventListener('comment_updated', handleCommentEvent);
  eventSource.addEventListener('comment_deleted', handleCommentEvent);

  const handlePreviewEvent = () => {
    invalidatePreviews(queryClient);
  };
  eventSource.addEventListener('preview_status_changed', handlePreviewEvent);
  eventSource.addEventListener('preview_deleted', handlePreviewEvent);

  const handleGithubSyncEvent = (event: MessageEvent) => {
    try {
      JSON.parse(event.data);
      queryClient.invalidateQueries({
        predicate: (query) => {
          const key = query.queryKey[0];
          return (
            typeof key === 'string' &&
            (key.includes('github-link') || key.includes('pull-requests'))
          );
        },
      });
    } catch {
      sseLog.error({ data: event.data }, 'failed to parse GitHub sync SSE event');
    }
  };
  eventSource.addEventListener('github_sync_completed', handleGithubSyncEvent);
  eventSource.addEventListener('github_sync_failed', handleGithubSyncEvent);

  eventSource.onerror = (event: Event) => {
    sseLog.error({ type: event.type }, 'Realtime connection error');
  };

  return () => {
    eventSource.close();
  };
}
