import { logger } from './logger';

const rtLog = logger.child({ module: 'realtime' });

type EventHandler = (event: MessageEvent) => void;

/** EventSource-compatible interface for both SSE and WebSocket transports. */
export interface EventSourceLike {
  addEventListener(type: string, handler: EventHandler): void;
  removeEventListener(type: string, handler: EventHandler): void;
  close(): void;
  readonly readyState: number;
  onerror: ((event: Event) => void) | null;
}

/** WebSocket adapter that dispatches events in the same format as EventSource. */
class WebSocketAdapter implements EventSourceLike {
  private ws: WebSocket;
  private listeners = new Map<string, Set<EventHandler>>();
  onerror: ((event: Event) => void) | null = null;

  constructor(ws: WebSocket) {
    this.ws = ws;
    this.ws.onmessage = (event) => this.dispatch(event);
    this.ws.onerror = (e) => this.onerror?.(e);
  }

  private dispatch(event: MessageEvent) {
    try {
      const msg = JSON.parse(event.data as string) as { event: string; data: unknown };
      const handlers = this.listeners.get(msg.event);
      if (handlers) {
        const synthetic = new MessageEvent(msg.event, {
          data: JSON.stringify(msg.data),
        });
        for (const handler of handlers) {
          handler(synthetic);
        }
      }
    } catch {
      rtLog.error({ data: event.data }, 'failed to parse WebSocket message');
    }
  }

  addEventListener(type: string, handler: EventHandler): void {
    if (!this.listeners.has(type)) this.listeners.set(type, new Set());
    this.listeners.get(type)!.add(handler);
  }

  removeEventListener(type: string, handler: EventHandler): void {
    this.listeners.get(type)?.delete(handler);
  }

  close(): void {
    this.ws.close();
  }

  get readyState(): number {
    // Normalize: WS uses 2=CLOSING,3=CLOSED; EventSource uses 2=CLOSED
    return this.ws.readyState >= 2 ? 2 : this.ws.readyState;
  }
}

const WS_CONNECT_TIMEOUT_MS = 3000;

/**
 * Create a realtime transport that tries WebSocket first with SSE fallback.
 *
 * Starts with EventSource immediately, then attempts a WebSocket upgrade
 * in the background. If the WebSocket connects within the timeout, all
 * event handlers are transparently migrated to the WebSocket transport.
 */
export function createRealtimeTransport(sseUrl: string): EventSourceLike {
  const wsUrl = sseUrl
    .replace(/^http:/, 'ws:')
    .replace(/^https:/, 'wss:')
    .replace('/events', '/ws');

  return new RealtimeProxy(sseUrl, wsUrl);
}

class RealtimeProxy implements EventSourceLike {
  private inner: EventSourceLike;
  private handlers = new Map<string, Set<EventHandler>>();
  private _onerror: ((event: Event) => void) | null = null;
  private pendingWs: WebSocket | null = null;
  private upgradeTimer: ReturnType<typeof setTimeout> | null = null;

  constructor(sseUrl: string, wsUrl: string) {
    this.inner = new EventSource(sseUrl);
    this.tryUpgrade(wsUrl);
  }

  private tryUpgrade(wsUrl: string) {
    if (typeof WebSocket === 'undefined') return;

    try {
      const ws = new WebSocket(wsUrl);
      this.pendingWs = ws;

      this.upgradeTimer = setTimeout(() => {
        ws.close();
        this.pendingWs = null;
        this.upgradeTimer = null;
        rtLog.debug('WebSocket upgrade timed out, staying on SSE');
      }, WS_CONNECT_TIMEOUT_MS);

      ws.onopen = () => {
        if (this.upgradeTimer) {
          clearTimeout(this.upgradeTimer);
          this.upgradeTimer = null;
        }
        this.pendingWs = null;

        // Close SSE and switch to WebSocket
        this.inner.close();
        const adapter = new WebSocketAdapter(ws);

        // Re-register all handlers on new transport
        for (const [type, handlerSet] of this.handlers) {
          for (const handler of handlerSet) {
            adapter.addEventListener(type, handler);
          }
        }
        if (this._onerror) adapter.onerror = this._onerror;
        this.inner = adapter;
        rtLog.info('Upgraded to WebSocket transport');
      };

      ws.onerror = () => {
        if (this.upgradeTimer) {
          clearTimeout(this.upgradeTimer);
          this.upgradeTimer = null;
        }
        ws.close();
        this.pendingWs = null;
        rtLog.debug('WebSocket upgrade failed, staying on SSE');
      };
    } catch {
      // WebSocket constructor failed, stay on SSE
    }
  }

  addEventListener(type: string, handler: EventHandler): void {
    if (!this.handlers.has(type)) this.handlers.set(type, new Set());
    this.handlers.get(type)!.add(handler);
    this.inner.addEventListener(type, handler);
  }

  removeEventListener(type: string, handler: EventHandler): void {
    this.handlers.get(type)?.delete(handler);
    this.inner.removeEventListener(type, handler);
  }

  close(): void {
    if (this.upgradeTimer) {
      clearTimeout(this.upgradeTimer);
      this.upgradeTimer = null;
    }
    this.pendingWs?.close();
    this.inner.close();
  }

  get readyState(): number {
    return this.inner.readyState;
  }

  set onerror(handler: ((event: Event) => void) | null) {
    this._onerror = handler;
    this.inner.onerror = handler;
  }

  get onerror(): ((event: Event) => void) | null {
    return this._onerror;
  }
}
