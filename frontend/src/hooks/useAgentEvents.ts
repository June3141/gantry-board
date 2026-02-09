import { useEffect } from 'react';

const API_BASE_URL = import.meta.env.VITE_API_BASE_URL ?? 'http://localhost:3000';

export function useAgentEvents(sessionId: string | null, onOutput: (text: string) => void): void {
  useEffect(() => {
    if (!sessionId) return;

    const eventSource = new EventSource(`${API_BASE_URL}/api/events`);

    const handleOutput = (event: MessageEvent) => {
      try {
        const data = JSON.parse(event.data) as {
          session_id: string;
          text: string;
        };
        if (data.session_id === sessionId) {
          onOutput(data.text);
        }
      } catch {
        // Ignore malformed events
      }
    };

    eventSource.addEventListener('agent_output', handleOutput);

    return () => {
      eventSource.close();
    };
  }, [sessionId, onOutput]);
}
