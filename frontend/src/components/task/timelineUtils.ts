import type {
  AgentSession,
  AgentSessionStatus,
  AgentType,
  TaskComment,
} from '@/api/generated/model';

export type TimelineItem =
  | { type: 'comment'; data: TaskComment }
  | { type: 'agent_session'; data: AgentSession };

export function mergeTimeline(comments: TaskComment[], sessions: AgentSession[]): TimelineItem[] {
  const items: TimelineItem[] = [
    ...comments.map((c) => ({ type: 'comment' as const, data: c })),
    ...sessions.map((s) => ({ type: 'agent_session' as const, data: s })),
  ];
  items.sort(
    (a, b) => new Date(b.data.created_at).getTime() - new Date(a.data.created_at).getTime(),
  );
  return items;
}

export const STATUS_COLORS: Record<AgentSessionStatus, string> = {
  pending: 'bg-warning/15 text-warning',
  running: 'bg-primary/15 text-primary',
  paused: 'bg-purple-100 text-purple-800',
  completed: 'bg-success/15 text-success',
  failed: 'bg-destructive/15 text-destructive',
  cancelled: 'bg-muted text-muted-foreground',
};

export const AGENT_LABELS: Record<AgentType, string> = {
  claude_code: 'Claude Code',
  gemini_cli: 'Gemini CLI',
};

export const TERMINAL_STATUSES: AgentSessionStatus[] = ['completed', 'failed', 'cancelled'];

export function timeAgo(dateStr: string): string {
  const now = Date.now();
  const then = new Date(dateStr).getTime();
  const diffSec = Math.floor((now - then) / 1000);
  if (diffSec < 60) return 'just now';
  const diffMin = Math.floor(diffSec / 60);
  if (diffMin < 60) return `${diffMin} min ago`;
  const diffHr = Math.floor(diffMin / 60);
  if (diffHr < 24) return `${diffHr}h ago`;
  const diffDay = Math.floor(diffHr / 24);
  return `${diffDay}d ago`;
}

export function getInitials(name: string): string {
  return name
    .split(' ')
    .map((w) => w[0])
    .filter(Boolean)
    .slice(0, 2)
    .join('')
    .toUpperCase();
}
