import type { AgentSession, TaskComment } from '../api/generated/model';

export type TimelineItem =
  | { type: 'comment'; data: TaskComment }
  | { type: 'agent_session'; data: AgentSession };

export function mergeTimeline(
  comments: TaskComment[],
  sessions: AgentSession[],
): TimelineItem[] {
  const items: TimelineItem[] = [
    ...comments.map((c) => ({ type: 'comment' as const, data: c })),
    ...sessions.map((s) => ({ type: 'agent_session' as const, data: s })),
  ];
  items.sort((a, b) => new Date(b.data.created_at).getTime() - new Date(a.data.created_at).getTime());
  return items;
}

export function TaskTimeline(_props: { taskId: string }) {
  return null;
}
