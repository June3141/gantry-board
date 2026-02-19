import type { QueryClient } from '@tanstack/react-query';

export function invalidateTasks(queryClient: QueryClient) {
  queryClient.invalidateQueries({ queryKey: ['/api/tasks'], exact: false });
}

export function invalidateProjects(queryClient: QueryClient) {
  queryClient.invalidateQueries({ queryKey: ['/api/projects'], exact: false });
}

export function invalidateComments(queryClient: QueryClient, taskId: string) {
  queryClient.invalidateQueries({
    queryKey: [`/api/tasks/${taskId}/comments`],
    exact: false,
  });
}

export function invalidateSessions(queryClient: QueryClient, taskId: string) {
  queryClient.invalidateQueries({
    queryKey: [`/api/tasks/${taskId}/sessions`],
    exact: false,
  });
}

export function invalidateMembers(queryClient: QueryClient, projectId: string) {
  queryClient.invalidateQueries({
    queryKey: [`/api/projects/${projectId}/members`],
    exact: false,
  });
}

export function invalidateWorktrees(queryClient: QueryClient) {
  queryClient.invalidateQueries({ queryKey: ['/api/worktrees'], exact: false });
}

export function invalidatePreviews(queryClient: QueryClient) {
  queryClient.invalidateQueries({ queryKey: ['/api/previews'], exact: false });
}

export function invalidateMessages(queryClient: QueryClient, projectId: string) {
  queryClient.invalidateQueries({
    queryKey: [`/api/projects/${projectId}/messages`],
    exact: false,
  });
}

export function invalidateGithubLinks(queryClient: QueryClient, projectId: string) {
  queryClient.invalidateQueries({
    queryKey: [`/api/projects/${projectId}/github-link`],
    exact: false,
  });
}
