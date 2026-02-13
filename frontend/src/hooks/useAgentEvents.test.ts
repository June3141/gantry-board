import { renderHook } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { _resetSharedEventSource, useAgentEvents } from './useAgentEvents';

class MockEventSource {
  static instances: MockEventSource[] = [];
  static CLOSED = 2;
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

  removeEventListener(type: string, handler: (event: MessageEvent) => void) {
    if (this.handlers[type]) {
      this.handlers[type] = this.handlers[type].filter((h) => h !== handler);
    }
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
    _resetSharedEventSource();
  });

  afterEach(() => {
    _resetSharedEventSource();
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

  it('shares EventSource across multiple hooks', () => {
    const onOutput1 = vi.fn();
    const onOutput2 = vi.fn();
    renderHook(() => useAgentEvents('session-1', onOutput1));
    renderHook(() => useAgentEvents('session-2', onOutput2));

    // Only one EventSource should be created (singleton)
    expect(MockEventSource.instances.length).toBe(1);
  });

  it('keeps EventSource open when one of multiple hooks unmounts', () => {
    const onOutput1 = vi.fn();
    const onOutput2 = vi.fn();
    const hook1 = renderHook(() => useAgentEvents('session-1', onOutput1));
    renderHook(() => useAgentEvents('session-2', onOutput2));

    const es = MockEventSource.instances[0];

    // Unmount first hook — EventSource should stay open for the second
    hook1.unmount();
    expect(es.readyState).toBe(0);
  });

  it('closes EventSource only when last hook unmounts', () => {
    const onOutput1 = vi.fn();
    const onOutput2 = vi.fn();
    const hook1 = renderHook(() => useAgentEvents('session-1', onOutput1));
    const hook2 = renderHook(() => useAgentEvents('session-2', onOutput2));

    const es = MockEventSource.instances[0];

    hook1.unmount();
    expect(es.readyState).toBe(0);

    hook2.unmount();
    expect(es.readyState).toBe(2);
  });
});
