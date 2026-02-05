import { create } from 'zustand';

import type { TaskStatus } from '../api/generated/model';

interface UiState {
  isTaskModalOpen: boolean;
  defaultStatus: TaskStatus | null;
  openTaskModal: (status?: TaskStatus) => void;
  closeTaskModal: () => void;
  isProjectModalOpen: boolean;
  openProjectModal: () => void;
  closeProjectModal: () => void;
}

export const useUiStore = create<UiState>((set) => ({
  isTaskModalOpen: false,
  defaultStatus: null,
  openTaskModal: (status?: TaskStatus) =>
    set({ isTaskModalOpen: true, defaultStatus: status ?? null }),
  closeTaskModal: () => set({ isTaskModalOpen: false, defaultStatus: null }),
  isProjectModalOpen: false,
  openProjectModal: () => set({ isProjectModalOpen: true }),
  closeProjectModal: () => set({ isProjectModalOpen: false }),
}));
