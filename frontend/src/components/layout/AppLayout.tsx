import { useQueryClient } from '@tanstack/react-query';
import { LogOut } from 'lucide-react';
import { useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { Link, Outlet } from 'react-router-dom';
import { useLogout } from '@/api/generated/endpoints/auth/auth';
import { LanguageSwitcher } from '@/components/common/LanguageSwitcher';
import { Button } from '@/components/ui/button';
import { connectEventSource } from '@/hooks/useEventSource';
import { useAuthStore } from '@/stores/authStore';

export function AppLayout() {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const user = useAuthStore((state) => state.user);
  const logout = useLogout();
  const setUser = useAuthStore((state) => state.setUser);

  // Connect to real-time updates (WebSocket with SSE fallback)
  // biome-ignore lint/correctness/useExhaustiveDependencies: queryClient is stable from provider
  useEffect(() => {
    const cleanup = connectEventSource(queryClient);
    return cleanup;
  }, []);

  const handleLogout = async () => {
    try {
      await logout.mutateAsync();
      setUser(null);
    } catch {
      setUser(null);
    }
  };

  return (
    <div className="min-h-screen bg-muted">
      <header className="bg-background shadow">
        <div className="mx-auto flex max-w-7xl items-center justify-between px-4 py-4">
          <Link to="/" className="text-2xl font-bold text-foreground hover:text-foreground/80">
            {t('app.title')}
          </Link>
          <div className="flex items-center gap-2 border-l pl-4">
            <LanguageSwitcher />
            <span className="text-sm text-muted-foreground">{user?.name ?? user?.email}</span>
            <Button
              variant="secondary"
              size="sm"
              onClick={handleLogout}
              disabled={logout.isPending}
            >
              <LogOut className="h-4 w-4" /> {t('auth.logout')}
            </Button>
          </div>
        </div>
      </header>
      <main>
        <Outlet />
      </main>
    </div>
  );
}
