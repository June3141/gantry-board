import type { Task } from '../api/generated/model';
import { TaskStatus } from '../api/generated/model';
import { TaskCard } from './TaskCard';

interface KanbanColumnProps {
  status: TaskStatus;
  tasks: Task[];
}

const statusLabels: Record<TaskStatus, string> = {
  [TaskStatus.backlog]: 'Backlog',
  [TaskStatus.todo]: 'To Do',
  [TaskStatus.in_progress]: 'In Progress',
  [TaskStatus.in_review]: 'In Review',
  [TaskStatus.done]: 'Done',
};

export function KanbanColumn({ status, tasks }: KanbanColumnProps) {
  return (
    <div className="flex min-h-[500px] w-72 flex-shrink-0 flex-col rounded-lg bg-gray-100">
      <div className="flex items-center justify-between p-3">
        <h2 className="text-sm font-semibold text-gray-700">{statusLabels[status]}</h2>
        <span className="rounded-full bg-gray-200 px-2 py-0.5 text-xs font-medium text-gray-600">
          {tasks.length}
        </span>
      </div>
      <div className="flex-1 space-y-2 overflow-y-auto p-2">
        {tasks.length === 0 ? (
          <div
            data-testid="column-empty"
            className="flex h-24 items-center justify-center rounded border-2 border-dashed border-gray-300 text-sm text-gray-400"
          >
            No tasks
          </div>
        ) : (
          tasks.map((task) => <TaskCard key={task.id} task={task} />)
        )}
      </div>
    </div>
  );
}
