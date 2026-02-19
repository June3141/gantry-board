import { create } from 'zustand';

import type { TaskStatus } from '../api/generated/model';

interface UiState {
  isTaskModalOpen: boolean;
  defaultStatus: TaskStatus | null;
  openTaskModal: (status?: TaskStatus) => void;
  closeTaskModal: () => void;
  selectedTaskId: string | null;
  isTaskDetailOpen: boolean;
  openTaskDetail: (taskId: string) => void;
  closeTaskDetail: () => void;
  isProjectModalOpen: boolean;
  openProjectModal: () => void;
  closeProjectModal: () => void;
  isProjectSettingsOpen: boolean;
  openProjectSettings: () => void;
  closeProjectSettings: () => void;
  isProjectMembersOpen: boolean;
  openProjectMembers: () => void;
  closeProjectMembers: () => void;
  isProjectChatOpen: boolean;
  openProjectChat: () => void;
  closeProjectChat: () => void;
}

export const useUiStore = create<UiState>((set) => ({
  isTaskModalOpen: false,
  defaultStatus: null,
  openTaskModal: (status?: TaskStatus) =>
    set({ isTaskModalOpen: true, defaultStatus: status ?? null }),
  closeTaskModal: () => set({ isTaskModalOpen: false, defaultStatus: null }),
  selectedTaskId: null,
  isTaskDetailOpen: false,
  openTaskDetail: (taskId: string) => set({ selectedTaskId: taskId, isTaskDetailOpen: true }),
  closeTaskDetail: () => set({ selectedTaskId: null, isTaskDetailOpen: false }),
  isProjectModalOpen: false,
  openProjectModal: () => set({ isProjectModalOpen: true }),
  closeProjectModal: () => set({ isProjectModalOpen: false }),
  isProjectSettingsOpen: false,
  openProjectSettings: () => set({ isProjectSettingsOpen: true }),
  closeProjectSettings: () => set({ isProjectSettingsOpen: false }),
  isProjectMembersOpen: false,
  openProjectMembers: () => set({ isProjectMembersOpen: true }),
  closeProjectMembers: () => set({ isProjectMembersOpen: false }),
  isProjectChatOpen: false,
  openProjectChat: () => set({ isProjectChatOpen: true }),
  closeProjectChat: () => set({ isProjectChatOpen: false }),
}));
