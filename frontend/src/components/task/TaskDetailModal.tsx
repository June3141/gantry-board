import { useQueryClient } from '@tanstack/react-query';
import { X } from 'lucide-react';
import { useCallback, useState } from 'react';
import { useListMembers } from '@/api/generated/endpoints/project-members/project-members';
import { useDeleteTask, useGetTask, useUpdateTask } from '@/api/generated/endpoints/tasks/tasks';
import type { TaskPriority, TaskStatus } from '@/api/generated/model';
import { useEscapeKey } from '@/hooks/useEscapeKey';
import { invalidateTasks } from '@/services/queryInvalidation';
import { useUiStore } from '@/stores/uiStore';
import { PullRequestList } from '../github/PullRequestList';
import { WorktreePanel } from '../preview/WorktreePanel';
import { TaskTimeline } from './TaskTimeline';

export function TaskDetailModal() {
  const selectedTaskId = useUiStore((s) => s.selectedTaskId);
  const isOpen = useUiStore((s) => s.isTaskDetailOpen);

  if (!isOpen || !selectedTaskId) return null;

  return <TaskDetailContent taskId={selectedTaskId} />;
}

function TaskDetailContent({ taskId }: { taskId: string }) {
  const queryClient = useQueryClient();
  const closeTaskDetail = useUiStore((s) => s.closeTaskDetail);
  const { data: task, isLoading, isError } = useGetTask(taskId);
  const updateTask = useUpdateTask();
  const deleteTask = useDeleteTask();
  const { data: members } = useListMembers(task?.project_id ?? '', { query: { enabled: !!task } });

  const [editingField, setEditingField] = useState<'title' | 'description' | null>(null);
  const [editValue, setEditValue] = useState('');
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const escapeGuard = useCallback(() => {
    if (editingField) {
      setEditingField(null);
      return true;
    }
    return false;
  }, [editingField]);
  useEscapeKey(closeTaskDetail, escapeGuard);

  const startEditing = (field: 'title' | 'description', value: string) => {
    setEditingField(field);
    setEditValue(value);
  };

  const saveField = async (field: 'title' | 'description') => {
    const trimmed = editValue.trim();
    if (field === 'title' && !trimmed) {
      setEditingField(null);
      return;
    }
    const currentValue = field === 'title' ? task?.title : task?.description;
    try {
      if (trimmed !== (currentValue ?? '')) {
        await updateTask.mutateAsync({
          id: taskId,
          data: { [field]: trimmed },
        });
      }
    } catch {
      setError(`Failed to update ${field}. Please try again.`);
    } finally {
      setEditingField(null);
    }
  };

  const handleDelete = async () => {
    try {
      await deleteTask.mutateAsync({ id: taskId });
      invalidateTasks(queryClient);
      closeTaskDetail();
    } catch {
      setError('Failed to delete task. Please try again.');
    }
  };

  const handleAssigneeChange = async (value: string) => {
    try {
      await updateTask.mutateAsync({
        id: taskId,
        data: { assigned_to: value || null },
      });
    } catch {
      setError('Failed to update assignee. Please try again.');
    }
  };

  const handleSelectChange = async (
    field: 'status' | 'priority',
    value: TaskStatus | TaskPriority,
  ) => {
    try {
      await updateTask.mutateAsync({
        id: taskId,
        data: { [field]: value },
      });
      invalidateTasks(queryClient);
    } catch {
      setError(`Failed to update ${field}. Please try again.`);
    }
  };

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="task-detail-modal-title"
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
      onClick={(e) => {
        if (e.target === e.currentTarget) closeTaskDetail();
      }}
    >
      <div
        data-testid="task-detail-modal"
        className="w-full max-w-2xl rounded-lg bg-white p-6 shadow-xl"
      >
        {isLoading ? (
          <p className="text-sm text-gray-500">Loading...</p>
        ) : isError || !task ? (
          <p className="text-sm text-red-500">Failed to load task.</p>
        ) : (
          <div className="space-y-4">
            {error && <div className="rounded-md bg-red-50 p-3 text-sm text-red-700">{error}</div>}
            <div className="flex items-start justify-between">
              {editingField === 'title' ? (
                <input
                  id="task-detail-modal-title"
                  type="text"
                  value={editValue}
                  onChange={(e) => setEditValue(e.target.value)}
                  onBlur={() => saveField('title')}
                  className="flex-1 rounded border border-blue-300 px-2 py-1 text-lg font-semibold text-gray-900 focus:outline-none focus:ring-2 focus:ring-blue-500"
                  autoFocus
                />
              ) : (
                <button
                  id="task-detail-modal-title"
                  type="button"
                  className="cursor-pointer rounded px-1 text-left text-lg font-semibold text-gray-900 hover:bg-gray-100"
                  onClick={() => startEditing('title', task.title)}
                >
                  {task.title}
                </button>
              )}
              <button
                type="button"
                onClick={closeTaskDetail}
                className="text-gray-400 hover:text-gray-600"
                aria-label="Close"
              >
                <X className="h-5 w-5" />
              </button>
            </div>

            <div>
              <h3 className="text-sm font-medium text-gray-700">Description</h3>
              {editingField === 'description' ? (
                <textarea
                  value={editValue}
                  onChange={(e) => setEditValue(e.target.value)}
                  onBlur={() => saveField('description')}
                  rows={3}
                  className="mt-1 block w-full rounded border border-blue-300 px-2 py-1 text-sm text-gray-600 focus:outline-none focus:ring-2 focus:ring-blue-500"
                  autoFocus
                />
              ) : (
                <button
                  type="button"
                  className="mt-1 cursor-pointer rounded px-1 text-left text-sm text-gray-600 hover:bg-gray-100"
                  onClick={() => startEditing('description', task.description ?? '')}
                >
                  {task.description || 'No description'}
                </button>
              )}
            </div>

            <div className="grid grid-cols-3 gap-4">
              <div>
                <label
                  htmlFor="task-detail-status"
                  className="block text-sm font-medium text-gray-700"
                >
                  Status
                </label>
                <select
                  id="task-detail-status"
                  value={task.status}
                  onChange={(e) => handleSelectChange('status', e.target.value as TaskStatus)}
                  className="mt-1 block w-full rounded-md border border-gray-300 px-3 py-2 text-sm"
                >
                  <option value="backlog">Backlog</option>
                  <option value="todo">To Do</option>
                  <option value="in_progress">In Progress</option>
                  <option value="in_review">In Review</option>
                  <option value="done">Done</option>
                </select>
              </div>
              <div>
                <label
                  htmlFor="task-detail-priority"
                  className="block text-sm font-medium text-gray-700"
                >
                  Priority
                </label>
                <select
                  id="task-detail-priority"
                  value={task.priority}
                  onChange={(e) => handleSelectChange('priority', e.target.value as TaskPriority)}
                  className="mt-1 block w-full rounded-md border border-gray-300 px-3 py-2 text-sm"
                >
                  <option value="low">Low</option>
                  <option value="medium">Medium</option>
                  <option value="high">High</option>
                  <option value="urgent">Urgent</option>
                </select>
              </div>
              <div>
                <label
                  htmlFor="task-detail-assignee"
                  className="block text-sm font-medium text-gray-700"
                >
                  Assignee
                </label>
                <select
                  id="task-detail-assignee"
                  value={task.assigned_to ?? ''}
                  onChange={(e) => handleAssigneeChange(e.target.value)}
                  className="mt-1 block w-full rounded-md border border-gray-300 px-3 py-2 text-sm"
                >
                  <option value="">Unassigned</option>
                  {members?.map((m) => (
                    <option key={m.user_id} value={m.user_id}>
                      {m.user_name}
                    </option>
                  ))}
                </select>
              </div>
            </div>

            <div className="border-t pt-4">
              <h3 className="text-sm font-medium text-gray-700 mb-2">Worktrees</h3>
              <WorktreePanel />
            </div>

            <div className="border-t pt-4">
              <h3 className="text-sm font-medium text-gray-700 mb-2">Pull Requests</h3>
              <PullRequestList taskId={taskId} />
            </div>

            <div className="border-t pt-4">
              <h3 className="text-sm font-medium text-gray-700 mb-2">Activity</h3>
              <TaskTimeline taskId={taskId} />
            </div>

            <div className="border-t pt-4">
              {showDeleteConfirm ? (
                <div className="flex items-center justify-between rounded-md bg-red-50 p-3">
                  <p className="text-sm text-red-700">Are you sure you want to delete this task?</p>
                  <div className="flex gap-2">
                    <button
                      type="button"
                      onClick={() => setShowDeleteConfirm(false)}
                      className="rounded-md border border-gray-300 px-3 py-1 text-sm text-gray-700 hover:bg-gray-50"
                    >
                      Cancel
                    </button>
                    <button
                      type="button"
                      onClick={handleDelete}
                      className="rounded-md bg-red-600 px-3 py-1 text-sm text-white hover:bg-red-700"
                    >
                      Confirm
                    </button>
                  </div>
                </div>
              ) : (
                <button
                  type="button"
                  onClick={() => setShowDeleteConfirm(true)}
                  className="rounded-md border border-red-300 px-4 py-2 text-sm text-red-700 hover:bg-red-50"
                >
                  Delete
                </button>
              )}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
