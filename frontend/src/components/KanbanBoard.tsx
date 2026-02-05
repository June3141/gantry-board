import type { DragEndEvent, DragStartEvent } from '@dnd-kit/core';
import {
  DndContext,
  DragOverlay,
  KeyboardSensor,
  PointerSensor,
  useSensor,
  useSensors,
} from '@dnd-kit/core';
import { useQueryClient } from '@tanstack/react-query';
import { useCallback, useMemo, useState } from 'react';
import {
  getListTasksQueryKey,
  useListTasks,
  useUpdateTask,
} from '../api/generated/endpoints/tasks/tasks';
import type { Task } from '../api/generated/model';
import { TaskStatus } from '../api/generated/model';
import { KanbanColumn } from './KanbanColumn';
import { TaskCard } from './TaskCard';

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
  const queryClient = useQueryClient();
  const [activeTask, setActiveTask] = useState<Task | null>(null);
  const { data: tasks, isLoading, error } = useListTasks({ project_id: projectId });

  const sensors = useSensors(useSensor(PointerSensor), useSensor(KeyboardSensor));

  const updateTaskMutation = useUpdateTask({
    mutation: {
      onMutate: async ({ id, data }) => {
        // Cancel outgoing refetches
        await queryClient.cancelQueries({
          queryKey: getListTasksQueryKey({ project_id: projectId }),
        });

        // Snapshot previous value
        const previousTasks = queryClient.getQueryData<Task[]>(
          getListTasksQueryKey({ project_id: projectId }),
        );

        // Optimistically update - only update status since that's what we change on drag
        queryClient.setQueryData<Task[]>(getListTasksQueryKey({ project_id: projectId }), (old) =>
          old?.map((task) =>
            task.id === id && data.status ? { ...task, status: data.status } : task,
          ),
        );

        return { previousTasks };
      },
      onError: (_err, _variables, context) => {
        // Rollback on error
        if (context?.previousTasks) {
          queryClient.setQueryData(
            getListTasksQueryKey({ project_id: projectId }),
            context.previousTasks,
          );
        }
      },
      onSettled: () => {
        // Refetch after mutation
        queryClient.invalidateQueries({
          queryKey: getListTasksQueryKey({ project_id: projectId }),
        });
      },
    },
  });

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

  const handleDragStart = useCallback((event: DragStartEvent) => {
    const task = event.active.data.current?.task as Task | undefined;
    if (task) {
      setActiveTask(task);
    }
  }, []);

  const handleDragEnd = useCallback(
    (event: DragEndEvent) => {
      const { active, over } = event;
      setActiveTask(null);

      if (!over) return;

      const taskId = active.id as string;
      const newStatus = over.id as TaskStatus;
      const task = tasks?.find((t) => t.id === taskId);

      if (task && task.status !== newStatus) {
        updateTaskMutation.mutate({
          id: taskId,
          data: { status: newStatus },
        });
      }
    },
    [tasks, updateTaskMutation],
  );

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
    <DndContext sensors={sensors} onDragStart={handleDragStart} onDragEnd={handleDragEnd}>
      <div className="flex gap-4 overflow-x-auto p-4">
        {COLUMN_ORDER.map((status) => (
          <KanbanColumn
            key={status}
            status={status}
            tasks={tasksByStatus[status]}
            activeTaskId={activeTask?.id}
          />
        ))}
      </div>
      <DragOverlay>{activeTask ? <TaskCard task={activeTask} /> : null}</DragOverlay>
    </DndContext>
  );
}
