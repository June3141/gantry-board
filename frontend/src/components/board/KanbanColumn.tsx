import { useDroppable } from '@dnd-kit/core';
import { Plus } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { ProjectMember, Task } from '@/api/generated/model';
import { TaskStatus } from '@/api/generated/model';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { useUiStore } from '@/stores/uiStore';
import { TaskCard } from './TaskCard';

interface KanbanColumnProps {
  status: TaskStatus;
  tasks: Task[];
  activeTaskId?: string | null;
  members?: ProjectMember[];
}

const statusLabelKeys: Record<TaskStatus, string> = {
  [TaskStatus.backlog]: 'board.status.backlog',
  [TaskStatus.todo]: 'board.status.todo',
  [TaskStatus.in_progress]: 'board.status.in_progress',
  [TaskStatus.in_review]: 'board.status.in_review',
  [TaskStatus.done]: 'board.status.done',
};

const statusColors: Record<TaskStatus, { bg: string; badge: string }> = {
  [TaskStatus.backlog]: { bg: 'bg-muted', badge: 'bg-muted text-muted-foreground' },
  [TaskStatus.todo]: { bg: 'bg-primary/10', badge: 'bg-primary/20 text-primary' },
  [TaskStatus.in_progress]: { bg: 'bg-warning/10', badge: 'bg-warning/20 text-warning' },
  [TaskStatus.in_review]: { bg: 'bg-purple-50', badge: 'bg-purple-200 text-purple-700' },
  [TaskStatus.done]: { bg: 'bg-success/10', badge: 'bg-success/20 text-success' },
};

export function KanbanColumn({ status, tasks, activeTaskId, members }: KanbanColumnProps) {
  const { t } = useTranslation();
  const openTaskModal = useUiStore((s) => s.openTaskModal);
  const { setNodeRef, isOver } = useDroppable({
    id: status,
  });

  return (
    <div
      className={`flex min-h-[500px] w-72 flex-shrink-0 flex-col rounded-lg ${
        isOver ? 'bg-primary/10 ring-2 ring-ring' : statusColors[status].bg
      }`}
    >
      <div className="flex items-center justify-between p-3">
        <h2 className="text-sm font-semibold text-foreground">{t(statusLabelKeys[status])}</h2>
        <Badge className={statusColors[status].badge}>{tasks.length}</Badge>
      </div>
      <div ref={setNodeRef} className="flex-1 space-y-2 overflow-y-auto p-2">
        {tasks.length === 0 ? (
          <div
            data-testid="column-empty"
            className="flex h-24 items-center justify-center rounded border-2 border-dashed border-border text-sm text-muted-foreground"
          >
            {t('board.noTasks')}
          </div>
        ) : (
          tasks.map((task) => (
            <TaskCard
              key={task.id}
              task={task}
              isDragging={activeTaskId === task.id}
              members={members}
            />
          ))
        )}
      </div>
      <div className="p-2">
        <Button
          variant="ghost"
          size="sm"
          onClick={() => openTaskModal(status)}
          className="w-full text-muted-foreground"
        >
          <Plus className="h-4 w-4" /> {t('board.addTask')}
        </Button>
      </div>
    </div>
  );
}
