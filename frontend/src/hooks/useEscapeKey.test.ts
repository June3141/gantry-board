import { renderHook } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { useEscapeKey } from './useEscapeKey';

function pressEscape() {
  window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
}

function pressEnter() {
  window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter' }));
}

describe('useEscapeKey', () => {
  it('calls onEscape when Escape is pressed', () => {
    const onEscape = vi.fn();
    renderHook(() => useEscapeKey(onEscape));

    pressEscape();

    expect(onEscape).toHaveBeenCalledOnce();
  });

  it('does not call onEscape for other keys', () => {
    const onEscape = vi.fn();
    renderHook(() => useEscapeKey(onEscape));

    pressEnter();

    expect(onEscape).not.toHaveBeenCalled();
  });

  it('skips onEscape when guard returns true', () => {
    const onEscape = vi.fn();
    const guard = vi.fn().mockReturnValue(true);
    renderHook(() => useEscapeKey(onEscape, guard));

    pressEscape();

    expect(guard).toHaveBeenCalledOnce();
    expect(onEscape).not.toHaveBeenCalled();
  });

  it('calls onEscape when guard returns false', () => {
    const onEscape = vi.fn();
    const guard = vi.fn().mockReturnValue(false);
    renderHook(() => useEscapeKey(onEscape, guard));

    pressEscape();

    expect(guard).toHaveBeenCalledOnce();
    expect(onEscape).toHaveBeenCalledOnce();
  });

  it('removes event listener on unmount', () => {
    const onEscape = vi.fn();
    const { unmount } = renderHook(() => useEscapeKey(onEscape));

    unmount();
    pressEscape();

    expect(onEscape).not.toHaveBeenCalled();
  });
});
