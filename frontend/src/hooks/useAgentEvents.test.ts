import { renderHook } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useAgentEvents } from './useAgentEvents';

class MockEventSource {
  static instances: MockEventSource[] = [];
  url: string;
  onerror: ((event: Event) => void) | null = null;
  readyState = 0;
  private handlers: Record<string, ((event: MessageEvent) => void)[]> = {};

  constructor(url: string) {
    this.url = url;
    MockEventSource.instances.push(this);
  }

  close() {
    this.readyState = 2;
  }

  addEventListener(type: string, handler: (event: MessageEvent) => void) {
    if (!this.handlers[type]) {
      this.handlers[type] = [];
    }
    this.handlers[type].push(handler);
  }

  simulateEvent(type: string, data: unknown) {
    const handlers = this.handlers[type] ?? [];
    for (const handler of handlers) {
      handler(new MessageEvent(type, { data: JSON.stringify(data) }));
    }
  }
}

describe('useAgentEvents', () => {
  beforeEach(() => {
    MockEventSource.instances = [];
    vi.stubGlobal('EventSource', MockEventSource);
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('does not connect when sessionId is null', () => {
    const onOutput = vi.fn();
    renderHook(() => useAgentEvents(null, onOutput));

    expect(MockEventSource.instances.length).toBe(0);
  });

  it('connects to SSE when sessionId is provided', () => {
    const onOutput = vi.fn();
    renderHook(() => useAgentEvents('session-1', onOutput));

    expect(MockEventSource.instances.length).toBe(1);
    expect(MockEventSource.instances[0].url).toContain('/api/events');
  });

  it('calls onOutput for matching session_id', () => {
    const onOutput = vi.fn();
    renderHook(() => useAgentEvents('session-1', onOutput));

    const es = MockEventSource.instances[0];
    es.simulateEvent('agent_output', {
      session_id: 'session-1',
      text: 'Hello world',
    });

    expect(onOutput).toHaveBeenCalledWith('Hello world');
  });

  it('filters out events for other sessions', () => {
    const onOutput = vi.fn();
    renderHook(() => useAgentEvents('session-1', onOutput));

    const es = MockEventSource.instances[0];
    es.simulateEvent('agent_output', {
      session_id: 'session-2',
      text: 'Other session output',
    });

    expect(onOutput).not.toHaveBeenCalled();
  });

  it('disconnects on cleanup', () => {
    const onOutput = vi.fn();
    const { unmount } = renderHook(() => useAgentEvents('session-1', onOutput));

    const es = MockEventSource.instances[0];
    unmount();

    expect(es.readyState).toBe(2);
  });
});
