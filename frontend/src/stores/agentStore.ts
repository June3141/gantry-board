import { create } from 'zustand';

interface AgentState {
  activeSessionId: string | null;
  outputLines: string[];
  isStarting: boolean;
  isLoadingHistory: boolean;
  setActiveSession: (sessionId: string | null) => void;
  appendOutput: (text: string) => void;
  setOutputLines: (lines: string[]) => void;
  setStarting: (value: boolean) => void;
  setLoadingHistory: (value: boolean) => void;
  reset: () => void;
}

const MAX_OUTPUT_LINES = 1000;

export const useAgentStore = create<AgentState>((set) => ({
  activeSessionId: null,
  outputLines: [],
  isStarting: false,
  isLoadingHistory: false,
  setActiveSession: (sessionId) => set({ activeSessionId: sessionId }),
  appendOutput: (text) =>
    set((state) => ({
      outputLines: [...state.outputLines.slice(-(MAX_OUTPUT_LINES - 1)), text],
    })),
  setOutputLines: (lines) => set({ outputLines: lines.slice(-MAX_OUTPUT_LINES) }),
  setStarting: (value) => set({ isStarting: value }),
  setLoadingHistory: (value) => set({ isLoadingHistory: value }),
  reset: () =>
    set({ activeSessionId: null, outputLines: [], isStarting: false, isLoadingHistory: false }),
}));
