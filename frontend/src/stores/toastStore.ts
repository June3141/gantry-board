import { create } from 'zustand';

export interface Toast {
  id: string;
  type: 'success' | 'error' | 'info';
  message: string;
}

interface ToastState {
  toasts: Toast[];
  addToast: (type: Toast['type'], message: string) => void;
  removeToast: (id: string) => void;
}

export const useToastStore = create<ToastState>((set) => {
  const timeouts = new Map<string, number>();

  return {
    toasts: [],
    addToast: (type, message) => {
      const id = crypto.randomUUID();
      set((state) => ({ toasts: [...state.toasts, { id, type, message }] }));
      const timeoutId = window.setTimeout(() => {
        set((state) => ({ toasts: state.toasts.filter((t) => t.id !== id) }));
        timeouts.delete(id);
      }, 5000);
      timeouts.set(id, timeoutId);
    },
    removeToast: (id) => {
      const timeoutId = timeouts.get(id);
      if (timeoutId !== undefined) {
        clearTimeout(timeoutId);
        timeouts.delete(id);
      }
      set((state) => ({ toasts: state.toasts.filter((t) => t.id !== id) }));
    },
  };
});
