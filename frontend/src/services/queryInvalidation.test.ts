import { QueryClient } from '@tanstack/react-query';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import {
  invalidateComments,
  invalidateGithubLinks,
  invalidateMembers,
  invalidatePreviews,
  invalidateProjects,
  invalidateSessions,
  invalidateTasks,
  invalidateWorktrees,
} from './queryInvalidation';

describe('queryInvalidation', () => {
  let queryClient: QueryClient;

  beforeEach(() => {
    queryClient = new QueryClient();
    vi.spyOn(queryClient, 'invalidateQueries').mockResolvedValue();
  });

  it('invalidateTasks invalidates /api/tasks prefix', () => {
    invalidateTasks(queryClient);
    expect(queryClient.invalidateQueries).toHaveBeenCalledWith({
      queryKey: ['/api/tasks'],
      exact: false,
    });
  });

  it('invalidateProjects invalidates /api/projects prefix', () => {
    invalidateProjects(queryClient);
    expect(queryClient.invalidateQueries).toHaveBeenCalledWith({
      queryKey: ['/api/projects'],
      exact: false,
    });
  });

  it('invalidateComments invalidates task-specific comments', () => {
    invalidateComments(queryClient, 'task-123');
    expect(queryClient.invalidateQueries).toHaveBeenCalledWith({
      queryKey: ['/api/tasks/task-123/comments'],
      exact: false,
    });
  });

  it('invalidateSessions invalidates task-specific sessions', () => {
    invalidateSessions(queryClient, 'task-456');
    expect(queryClient.invalidateQueries).toHaveBeenCalledWith({
      queryKey: ['/api/tasks/task-456/sessions'],
      exact: false,
    });
  });

  it('invalidateMembers invalidates project-specific members', () => {
    invalidateMembers(queryClient, 'proj-789');
    expect(queryClient.invalidateQueries).toHaveBeenCalledWith({
      queryKey: ['/api/projects/proj-789/members'],
      exact: false,
    });
  });

  it('invalidateWorktrees invalidates worktrees', () => {
    invalidateWorktrees(queryClient);
    expect(queryClient.invalidateQueries).toHaveBeenCalledWith({
      queryKey: ['/api/worktrees'],
      exact: false,
    });
  });

  it('invalidatePreviews invalidates previews', () => {
    invalidatePreviews(queryClient);
    expect(queryClient.invalidateQueries).toHaveBeenCalledWith({
      queryKey: ['/api/previews'],
      exact: false,
    });
  });

  it('invalidateGithubLinks invalidates project-specific github link', () => {
    invalidateGithubLinks(queryClient, 'proj-abc');
    expect(queryClient.invalidateQueries).toHaveBeenCalledWith({
      queryKey: ['/api/projects/proj-abc/github-link'],
      exact: false,
    });
  });
});
