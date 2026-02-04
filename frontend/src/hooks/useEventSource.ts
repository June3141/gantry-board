import type { QueryClient } from '@tanstack/react-query';

import type { Task } from '../api/generated/model';

const API_BASE_URL = import.meta.env.VITE_API_BASE_URL ?? 'http://localhost:3000';

export type SseEvent =
  | { type: 'TaskCreated'; task: Task }
  | { type: 'TaskUpdated'; task: Task }
  | { type: 'TaskDeleted'; task_id: string };

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
