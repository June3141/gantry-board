import type { QueryClient } from '@tanstack/react-query';

import type { Task } from '../api/generated/model';

const API_BASE_URL = import.meta.env.VITE_API_BASE_URL ?? 'http://localhost:3000';

export type SseEvent =
  | { type: 'TaskCreated'; task: Task }
  | { type: 'TaskUpdated'; task: Task }
  | { type: 'TaskDeleted'; task_id: string };

export function useEventSource(queryClient: QueryClient): () => void {
  const eventSource = new EventSource(`${API_BASE_URL}/api/events`);

  const handleTaskEvent = () => {
    queryClient.invalidateQueries({
      queryKey: ['/api/tasks'],
    });
  };

  eventSource.addEventListener('task_created', (event: MessageEvent) => {
    // Parse and handle task_created event
    try {
      JSON.parse(event.data) as SseEvent;
      handleTaskEvent();
    } catch {
      console.error('Failed to parse SSE event:', event.data);
    }
  });

  eventSource.addEventListener('task_updated', (event: MessageEvent) => {
    try {
      JSON.parse(event.data) as SseEvent;
      handleTaskEvent();
    } catch {
      console.error('Failed to parse SSE event:', event.data);
    }
  });

  eventSource.addEventListener('task_deleted', (event: MessageEvent) => {
    try {
      JSON.parse(event.data) as SseEvent;
      handleTaskEvent();
    } catch {
      console.error('Failed to parse SSE event:', event.data);
    }
  });

  eventSource.onerror = () => {
    console.error('SSE connection error');
  };

  return () => {
    eventSource.close();
  };
}
