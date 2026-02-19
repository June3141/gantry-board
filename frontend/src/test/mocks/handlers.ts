import { HttpResponse, http } from 'msw';
import type { Task } from '@/api/generated/model';
import { buildMember, buildTask } from './factories';

const defaultTask = buildTask({ id: 'task-1' });
const defaultMembers = [
  buildMember({
    user_id: 'user-1',
    user_name: 'Alice',
    user_email: 'alice@test.com',
    role: 'owner' as ProjectMemberRole,
  }),
  buildMember({
    user_id: 'user-2',
    user_name: 'Bob',
    user_email: 'bob@test.com',
    role: 'member' as ProjectMemberRole,
  }),
];

type ProjectMemberRole = 'owner' | 'admin' | 'member';

/** Default handlers that serve stable mock data for TaskDetailModal tests. */
export const handlers = [
  // Tasks
  http.get('*/api/tasks/:id', ({ params }) => {
    if (params.id === defaultTask.id) {
      return HttpResponse.json(defaultTask);
    }
    return HttpResponse.json({ error: 'Not found' }, { status: 404 });
  }),

  http.patch('*/api/tasks/:id', async ({ params, request }) => {
    const body = (await request.json()) as Partial<Task>;
    return HttpResponse.json({ ...defaultTask, id: params.id, ...body });
  }),

  http.delete('*/api/tasks/:id', () => {
    return new HttpResponse(null, { status: 204 });
  }),

  // Agent sessions
  http.get('*/api/tasks/:taskId/sessions', () => {
    return HttpResponse.json([]);
  }),

  // Worktrees
  http.get('*/api/worktrees', () => {
    return HttpResponse.json([]);
  }),

  // Project members
  http.get('*/api/projects/:projectId/members', () => {
    return HttpResponse.json(defaultMembers);
  }),

  // Task comments
  http.get('*/api/tasks/:taskId/comments', () => {
    return HttpResponse.json([]);
  }),

  // Pull requests
  http.get('*/api/tasks/:taskId/pull-requests', () => {
    return HttpResponse.json([]);
  }),

  // Agent session outputs
  http.get('*/api/tasks/:taskId/sessions/:sessionId/outputs', () => {
    return HttpResponse.json([]);
  }),
];
