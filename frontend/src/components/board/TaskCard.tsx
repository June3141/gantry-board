import { useDraggable } from '@dnd-kit/core';
import { useRef } from 'react';
import type { ProjectMember, Task } from '@/api/generated/model';
import { TaskPriority } from '@/api/generated/model';
import { Badge } from '@/components/ui/badge';
import { useUiStore } from '@/stores/uiStore';

interface TaskCardProps {
  task: Task;
  isDragging?: boolean;
  members?: ProjectMember[];
}

function getInitials(name: string): string {
  return name
    .split(' ')
    .map((w) => w[0])
    .filter(Boolean)
    .slice(0, 2)
    .join('')
    .toUpperCase();
}

const CLICK_THRESHOLD = 5;

const priorityStyles: Record<TaskPriority, string> = {
  [TaskPriority.urgent]: 'bg-destructive/15 text-destructive',
  [TaskPriority.high]: 'bg-warning/25 text-warning',
  [TaskPriority.medium]: 'bg-warning/15 text-warning',
  [TaskPriority.low]: 'bg-muted text-muted-foreground',
};

export function TaskCard({ task, isDragging, members }: TaskCardProps) {
  const { attributes, listeners, setNodeRef, transform } = useDraggable({
    id: task.id,
    data: { task },
  });
  const openTaskDetail = useUiStore((s) => s.openTaskDetail);
  const pointerStart = useRef<{ x: number; y: number } | null>(null);

  const style = transform
    ? {
        transform: `translate3d(${transform.x}px, ${transform.y}px, 0)`,
      }
    : undefined;

  const handlePointerDown = (e: React.PointerEvent) => {
    pointerStart.current = { x: e.clientX, y: e.clientY };
    listeners?.onPointerDown?.(e);
  };

  const handleClick = (e: React.MouseEvent) => {
    if (pointerStart.current) {
      const dx = e.clientX - pointerStart.current.x;
      const dy = e.clientY - pointerStart.current.y;
      if (Math.sqrt(dx * dx + dy * dy) > CLICK_THRESHOLD) return;
    }
    openTaskDetail(task.id);
  };

  return (
    // biome-ignore lint/a11y/useSemanticElements: dnd-kit requires div with ref/listeners spread
    <div
      ref={setNodeRef}
      style={style}
      {...listeners}
      {...attributes}
      role="button"
      tabIndex={0}
      onPointerDown={handlePointerDown}
      onClick={handleClick}
      onKeyDown={(e) => {
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault();
          openTaskDetail(task.id);
        }
      }}
      data-testid="task-card"
      className={`cursor-pointer rounded-lg border border-border bg-background p-3 shadow-sm ${
        isDragging ? 'opacity-50' : ''
      }`}
    >
      <div className="mb-2 flex items-start justify-between">
        <h3 className="text-sm font-medium text-foreground">{task.title}</h3>
        <Badge className={`rounded ${priorityStyles[task.priority]}`}>{task.priority}</Badge>
      </div>
      {task.description && (
        <p data-testid="task-description" className="text-xs text-muted-foreground">
          {task.description}
        </p>
      )}
      {task.assigned_to &&
        (() => {
          const member = members?.find((m) => m.user_id === task.assigned_to);
          const initials = member ? getInitials(member.user_name) : '?';
          return (
            <div className="mt-2 flex justify-end">
              <div
                data-testid="assignee-avatar"
                className="flex h-6 w-6 items-center justify-center rounded-full bg-primary/15 text-xs font-medium text-primary"
                title={member?.user_name}
              >
                {initials}
              </div>
            </div>
          );
        })()}
    </div>
  );
}
