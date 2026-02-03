import { useMemo } from 'react';
import { useListTasks } from '../api/generated/endpoints/tasks/tasks';
import type { Task } from '../api/generated/model';
import { TaskStatus } from '../api/generated/model';
import { KanbanColumn } from './KanbanColumn';

interface KanbanBoardProps {
  projectId: string;
}

const COLUMN_ORDER: TaskStatus[] = [
  TaskStatus.backlog,
  TaskStatus.todo,
  TaskStatus.in_progress,
  TaskStatus.in_review,
  TaskStatus.done,
];

export function KanbanBoard({ projectId }: KanbanBoardProps) {
  const { data: tasks, isLoading, error } = useListTasks({ project_id: projectId });

  const tasksByStatus = useMemo(() => {
    const grouped: Record<TaskStatus, Task[]> = {
      [TaskStatus.backlog]: [],
      [TaskStatus.todo]: [],
      [TaskStatus.in_progress]: [],
      [TaskStatus.in_review]: [],
      [TaskStatus.done]: [],
    };

    if (tasks) {
      for (const task of tasks) {
        grouped[task.status].push(task);
      }
    }

    return grouped;
  }, [tasks]);

  if (isLoading) {
    return (
      <div data-testid="kanban-loading" className="flex items-center justify-center p-8">
        <div className="text-gray-500">Loading tasks...</div>
      </div>
    );
  }

  if (error) {
    return (
      <div data-testid="kanban-error" className="flex items-center justify-center p-8">
        <div className="text-red-500">Failed to load tasks</div>
      </div>
    );
  }

  return (
    <div className="flex gap-4 overflow-x-auto p-4">
      {COLUMN_ORDER.map((status) => (
        <KanbanColumn key={status} status={status} tasks={tasksByStatus[status]} />
      ))}
    </div>
  );
}
