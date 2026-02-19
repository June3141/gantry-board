import type { AgentSessionOutput, ProjectMember, Task, TaskComment } from '@/api/generated/model';
import { MemberRole, TaskPriority, TaskStatus } from '@/api/generated/model';

let idCounter = 0;

function nextId(): string {
  idCounter++;
  return `00000000-0000-0000-0000-${String(idCounter).padStart(12, '0')}`;
}

export function resetFactories(): void {
  idCounter = 0;
}

export function buildTask(overrides: Partial<Task> = {}): Task {
  const id = overrides.id ?? nextId();
  return {
    id,
    project_id: 'project-1',
    title: 'Test Task',
    description: 'Test description',
    status: TaskStatus.todo,
    priority: TaskPriority.medium,
    position: 0,
    created_at: '2026-01-01T00:00:00Z',
    updated_at: '2026-01-01T00:00:00Z',
    ...overrides,
  };
}

export function buildMember(overrides: Partial<ProjectMember> = {}): ProjectMember {
  const userId = overrides.user_id ?? nextId();
  return {
    user_id: userId,
    user_name: 'Test User',
    user_email: 'test@test.com',
    role: MemberRole.member,
    project_id: 'project-1',
    created_at: '2026-01-01T00:00:00Z',
    ...overrides,
  };
}

export function buildComment(overrides: Partial<TaskComment> = {}): TaskComment {
  const id = overrides.id ?? nextId();
  return {
    id,
    task_id: 'task-1',
    user_id: 'user-1',
    user_name: 'Test User',
    body: 'Test comment',
    created_at: '2026-01-01T00:00:00Z',
    updated_at: '2026-01-01T00:00:00Z',
    ...overrides,
  };
}

export function buildSessionOutput(
  overrides: Partial<AgentSessionOutput> = {},
): AgentSessionOutput {
  const id = overrides.id ?? nextId();
  return {
    id,
    session_id: 'session-1',
    output_type: 'assistant',
    content: 'Test output',
    created_at: '2026-01-01T00:00:00Z',
    ...overrides,
  };
}
