import type { AgentSession, TaskComment } from '../api/generated/model';

export type TimelineItem =
  | { type: 'comment'; data: TaskComment }
  | { type: 'agent_session'; data: AgentSession };

export function mergeTimeline(
  _comments: TaskComment[],
  _sessions: AgentSession[],
): TimelineItem[] {
  return [];
}

export function TaskTimeline(_props: { taskId: string }) {
  return null;
}
