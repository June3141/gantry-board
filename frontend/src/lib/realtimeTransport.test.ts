import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createRealtimeTransport } from './realtimeTransport';

// --- Mock WebSocket ---
class MockWebSocket {
  static instances: MockWebSocket[] = [];
  url: string;
  readyState = 0;
  onopen: ((e: Event) => void) | null = null;
  onmessage: ((e: MessageEvent) => void) | null = null;
  onerror: ((e: Event) => void) | null = null;
  onclose: ((e: CloseEvent) => void) | null = null;

  constructor(url: string) {
    this.url = url;
    MockWebSocket.instances.push(this);
  }

  close() {
    this.readyState = 3;
  }

  addEventListener(_type: string, _handler: unknown) {}
  removeEventListener(_type: string, _handler: unknown) {}
  send(_data: unknown) {}

  // Test helpers
  simulateOpen() {
    this.readyState = 1;
    this.onopen?.(new Event('open'));
  }

  simulateError() {
    this.onerror?.(new Event('error'));
  }

  simulateMessage(data: unknown) {
    this.onmessage?.(new MessageEvent('message', { data: JSON.stringify(data) }));
  }
}

// --- Mock EventSource ---
class MockEventSource {
  static instances: MockEventSource[] = [];
  static CLOSED = 2;
  url: string;
  readyState = 0;
  onerror: ((event: Event) => void) | null = null;
  private handlers: Record<string, Set<(event: MessageEvent) => void>> = {};

  constructor(url: string) {
    this.url = url;
    MockEventSource.instances.push(this);
  }

  close() {
    this.readyState = 2;
  }

  addEventListener(type: string, handler: (event: MessageEvent) => void) {
    if (!this.handlers[type]) this.handlers[type] = new Set();
    this.handlers[type].add(handler);
  }

  removeEventListener(type: string, handler: (event: MessageEvent) => void) {
    this.handlers[type]?.delete(handler);
  }

  simulateEvent(type: string, data: unknown) {
    const handlers = this.handlers[type];
    if (handlers) {
      for (const handler of handlers) {
        handler(new MessageEvent(type, { data: JSON.stringify(data) }));
      }
    }
  }
}

describe('createRealtimeTransport', () => {
  beforeEach(() => {
    MockWebSocket.instances = [];
    MockEventSource.instances = [];
    vi.stubGlobal('WebSocket', MockWebSocket);
    vi.stubGlobal('EventSource', MockEventSource);
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('creates EventSource as initial transport', () => {
    createRealtimeTransport('http://localhost:3000/api/events');

    expect(MockEventSource.instances.length).toBe(1);
    expect(MockEventSource.instances[0].url).toBe('http://localhost:3000/api/events');
  });

  it('attempts WebSocket upgrade in background', () => {
    createRealtimeTransport('http://localhost:3000/api/events');

    expect(MockWebSocket.instances.length).toBe(1);
    expect(MockWebSocket.instances[0].url).toBe('ws://localhost:3000/api/ws');
  });

  it('converts https to wss for WebSocket URL', () => {
    createRealtimeTransport('https://example.com/api/events');

    expect(MockWebSocket.instances[0].url).toBe('wss://example.com/api/ws');
  });

  it('closes SSE and switches to WebSocket on successful upgrade', () => {
    createRealtimeTransport('http://localhost:3000/api/events');

    const es = MockEventSource.instances[0];
    const ws = MockWebSocket.instances[0];

    ws.simulateOpen();

    expect(es.readyState).toBe(2); // SSE closed
  });

  it('stays on SSE when WebSocket fails', () => {
    createRealtimeTransport('http://localhost:3000/api/events');

    const es = MockEventSource.instances[0];
    const ws = MockWebSocket.instances[0];

    ws.simulateError();

    expect(es.readyState).toBe(0); // SSE still open
    expect(ws.readyState).toBe(3); // WS closed
  });

  it('delivers events via SSE before upgrade', () => {
    const transport = createRealtimeTransport('http://localhost:3000/api/events');
    const handler = vi.fn();

    transport.addEventListener('task_created', handler);

    const es = MockEventSource.instances[0];
    es.simulateEvent('task_created', { type: 'TaskCreated', task: { id: '123' } });

    expect(handler).toHaveBeenCalledTimes(1);
  });

  it('delivers events via WebSocket after upgrade', () => {
    const transport = createRealtimeTransport('http://localhost:3000/api/events');
    const handler = vi.fn();

    transport.addEventListener('task_created', handler);

    // Upgrade to WebSocket
    const ws = MockWebSocket.instances[0];
    ws.simulateOpen();

    // Send event via WebSocket
    ws.simulateMessage({
      event: 'task_created',
      data: { type: 'TaskCreated', task: { id: '456' } },
    });

    expect(handler).toHaveBeenCalledTimes(1);
    const event = handler.mock.calls[0][0] as MessageEvent;
    expect(JSON.parse(event.data).task.id).toBe('456');
  });

  it('re-registers all handlers after WebSocket upgrade', () => {
    const transport = createRealtimeTransport('http://localhost:3000/api/events');
    const handler1 = vi.fn();
    const handler2 = vi.fn();

    transport.addEventListener('task_created', handler1);
    transport.addEventListener('agent_output', handler2);

    // Upgrade to WebSocket
    const ws = MockWebSocket.instances[0];
    ws.simulateOpen();

    ws.simulateMessage({ event: 'task_created', data: { type: 'TaskCreated' } });
    ws.simulateMessage({
      event: 'agent_output',
      data: { type: 'AgentOutput', session_id: 'abc', text: 'hi' },
    });

    expect(handler1).toHaveBeenCalledTimes(1);
    expect(handler2).toHaveBeenCalledTimes(1);
  });

  it('closes both transports on close()', () => {
    const transport = createRealtimeTransport('http://localhost:3000/api/events');

    const es = MockEventSource.instances[0];
    const ws = MockWebSocket.instances[0];

    transport.close();

    expect(es.readyState).toBe(2);
    expect(ws.readyState).toBe(3);
  });

  it('removes event listeners correctly', () => {
    const transport = createRealtimeTransport('http://localhost:3000/api/events');
    const handler = vi.fn();

    transport.addEventListener('task_created', handler);
    transport.removeEventListener('task_created', handler);

    const es = MockEventSource.instances[0];
    es.simulateEvent('task_created', { type: 'TaskCreated', task: { id: '123' } });

    expect(handler).not.toHaveBeenCalled();
  });

  it('does not re-register removed handlers after upgrade', () => {
    const transport = createRealtimeTransport('http://localhost:3000/api/events');
    const handler = vi.fn();

    transport.addEventListener('task_created', handler);
    transport.removeEventListener('task_created', handler);

    // Upgrade to WebSocket
    const ws = MockWebSocket.instances[0];
    ws.simulateOpen();

    ws.simulateMessage({ event: 'task_created', data: { type: 'TaskCreated' } });

    expect(handler).not.toHaveBeenCalled();
  });

  it('propagates onerror to active transport', () => {
    const transport = createRealtimeTransport('http://localhost:3000/api/events');
    const errorHandler = vi.fn();

    transport.onerror = errorHandler;

    const es = MockEventSource.instances[0];
    es.onerror?.(new Event('error'));

    expect(errorHandler).toHaveBeenCalledTimes(1);
  });
});
