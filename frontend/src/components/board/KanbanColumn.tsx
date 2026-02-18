import { useDroppable } from '@dnd-kit/core';
import type { ProjectMember, Task } from '@/api/generated/model';
import { TaskStatus } from '@/api/generated/model';
import { useUiStore } from '@/stores/uiStore';
import { TaskCard } from './TaskCard';

interface KanbanColumnProps {
  status: TaskStatus;
  tasks: Task[];
  activeTaskId?: string | null;
  members?: ProjectMember[];
}

const statusLabels: Record<TaskStatus, string> = {
  [TaskStatus.backlog]: 'Backlog',
  [TaskStatus.todo]: 'To Do',
  [TaskStatus.in_progress]: 'In Progress',
  [TaskStatus.in_review]: 'In Review',
  [TaskStatus.done]: 'Done',
};

export function KanbanColumn({ status, tasks, activeTaskId, members }: KanbanColumnProps) {
  const openTaskModal = useUiStore((s) => s.openTaskModal);
  const { setNodeRef, isOver } = useDroppable({
    id: status,
  });

  return (
    <div
      className={`flex min-h-[500px] w-72 flex-shrink-0 flex-col rounded-lg ${
        isOver ? 'bg-blue-50 ring-2 ring-blue-400' : 'bg-gray-100'
      }`}
    >
      <div className="flex items-center justify-between p-3">
        <h2 className="text-sm font-semibold text-gray-700">{statusLabels[status]}</h2>
        <span className="rounded-full bg-gray-200 px-2 py-0.5 text-xs font-medium text-gray-600">
          {tasks.length}
        </span>
      </div>
      <div ref={setNodeRef} className="flex-1 space-y-2 overflow-y-auto p-2">
        {tasks.length === 0 ? (
          <div
            data-testid="column-empty"
            className="flex h-24 items-center justify-center rounded border-2 border-dashed border-gray-300 text-sm text-gray-400"
          >
            No tasks
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
        <button
          type="button"
          onClick={() => openTaskModal(status)}
          className="w-full rounded-md py-1.5 text-sm text-gray-500 hover:bg-gray-200 hover:text-gray-700"
        >
          + Add Task
        </button>
      </div>
    </div>
  );
}
