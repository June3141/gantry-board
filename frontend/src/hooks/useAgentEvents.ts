import { useEffect, useRef } from 'react';
import { createRealtimeTransport, type EventSourceLike } from '@/lib/realtimeTransport';

const API_BASE_URL = import.meta.env.VITE_API_BASE_URL ?? 'http://localhost:3000';

const CLOSED = 2;

// Module-level shared transport with reference counting
let sharedTransport: EventSourceLike | null = null;
let listenerCount = 0;

function acquireTransport(): EventSourceLike {
  if (!sharedTransport || sharedTransport.readyState === CLOSED) {
    sharedTransport = createRealtimeTransport(`${API_BASE_URL}/api/events`);
  }
  listenerCount++;
  return sharedTransport;
}

function releaseTransport(): void {
  listenerCount--;
  if (listenerCount <= 0) {
    sharedTransport?.close();
    sharedTransport = null;
    listenerCount = 0;
  }
}

/** Reset shared state — only for testing. */
export function _resetSharedEventSource(): void {
  sharedTransport?.close();
  sharedTransport = null;
  listenerCount = 0;
}

export function useAgentEvents(sessionId: string | null, onOutput: (text: string) => void): void {
  const onOutputRef = useRef(onOutput);
  onOutputRef.current = onOutput;

  useEffect(() => {
    if (!sessionId) return;

    const es = acquireTransport();

    const handler = (event: MessageEvent) => {
      try {
        const data = JSON.parse(event.data) as { session_id: string; text: string };
        if (data.session_id === sessionId) {
          onOutputRef.current(data.text);
        }
      } catch {
        // Ignore malformed events
      }
    };

    es.addEventListener('agent_output', handler);

    return () => {
      es.removeEventListener('agent_output', handler);
      releaseTransport();
    };
  }, [sessionId]);
}
