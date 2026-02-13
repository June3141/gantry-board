import { useEffect, useRef } from 'react';

const API_BASE_URL = import.meta.env.VITE_API_BASE_URL ?? 'http://localhost:3000';

// Module-level shared EventSource with reference counting
let sharedEventSource: EventSource | null = null;
let listenerCount = 0;

function acquireEventSource(): EventSource {
  if (!sharedEventSource || sharedEventSource.readyState === EventSource.CLOSED) {
    sharedEventSource = new EventSource(`${API_BASE_URL}/api/events`);
  }
  listenerCount++;
  return sharedEventSource;
}

function releaseEventSource(): void {
  listenerCount--;
  if (listenerCount <= 0) {
    sharedEventSource?.close();
    sharedEventSource = null;
    listenerCount = 0;
  }
}

/** Reset shared state — only for testing. */
export function _resetSharedEventSource(): void {
  sharedEventSource?.close();
  sharedEventSource = null;
  listenerCount = 0;
}

export function useAgentEvents(sessionId: string | null, onOutput: (text: string) => void): void {
  const onOutputRef = useRef(onOutput);
  onOutputRef.current = onOutput;

  useEffect(() => {
    if (!sessionId) return;

    const es = acquireEventSource();

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
      releaseEventSource();
    };
  }, [sessionId]);
}
