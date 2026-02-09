import { create } from 'zustand';

interface AgentState {
  activeSessionId: string | null;
  outputLines: string[];
  isStarting: boolean;
  setActiveSession: (sessionId: string | null) => void;
  appendOutput: (text: string) => void;
  setStarting: (value: boolean) => void;
  reset: () => void;
}

const MAX_OUTPUT_LINES = 1000;

export const useAgentStore = create<AgentState>((set) => ({
  activeSessionId: null,
  outputLines: [],
  isStarting: false,
  setActiveSession: (sessionId) => set({ activeSessionId: sessionId }),
  appendOutput: (text) =>
    set((state) => ({
      outputLines: [...state.outputLines.slice(-(MAX_OUTPUT_LINES - 1)), text],
    })),
  setStarting: (value) => set({ isStarting: value }),
  reset: () => set({ activeSessionId: null, outputLines: [], isStarting: false }),
}));
