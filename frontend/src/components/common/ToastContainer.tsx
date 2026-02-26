import { useTranslation } from 'react-i18next';
import { useToastStore } from '@/stores/toastStore';

const typeStyles = {
  success: 'bg-success/10 border-success text-success',
  error: 'bg-destructive/10 border-destructive text-destructive',
  info: 'bg-primary/10 border-primary text-primary',
};

export function ToastContainer() {
  const { t } = useTranslation();
  const toasts = useToastStore((s) => s.toasts);
  const removeToast = useToastStore((s) => s.removeToast);

  return (
    <div
      data-testid="toast-container"
      className="fixed right-4 bottom-4 z-[60] flex flex-col gap-2"
    >
      {toasts.map((toast) => (
        <div
          key={toast.id}
          data-testid="toast-item"
          className={`flex items-center gap-2 rounded-md border px-4 py-3 shadow-md ${typeStyles[toast.type]}`}
        >
          <span className="flex-1 text-sm">{toast.message}</span>
          <button
            type="button"
            onClick={() => removeToast(toast.id)}
            aria-label={t('common.dismiss')}
            className="ml-2 text-lg leading-none opacity-60 hover:opacity-100"
          >
            &times;
          </button>
        </div>
      ))}
    </div>
  );
}
