import { describe, expect, it } from 'vitest';

import { TaskStatus } from '@/api/generated/model';
import { useUiStore } from './uiStore';

describe('uiStore', () => {
  describe('task modal', () => {
    it('has correct initial state', () => {
      const state = useUiStore.getState();
      expect(state.isTaskModalOpen).toBe(false);
      expect(state.defaultStatus).toBeNull();
    });

    it('opens task modal without default status', () => {
      useUiStore.getState().openTaskModal();
      const state = useUiStore.getState();
      expect(state.isTaskModalOpen).toBe(true);
      expect(state.defaultStatus).toBeNull();
    });

    it('opens task modal with default status', () => {
      useUiStore.getState().openTaskModal(TaskStatus.todo);
      const state = useUiStore.getState();
      expect(state.isTaskModalOpen).toBe(true);
      expect(state.defaultStatus).toBe('todo');
    });

    it('resets defaultStatus when closing task modal', () => {
      useUiStore.getState().openTaskModal(TaskStatus.in_progress);
      useUiStore.getState().closeTaskModal();
      const state = useUiStore.getState();
      expect(state.isTaskModalOpen).toBe(false);
      expect(state.defaultStatus).toBeNull();
    });
  });

  describe('task detail modal', () => {
    it('has correct initial state', () => {
      const state = useUiStore.getState();
      expect(state.selectedTaskId).toBeNull();
      expect(state.isTaskDetailOpen).toBe(false);
    });

    it('opens task detail with task id', () => {
      useUiStore.getState().openTaskDetail('task-123');
      const state = useUiStore.getState();
      expect(state.selectedTaskId).toBe('task-123');
      expect(state.isTaskDetailOpen).toBe(true);
    });

    it('closes task detail and resets selectedTaskId', () => {
      useUiStore.getState().openTaskDetail('task-123');
      useUiStore.getState().closeTaskDetail();
      const state = useUiStore.getState();
      expect(state.selectedTaskId).toBeNull();
      expect(state.isTaskDetailOpen).toBe(false);
    });
  });

  describe('project modal', () => {
    it('has correct initial state', () => {
      const state = useUiStore.getState();
      expect(state.isProjectModalOpen).toBe(false);
    });

    it('opens project modal', () => {
      useUiStore.getState().openProjectModal();
      expect(useUiStore.getState().isProjectModalOpen).toBe(true);
    });

    it('closes project modal', () => {
      useUiStore.getState().openProjectModal();
      useUiStore.getState().closeProjectModal();
      expect(useUiStore.getState().isProjectModalOpen).toBe(false);
    });
  });

  describe('project settings modal', () => {
    it('has correct initial state', () => {
      expect(useUiStore.getState().isProjectSettingsOpen).toBe(false);
    });

    it('opens project settings modal', () => {
      useUiStore.getState().openProjectSettings();
      expect(useUiStore.getState().isProjectSettingsOpen).toBe(true);
    });

    it('closes project settings modal', () => {
      useUiStore.getState().openProjectSettings();
      useUiStore.getState().closeProjectSettings();
      expect(useUiStore.getState().isProjectSettingsOpen).toBe(false);
    });
  });

  describe('project members panel', () => {
    it('has correct initial state', () => {
      expect(useUiStore.getState().isProjectMembersOpen).toBe(false);
    });

    it('opens project members panel', () => {
      useUiStore.getState().openProjectMembers();
      expect(useUiStore.getState().isProjectMembersOpen).toBe(true);
    });

    it('closes project members panel', () => {
      useUiStore.getState().openProjectMembers();
      useUiStore.getState().closeProjectMembers();
      expect(useUiStore.getState().isProjectMembersOpen).toBe(false);
    });
  });
});
