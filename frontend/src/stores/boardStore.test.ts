import { describe, expect, it } from 'vitest';
import { TaskPriority } from '@/api/generated/model';
import { useBoardStore } from './boardStore';

describe('boardStore filters', () => {
  it('has empty initial filter state', () => {
    const state = useBoardStore.getState();
    expect(state.searchText).toBe('');
    expect(state.assigneeFilter).toEqual([]);
    expect(state.priorityFilter).toEqual([]);
  });

  it('sets search text', () => {
    useBoardStore.getState().setSearchText('bug fix');
    expect(useBoardStore.getState().searchText).toBe('bug fix');
  });

  it('sets assignee filter', () => {
    useBoardStore.getState().setAssigneeFilter(['user-1', 'unassigned']);
    expect(useBoardStore.getState().assigneeFilter).toEqual(['user-1', 'unassigned']);
  });

  it('sets priority filter', () => {
    useBoardStore.getState().setPriorityFilter([TaskPriority.high, TaskPriority.urgent]);
    expect(useBoardStore.getState().priorityFilter).toEqual([
      TaskPriority.high,
      TaskPriority.urgent,
    ]);
  });

  it('clears all filters', () => {
    useBoardStore.getState().setSearchText('test');
    useBoardStore.getState().setAssigneeFilter(['user-1']);
    useBoardStore.getState().setPriorityFilter([TaskPriority.low]);

    useBoardStore.getState().clearFilters();

    const state = useBoardStore.getState();
    expect(state.searchText).toBe('');
    expect(state.assigneeFilter).toEqual([]);
    expect(state.priorityFilter).toEqual([]);
  });

  it('hasActiveFilters returns false when no filters', () => {
    expect(useBoardStore.getState().hasActiveFilters()).toBe(false);
  });

  it('hasActiveFilters returns true when searchText is set', () => {
    useBoardStore.getState().setSearchText('search');
    expect(useBoardStore.getState().hasActiveFilters()).toBe(true);
  });

  it('hasActiveFilters returns true when assigneeFilter is set', () => {
    useBoardStore.getState().setAssigneeFilter(['user-1']);
    expect(useBoardStore.getState().hasActiveFilters()).toBe(true);
  });

  it('hasActiveFilters returns true when priorityFilter is set', () => {
    useBoardStore.getState().setPriorityFilter([TaskPriority.medium]);
    expect(useBoardStore.getState().hasActiveFilters()).toBe(true);
  });
});
