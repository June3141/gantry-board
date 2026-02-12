import { create } from 'zustand';

import type { TaskPriority } from '../api/generated/model';

interface BoardState {
  activeTaskId: string | null;
  setActiveTaskId: (id: string | null) => void;
  searchText: string;
  setSearchText: (text: string) => void;
  assigneeFilter: string[];
  setAssigneeFilter: (ids: string[]) => void;
  priorityFilter: TaskPriority[];
  setPriorityFilter: (priorities: TaskPriority[]) => void;
  clearFilters: () => void;
  hasActiveFilters: () => boolean;
}

export const useBoardStore = create<BoardState>((set, get) => ({
  activeTaskId: null,
  setActiveTaskId: (id) => set({ activeTaskId: id }),
  searchText: '',
  setSearchText: (text) => set({ searchText: text }),
  assigneeFilter: [],
  setAssigneeFilter: (ids) => set({ assigneeFilter: ids }),
  priorityFilter: [],
  setPriorityFilter: (priorities) => set({ priorityFilter: priorities }),
  clearFilters: () => set({ searchText: '', assigneeFilter: [], priorityFilter: [] }),
  hasActiveFilters: () => {
    const state = get();
    return (
      state.searchText !== '' ||
      state.assigneeFilter.length > 0 ||
      state.priorityFilter.length > 0
    );
  },
}));
