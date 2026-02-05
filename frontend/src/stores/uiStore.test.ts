import { afterEach, describe, expect, it } from 'vitest';

import { TaskStatus } from '../api/generated/model';
import { useUiStore } from './uiStore';

describe('uiStore', () => {
  afterEach(() => {
    // Reset store to initial state between tests
    useUiStore.setState({
      isTaskModalOpen: false,
      defaultStatus: null,
      isProjectModalOpen: false,
    });
  });

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
});
