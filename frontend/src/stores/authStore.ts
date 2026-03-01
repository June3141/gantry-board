import { create } from 'zustand';
import { createJSONStorage, persist } from 'zustand/middleware';
import { me } from '@/api/generated/endpoints/auth/auth';
import type { User } from '@/api/generated/model';

interface AuthState {
  user: User | null;
  isAuthenticated: boolean;
  isLoading: boolean;
  setUser: (user: User | null) => void;
  setLoading: (loading: boolean) => void;
  logout: () => void;
}

export const useAuthStore = create<AuthState>()(
  persist(
    (set) => ({
      user: null,
      isAuthenticated: false,
      isLoading: true,
      setUser: (user) =>
        set({
          user,
          isAuthenticated: !!user,
          isLoading: false,
        }),
      setLoading: (loading) => set({ isLoading: loading }),
      logout: () => set({ user: null, isAuthenticated: false, isLoading: false }),
    }),
    {
      name: 'auth-storage',
      storage: createJSONStorage(() => sessionStorage),
      partialize: (state) => ({ user: state.user, isAuthenticated: state.isAuthenticated }),
      onRehydrateStorage: () => {
        return (state, error) => {
          if (error) {
            useAuthStore.setState({ isAuthenticated: false, user: null, isLoading: true });
            return;
          }

          // When restored isAuthenticated is true, revalidate with server
          if (state?.isAuthenticated) {
            me()
              .then((user) => {
                useAuthStore.setState({
                  user,
                  isAuthenticated: true,
                  isLoading: false,
                });
              })
              .catch(() => {
                // Session expired or invalid — clear auth state
                useAuthStore.setState({
                  user: null,
                  isAuthenticated: false,
                  isLoading: false,
                });
              });
          } else {
            // Not authenticated — clear loading state immediately
            useAuthStore.setState({ isLoading: false });
          }
        };
      },
    },
  ),
);
