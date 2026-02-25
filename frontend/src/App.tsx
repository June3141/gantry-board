import { useEffect } from 'react';
import { BrowserRouter, Navigate, Route, Routes } from 'react-router-dom';
import { useMe } from '@/api/generated/endpoints/auth/auth';
import { ProjectBoardPage } from '@/components/board';
import { ErrorBoundary, ToastContainer } from '@/components/common';
import {
  AppLayout,
  GuestRoute,
  InvitationAcceptPage,
  LoginPage,
  ProtectedRoute,
  RegisterPage,
} from '@/components/layout';
import { ProjectCreateDialog, ProjectListPage } from '@/components/project';
import { TaskDetailPage } from '@/components/task';
import { useAuthStore } from '@/stores/authStore';

function AuthProvider({ children }: { children: React.ReactNode }) {
  const setUser = useAuthStore((state) => state.setUser);
  const setLoading = useAuthStore((state) => state.setLoading);
  const isAuthenticated = useAuthStore((state) => state.isAuthenticated);

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
        setLoading(true);
      } else if (isError) {
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
        <Route
          path="/login"
          element={
            <GuestRoute>
              <LoginPage />
            </GuestRoute>
          }
        />
        <Route
          path="/register"
          element={
            <GuestRoute>
              <RegisterPage />
            </GuestRoute>
          }
        />
        <Route path="/invite/:token" element={<InvitationAcceptPage />} />
        <Route
          element={
            <ProtectedRoute>
              <AppLayout />
            </ProtectedRoute>
          }
        >
          <Route index element={<ProjectListPage />} />
          <Route path="projects/:projectId" element={<ProjectBoardPage />} />
          <Route path="projects/:projectId/tasks/:taskId" element={<TaskDetailPage />} />
        </Route>
        <Route path="*" element={<Navigate to="/" replace />} />
      </Routes>
      <ProjectCreateDialog />
    </AuthProvider>
  );
}

function App() {
  return (
    <BrowserRouter>
      <ErrorBoundary>
        <AppRoutes />
      </ErrorBoundary>
      <ToastContainer />
    </BrowserRouter>
  );
}

export default App;
