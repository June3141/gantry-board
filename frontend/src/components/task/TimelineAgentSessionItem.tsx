import type { AgentSession } from '@/api/generated/model';
import { AGENT_LABELS, STATUS_COLORS, timeAgo } from './timelineUtils';

export function TimelineAgentSessionItem({
  session,
  onView,
}: {
  session: AgentSession;
  onView: (session: AgentSession) => void;
}) {
  return (
    <button
      type="button"
      data-testid="timeline-session"
      className="flex w-full items-center gap-3 rounded-md bg-muted px-3 py-2 text-left hover:bg-accent"
      onClick={() => onView(session)}
    >
      <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-purple-100 text-xs font-medium text-purple-800">
        AI
      </div>
      <div className="flex flex-1 items-center gap-2">
        <span className="text-sm font-medium text-foreground">
          {AGENT_LABELS[session.agent_type] ?? session.agent_type}
        </span>
        <span
          className={`inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium ${STATUS_COLORS[session.status]}`}
        >
          {session.status}
        </span>
        <span className="text-xs text-muted-foreground">{timeAgo(session.created_at)}</span>
      </div>
    </button>
  );
}
