import { useTranslation } from 'react-i18next';
import { Navigate } from 'react-router';
import { useAuthStore } from '@/stores/authStore';

interface GuestRouteProps {
  children: React.ReactNode;
}

export function GuestRoute({ children }: GuestRouteProps) {
  const { t } = useTranslation();
  const { isAuthenticated, isLoading } = useAuthStore();

  if (isLoading) {
    return (
      <div className="flex min-h-screen items-center justify-center bg-muted">
        <div className="text-muted-foreground">{t('common.loading')}</div>
      </div>
    );
  }

  if (isAuthenticated) {
    return <Navigate to="/" replace />;
  }

  return <>{children}</>;
}
