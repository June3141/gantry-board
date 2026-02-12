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
import { useListMembers } from '../api/generated/endpoints/project-members/project-members';
import {
  getListTasksQueryKey,
  useListTasks,
  useUpdateTask,
} from '../api/generated/endpoints/tasks/tasks';
import type { PaginatedResponseTask, Task } from '../api/generated/model';
import { TaskStatus } from '../api/generated/model';
import { useBoardStore } from '../stores/boardStore';
import { KanbanColumn } from './KanbanColumn';
import { TaskCard } from './TaskCard';
import { TaskFilterBar } from './TaskFilterBar';

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
  const { data: tasksResponse, isLoading, error } = useListTasks({ project_id: projectId });
  const { data: members } = useListMembers(projectId);
  const tasks = tasksResponse?.data;
  const searchText = useBoardStore((s) => s.searchText);
  const assigneeFilter = useBoardStore((s) => s.assigneeFilter);
  const priorityFilter = useBoardStore((s) => s.priorityFilter);

  const sensors = useSensors(useSensor(PointerSensor), useSensor(KeyboardSensor));

  const updateTaskMutation = useUpdateTask({
    mutation: {
      onMutate: async ({ id, data }) => {
        // Cancel outgoing refetches
        await queryClient.cancelQueries({
          queryKey: getListTasksQueryKey({ project_id: projectId }),
        });

        // Snapshot previous value
        const previousTasks = queryClient.getQueryData<PaginatedResponseTask>(
          getListTasksQueryKey({ project_id: projectId }),
        );

        // Optimistically update - only update status since that's what we change on drag
        queryClient.setQueryData<PaginatedResponseTask>(
          getListTasksQueryKey({ project_id: projectId }),
          (old) =>
            old
              ? {
                  ...old,
                  data: old.data.map((task) =>
                    task.id === id && data.status ? { ...task, status: data.status } : task,
                  ),
                }
              : old,
        );

        return { previousTasks };
      },
      onError: (_err, _variables, context) => {
        // Rollback on error
        if (context?.previousTasks != null) {
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

  const filteredTasks = useMemo(() => {
    if (!tasks) return [];
    return tasks.filter((task) => {
      if (searchText && !task.title.toLowerCase().includes(searchText.toLowerCase())) {
        return false;
      }
      if (assigneeFilter.length > 0) {
        const isUnassigned = !task.assigned_to;
        const matchesAssignee = assigneeFilter.includes(task.assigned_to ?? '');
        const matchesUnassigned = assigneeFilter.includes('unassigned') && isUnassigned;
        if (!matchesAssignee && !matchesUnassigned) return false;
      }
      if (priorityFilter.length > 0 && !priorityFilter.includes(task.priority)) {
        return false;
      }
      return true;
    });
  }, [tasks, searchText, assigneeFilter, priorityFilter]);

  const tasksByStatus = useMemo(() => {
    const grouped: Record<TaskStatus, Task[]> = {
      [TaskStatus.backlog]: [],
      [TaskStatus.todo]: [],
      [TaskStatus.in_progress]: [],
      [TaskStatus.in_review]: [],
      [TaskStatus.done]: [],
    };

    for (const task of filteredTasks) {
      grouped[task.status].push(task);
    }

    return grouped;
  }, [filteredTasks]);

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
      <div className="px-4 pt-4">
        <TaskFilterBar members={members} />
      </div>
      <div className="flex gap-4 overflow-x-auto p-4">
        {COLUMN_ORDER.map((status) => (
          <KanbanColumn
            key={status}
            status={status}
            tasks={tasksByStatus[status]}
            activeTaskId={activeTask?.id}
            members={members}
          />
        ))}
      </div>
      <DragOverlay>{activeTask ? <TaskCard task={activeTask} members={members} /> : null}</DragOverlay>
    </DndContext>
  );
}
