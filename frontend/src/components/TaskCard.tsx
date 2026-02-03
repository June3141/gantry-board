import { useDraggable } from '@dnd-kit/core';
import type { Task } from '../api/generated/model';
import { TaskPriority } from '../api/generated/model';

interface TaskCardProps {
  task: Task;
  isDragging?: boolean;
}

const priorityStyles: Record<TaskPriority, string> = {
  [TaskPriority.urgent]: 'bg-red-100 text-red-800',
  [TaskPriority.high]: 'bg-orange-100 text-orange-800',
  [TaskPriority.medium]: 'bg-yellow-100 text-yellow-800',
  [TaskPriority.low]: 'bg-gray-100 text-gray-800',
};

export function TaskCard({ task, isDragging }: TaskCardProps) {
  const { attributes, listeners, setNodeRef, transform } = useDraggable({
    id: task.id,
    data: { task },
  });

  const style = transform
    ? {
        transform: `translate3d(${transform.x}px, ${transform.y}px, 0)`,
      }
    : undefined;

  return (
    <div
      ref={setNodeRef}
      style={style}
      {...listeners}
      {...attributes}
      className={`cursor-grab rounded-lg border border-gray-200 bg-white p-3 shadow-sm ${
        isDragging ? 'opacity-50' : ''
      }`}
    >
      <div className="mb-2 flex items-start justify-between">
        <h3 className="text-sm font-medium text-gray-900">{task.title}</h3>
        <span
          className={`rounded px-2 py-0.5 text-xs font-medium ${priorityStyles[task.priority]}`}
        >
          {task.priority}
        </span>
      </div>
      {task.description && (
        <p data-testid="task-description" className="text-xs text-gray-500">
          {task.description}
        </p>
      )}
    </div>
  );
}
