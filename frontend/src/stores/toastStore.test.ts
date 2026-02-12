import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { useToastStore } from './toastStore';

describe('toastStore', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.restoreAllTimers();
    useToastStore.setState({ toasts: [] });
  });

  it('has correct initial state', () => {
    const state = useToastStore.getState();
    expect(state.toasts).toEqual([]);
  });

  it('adds a toast with addToast', () => {
    useToastStore.getState().addToast('success', 'Operation succeeded');
    const { toasts } = useToastStore.getState();
    expect(toasts).toHaveLength(1);
    expect(toasts[0].type).toBe('success');
    expect(toasts[0].message).toBe('Operation succeeded');
    expect(toasts[0].id).toBeDefined();
  });

  it('adds multiple toasts', () => {
    const { addToast } = useToastStore.getState();
    addToast('success', 'First');
    addToast('error', 'Second');
    addToast('info', 'Third');
    const { toasts } = useToastStore.getState();
    expect(toasts).toHaveLength(3);
    expect(toasts[0].type).toBe('success');
    expect(toasts[1].type).toBe('error');
    expect(toasts[2].type).toBe('info');
  });

  it('removes a toast with removeToast', () => {
    useToastStore.getState().addToast('info', 'Will be removed');
    const { toasts } = useToastStore.getState();
    const toastId = toasts[0].id;

    useToastStore.getState().removeToast(toastId);
    expect(useToastStore.getState().toasts).toHaveLength(0);
  });

  it('only removes the specified toast', () => {
    const { addToast } = useToastStore.getState();
    addToast('success', 'Keep me');
    addToast('error', 'Remove me');
    const toasts = useToastStore.getState().toasts;
    const removeId = toasts[1].id;

    useToastStore.getState().removeToast(removeId);
    const remaining = useToastStore.getState().toasts;
    expect(remaining).toHaveLength(1);
    expect(remaining[0].message).toBe('Keep me');
  });

  it('auto-removes toast after 5 seconds', () => {
    useToastStore.getState().addToast('success', 'Auto dismiss');
    expect(useToastStore.getState().toasts).toHaveLength(1);

    vi.advanceTimersByTime(5000);
    expect(useToastStore.getState().toasts).toHaveLength(0);
  });

  it('does not remove toast before 5 seconds', () => {
    useToastStore.getState().addToast('success', 'Not yet');
    vi.advanceTimersByTime(4999);
    expect(useToastStore.getState().toasts).toHaveLength(1);
  });
});
