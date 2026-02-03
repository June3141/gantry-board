import { create } from 'zustand';

interface BoardState {
  activeTaskId: string | null;
  setActiveTaskId: (id: string | null) => void;
}

export const useBoardStore = create<BoardState>((set) => ({
  activeTaskId: null,
  setActiveTaskId: (id) => set({ activeTaskId: id }),
}));
