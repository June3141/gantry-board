import { QueryClient } from '@tanstack/react-query';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { useEventSource, SseEvent } from './useEventSource';

// Mock EventSource
class MockEventSource {
  static instances: MockEventSource[] = [];
  url: string;
  onmessage: ((event: MessageEvent) => void) | null = null;
  onerror: ((event: Event) => void) | null = null;
  readyState = 0;

  constructor(url: string) {
    this.url = url;
    MockEventSource.instances.push(this);
  }

  close() {
    this.readyState = 2;
  }

  simulateMessage(type: string, data: unknown) {
    if (this.onmessage) {
      this.onmessage(new MessageEvent('message', { data: JSON.stringify(data) }));
    }
  }

  addEventListener(type: string, handler: (event: MessageEvent) => void) {
    if (type === 'task_created' || type === 'task_updated' || type === 'task_deleted') {
      // Store handlers for specific event types
      (this as Record<string, unknown>)[`on${type}`] = handler;
    }
  }

  simulateTypedEvent(type: string, data: unknown) {
    const handler = (this as Record<string, unknown>)[`on${type}`] as
      | ((event: MessageEvent) => void)
      | undefined;
    if (handler) {
      handler(new MessageEvent(type, { data: JSON.stringify(data) }));
    }
  }
}

describe('useEventSource', () => {
  let queryClient: QueryClient;

  beforeEach(() => {
    MockEventSource.instances = [];
    vi.stubGlobal('EventSource', MockEventSource);
    queryClient = new QueryClient({
      defaultOptions: {
        queries: { retry: false },
      },
    });
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    queryClient.clear();
  });

  it('connects to the SSE endpoint', () => {
    const cleanup = useEventSource(queryClient);

    expect(MockEventSource.instances.length).toBe(1);
    expect(MockEventSource.instances[0].url).toContain('/api/events');

    cleanup();
  });

  it('closes connection on cleanup', () => {
    const cleanup = useEventSource(queryClient);
    const eventSource = MockEventSource.instances[0];

    cleanup();

    expect(eventSource.readyState).toBe(2);
  });

  it('invalidates task queries on task_created event', () => {
    const invalidateSpy = vi.spyOn(queryClient, 'invalidateQueries');
    const cleanup = useEventSource(queryClient);
    const eventSource = MockEventSource.instances[0];

    const taskEvent: SseEvent = {
      type: 'TaskCreated',
      task: {
        id: '123',
        project_id: 'proj-1',
        title: 'New Task',
        status: 'backlog',
        priority: 'medium',
        position: 0,
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
      },
    };

    eventSource.simulateTypedEvent('task_created', taskEvent);

    expect(invalidateSpy).toHaveBeenCalledWith({
      queryKey: ['/api/tasks'],
    });

    cleanup();
  });

  it('invalidates task queries on task_updated event', () => {
    const invalidateSpy = vi.spyOn(queryClient, 'invalidateQueries');
    const cleanup = useEventSource(queryClient);
    const eventSource = MockEventSource.instances[0];

    const taskEvent: SseEvent = {
      type: 'TaskUpdated',
      task: {
        id: '123',
        project_id: 'proj-1',
        title: 'Updated Task',
        status: 'in_progress',
        priority: 'high',
        position: 1,
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
      },
    };

    eventSource.simulateTypedEvent('task_updated', taskEvent);

    expect(invalidateSpy).toHaveBeenCalledWith({
      queryKey: ['/api/tasks'],
    });

    cleanup();
  });

  it('invalidates task queries on task_deleted event', () => {
    const invalidateSpy = vi.spyOn(queryClient, 'invalidateQueries');
    const cleanup = useEventSource(queryClient);
    const eventSource = MockEventSource.instances[0];

    const deleteEvent: SseEvent = {
      type: 'TaskDeleted',
      task_id: '123',
    };

    eventSource.simulateTypedEvent('task_deleted', deleteEvent);

    expect(invalidateSpy).toHaveBeenCalledWith({
      queryKey: ['/api/tasks'],
    });

    cleanup();
  });
});
