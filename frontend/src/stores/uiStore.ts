import { create } from 'zustand';

interface UiState {
  isTaskModalOpen: boolean;
  openTaskModal: () => void;
  closeTaskModal: () => void;
}

export const useUiStore = create<UiState>((set) => ({
  isTaskModalOpen: false,
  openTaskModal: () => set({ isTaskModalOpen: true }),
  closeTaskModal: () => set({ isTaskModalOpen: false }),
}));
