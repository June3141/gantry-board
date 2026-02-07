import { useDraggable } from '@dnd-kit/core';
import { useRef } from 'react';
import type { Task } from '../api/generated/model';
import { TaskPriority } from '../api/generated/model';
import { useUiStore } from '../stores/uiStore';

interface TaskCardProps {
  task: Task;
  isDragging?: boolean;
}

const CLICK_THRESHOLD = 5;

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
    <div
      ref={setNodeRef}
      style={style}
      {...listeners}
      {...attributes}
      onPointerDown={handlePointerDown}
      onClick={handleClick}
      data-testid="task-card"
      className={`cursor-pointer rounded-lg border border-gray-200 bg-white p-3 shadow-sm ${
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
