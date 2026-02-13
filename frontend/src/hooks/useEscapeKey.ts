import { useEffect } from 'react';

/**
 * Calls the given callback when the Escape key is pressed.
 * If `guard` is provided and returns true, the callback is skipped for that event.
 */
export function useEscapeKey(onEscape: () => void, guard?: (e: KeyboardEvent) => boolean): void {
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        if (guard?.(e)) return;
        onEscape();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [onEscape, guard]);
}
