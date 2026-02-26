import { useQueryClient } from '@tanstack/react-query';
import { LogOut } from 'lucide-react';
import { useEffect } from 'react';
import { Link, Outlet } from 'react-router-dom';
import { useLogout } from '@/api/generated/endpoints/auth/auth';
import { connectEventSource } from '@/hooks/useEventSource';
import { useAuthStore } from '@/stores/authStore';

export function AppLayout() {
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
    <div className="min-h-screen bg-gray-100">
      <header className="bg-white shadow">
        <div className="mx-auto flex max-w-7xl items-center justify-between px-4 py-4">
          <Link to="/" className="text-2xl font-bold text-gray-900 hover:text-gray-700">
            Gantry Board
          </Link>
          <div className="flex items-center gap-2 border-l pl-4">
            <span className="text-sm text-gray-600">{user?.name ?? user?.email}</span>
            <button
              type="button"
              onClick={handleLogout}
              disabled={logout.isPending}
              className="flex items-center gap-1.5 rounded-md bg-gray-100 px-3 py-1.5 text-sm text-gray-700 hover:bg-gray-200"
            >
              <LogOut className="h-4 w-4" /> Logout
            </button>
          </div>
        </div>
      </header>
      <main>
        <Outlet />
      </main>
    </div>
  );
}
