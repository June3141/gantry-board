import type { QueryClient } from '@tanstack/react-query';

import type {
  AgentSession,
  DockerPreview,
  SyncResult,
  Task,
  TaskComment,
} from '../api/generated/model';

const API_BASE_URL = import.meta.env.VITE_API_BASE_URL ?? 'http://localhost:3000';

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
  const eventSource = new EventSource(`${API_BASE_URL}/api/events`);

  const handleTaskEvent = () => {
    // Invalidate all task queries (including variants with project_id filter)
    queryClient.invalidateQueries({
      queryKey: ['/api/tasks'],
      exact: false,
    });
  };

  const handleSseMessage = (event: MessageEvent) => {
    try {
      // Validate JSON structure; parsed value used for type checking
      const parsed = JSON.parse(event.data) as SseEvent;
      if (parsed.type) {
        handleTaskEvent();
      }
    } catch {
      console.error('Failed to parse SSE event:', event.data);
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
        queryClient.invalidateQueries({
          queryKey: [`/api/tasks/${taskId}/sessions`],
          exact: false,
        });
      }
      handleTaskEvent();
    } catch {
      console.error('Failed to parse SSE event:', event.data);
    }
  };
  eventSource.addEventListener('agent_session_status_changed', handleAgentSessionEvent);

  const handleCommentEvent = (event: MessageEvent) => {
    try {
      const parsed = JSON.parse(event.data) as SseEvent;
      const taskId =
        'comment' in parsed ? parsed.comment.task_id : 'task_id' in parsed ? parsed.task_id : null;
      if (taskId) {
        queryClient.invalidateQueries({
          queryKey: [`/api/tasks/${taskId}/comments`],
          exact: false,
        });
      }
    } catch {
      console.error('Failed to parse SSE event:', event.data);
    }
  };
  eventSource.addEventListener('comment_created', handleCommentEvent);
  eventSource.addEventListener('comment_updated', handleCommentEvent);
  eventSource.addEventListener('comment_deleted', handleCommentEvent);

  const handlePreviewEvent = () => {
    queryClient.invalidateQueries({
      queryKey: ['/api/previews'],
      exact: false,
    });
  };
  eventSource.addEventListener('preview_status_changed', handlePreviewEvent);
  eventSource.addEventListener('preview_deleted', handlePreviewEvent);

  const handleGithubSyncEvent = () => {
    // Invalidate GitHub link and pull request queries
    queryClient.invalidateQueries({
      predicate: (query) => {
        const key = query.queryKey[0];
        return (
          typeof key === 'string' &&
          (key.includes('github-link') || key.includes('pull-requests'))
        );
      },
    });
  };
  eventSource.addEventListener('github_sync_completed', handleGithubSyncEvent);
  eventSource.addEventListener('github_sync_failed', handleGithubSyncEvent);

  eventSource.onerror = (event: Event) => {
    const source = event.currentTarget as EventSource | null;
    console.error('SSE connection error', {
      type: event.type,
      readyState: source?.readyState,
    });
  };

  return () => {
    eventSource.close();
  };
}
