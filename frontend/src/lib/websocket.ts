const WS_BASE_URL = import.meta.env.VITE_WS_BASE_URL ?? 'ws://localhost:3000';

export function createWebSocket(path: string): WebSocket {
  return new WebSocket(`${WS_BASE_URL}${path}`);
}
