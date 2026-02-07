import { useQueryClient } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import { BrowserRouter, Navigate, Route, Routes } from 'react-router-dom';
import { useLogout, useMe } from './api/generated/endpoints/auth/auth';
import { useListProjects } from './api/generated/endpoints/projects/projects';
import { KanbanBoard } from './components/KanbanBoard';
import { LoginPage } from './components/LoginPage';
import { ProjectCreateDialog } from './components/ProjectCreateDialog';
import { ProtectedRoute } from './components/ProtectedRoute';
import { RegisterPage } from './components/RegisterPage';
import { TaskCreateDialog } from './components/TaskCreateDialog';
import { TaskDetailModal } from './components/TaskDetailModal';
import { connectEventSource } from './hooks/useEventSource';
import { useAuthStore } from './stores/authStore';
import { useUiStore } from './stores/uiStore';

function KanbanApp() {
  const queryClient = useQueryClient();
  const [selectedProjectId, setSelectedProjectId] = useState<string | null>(null);
  const { data: projects, isLoading: projectsLoading } = useListProjects();
  const user = useAuthStore((state) => state.user);
  const logout = useLogout();
  const setUser = useAuthStore((state) => state.setUser);
  const openProjectModal = useUiStore((s) => s.openProjectModal);

  // Connect to SSE for real-time updates
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
      // If logout fails on server, still clear local state
      setUser(null);
    }
  };

  return (
    <div className="min-h-screen bg-gray-100">
      <header className="bg-white shadow">
        <div className="mx-auto flex max-w-7xl items-center justify-between px-4 py-4">
          <h1 className="text-2xl font-bold text-gray-900">Gantry Board</h1>
          <div className="flex items-center gap-4">
            <label htmlFor="project-select" className="text-sm font-medium text-gray-700">
              Project:
            </label>
            <select
              id="project-select"
              value={selectedProjectId ?? ''}
              onChange={(e) => setSelectedProjectId(e.target.value || null)}
              className="rounded-md border border-gray-300 px-3 py-1.5 text-sm focus:border-blue-500 focus:ring-1 focus:ring-blue-500"
              disabled={projectsLoading}
            >
              <option value="">Select a project...</option>
              {projects?.map((project) => (
                <option key={project.id} value={project.id}>
                  {project.name}
                </option>
              ))}
            </select>
            <button
              type="button"
              onClick={openProjectModal}
              className="rounded-md bg-blue-600 px-3 py-1.5 text-sm text-white hover:bg-blue-700"
            >
              New Project
            </button>

            <div className="flex items-center gap-2 border-l pl-4">
              <span className="text-sm text-gray-600">{user?.name ?? user?.email}</span>
              <button
                type="button"
                onClick={handleLogout}
                disabled={logout.isPending}
                className="rounded-md bg-gray-100 px-3 py-1.5 text-sm text-gray-700 hover:bg-gray-200"
              >
                Logout
              </button>
            </div>
          </div>
        </div>
      </header>
      <main>
        {selectedProjectId ? (
          <KanbanBoard projectId={selectedProjectId} />
        ) : (
          <div className="flex items-center justify-center p-12">
            <p className="text-gray-500">Select a project to view its tasks</p>
          </div>
        )}
      </main>
      <ProjectCreateDialog />
      {selectedProjectId && <TaskCreateDialog projectId={selectedProjectId} />}
      <TaskDetailModal />
    </div>
  );
}

function AuthProvider({ children }: { children: React.ReactNode }) {
  const setUser = useAuthStore((state) => state.setUser);
  const setLoading = useAuthStore((state) => state.setLoading);
  const isAuthenticated = useAuthStore((state) => state.isAuthenticated);

  // Only fetch /me if we think we're authenticated (from persisted state)
  const {
    data: user,
    isLoading,
    isError,
  } = useMe({
    query: {
      enabled: isAuthenticated,
      retry: false,
    },
  });

  useEffect(() => {
    if (isAuthenticated) {
      if (isLoading) {
        // Keep loading while /me query is in-flight
        setLoading(true);
      } else if (isError) {
        // Session expired or invalid
        setUser(null);
      } else if (user) {
        setUser(user);
      } else {
        setLoading(false);
      }
    } else {
      setLoading(false);
    }
  }, [user, isLoading, isError, isAuthenticated, setUser, setLoading]);

  return <>{children}</>;
}

export function AppRoutes() {
  return (
    <AuthProvider>
      <Routes>
        <Route path="/login" element={<LoginPage />} />
        <Route path="/register" element={<RegisterPage />} />
        <Route
          path="/"
          element={
            <ProtectedRoute>
              <KanbanApp />
            </ProtectedRoute>
          }
        />
        <Route path="*" element={<Navigate to="/" replace />} />
      </Routes>
    </AuthProvider>
  );
}

function App() {
  return (
    <BrowserRouter>
      <AppRoutes />
    </BrowserRouter>
  );
}

export default App;
