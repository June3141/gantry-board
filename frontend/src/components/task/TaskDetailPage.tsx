import { useQueryClient } from '@tanstack/react-query';
import { ArrowLeft } from 'lucide-react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Link, useNavigate, useParams } from 'react-router-dom';
import { useListMembers } from '@/api/generated/endpoints/project-members/project-members';
import {
  getGetTaskQueryKey,
  useDeleteTask,
  useGetTask,
  useUpdateTask,
} from '@/api/generated/endpoints/tasks/tasks';
import type { TaskPriority, TaskStatus } from '@/api/generated/model';
import { invalidateTasks } from '@/services/queryInvalidation';
import { PullRequestList } from '../github/PullRequestList';
import { WorktreePanel } from '../preview/WorktreePanel';
import { TaskTimeline } from './TaskTimeline';

export function TaskDetailPage() {
  const { t } = useTranslation();
  const { projectId, taskId } = useParams<{ projectId: string; taskId: string }>();
  const navigate = useNavigate();
  const queryClient = useQueryClient();

  const { data: task, isLoading, isError } = useGetTask(taskId ?? '');
  const updateTask = useUpdateTask();
  const deleteTask = useDeleteTask();
  const { data: members } = useListMembers(projectId ?? '', {
    query: { enabled: !!projectId },
  });

  const [editingField, setEditingField] = useState<'title' | 'description' | null>(null);
  const [editValue, setEditValue] = useState('');
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const startEditing = (field: 'title' | 'description', value: string) => {
    setEditingField(field);
    setEditValue(value);
  };

  const saveField = async (field: 'title' | 'description') => {
    if (!taskId) return;
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
        queryClient.invalidateQueries({ queryKey: getGetTaskQueryKey(taskId) });
        invalidateTasks(queryClient);
      }
    } catch {
      setError(t('task.updateFailed', { field: t(`task.${field}`) }));
    } finally {
      setEditingField(null);
    }
  };

  const handleDelete = async () => {
    if (!taskId) return;
    try {
      await deleteTask.mutateAsync({ id: taskId });
      invalidateTasks(queryClient);
      navigate(`/projects/${projectId}`);
    } catch {
      setError(t('task.deleteFailed'));
    }
  };

  const handleAssigneeChange = async (value: string) => {
    if (!taskId) return;
    try {
      await updateTask.mutateAsync({
        id: taskId,
        data: { assigned_to: value || null },
      });
      queryClient.invalidateQueries({ queryKey: getGetTaskQueryKey(taskId) });
      invalidateTasks(queryClient);
    } catch {
      setError(t('task.assigneeFailed'));
    }
  };

  const handleSelectChange = async (
    field: 'status' | 'priority',
    value: TaskStatus | TaskPriority,
  ) => {
    if (!taskId) return;
    try {
      await updateTask.mutateAsync({
        id: taskId,
        data: { [field]: value },
      });
      queryClient.invalidateQueries({ queryKey: getGetTaskQueryKey(taskId) });
      invalidateTasks(queryClient);
    } catch {
      setError(t('task.updateFailed', { field: t(`task.${field}`) }));
    }
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center p-12">
        <p className="text-gray-500">{t('task.loadingTask')}</p>
      </div>
    );
  }

  if (isError || !task) {
    return (
      <div className="mx-auto max-w-4xl px-4 py-8">
        <Link
          to={`/projects/${projectId}`}
          className="mb-4 inline-flex items-center gap-1 text-sm text-gray-500 hover:text-gray-700"
        >
          <ArrowLeft className="h-4 w-4" /> {t('task.backToBoard')}
        </Link>
        <p className="text-red-500">{t('task.loadFailed')}</p>
      </div>
    );
  }

  return (
    <div className="mx-auto max-w-4xl px-4 py-8">
      <Link
        to={`/projects/${projectId}`}
        className="mb-6 inline-flex items-center gap-1 text-sm text-gray-500 hover:text-gray-700"
      >
        <ArrowLeft className="h-4 w-4" /> {t('task.backToBoard')}
      </Link>

      {error && <div className="mb-4 rounded-md bg-red-50 p-3 text-sm text-red-700">{error}</div>}

      <div className="grid grid-cols-1 gap-8 lg:grid-cols-3">
        {/* Main content */}
        <div className="space-y-6 lg:col-span-2">
          {/* Title */}
          <div>
            {editingField === 'title' ? (
              <input
                type="text"
                value={editValue}
                onChange={(e) => setEditValue(e.target.value)}
                onBlur={() => saveField('title')}
                className="w-full rounded border border-blue-300 px-3 py-2 text-xl font-semibold text-gray-900 focus:outline-none focus:ring-2 focus:ring-blue-500"
                autoFocus
              />
            ) : (
              <button
                type="button"
                className="w-full cursor-pointer rounded px-2 py-1 text-left text-xl font-semibold text-gray-900 hover:bg-gray-100"
                onClick={() => startEditing('title', task.title)}
              >
                {task.title}
              </button>
            )}
          </div>

          {/* Description */}
          <div>
            <h3 className="mb-1 text-sm font-medium text-gray-700">{t('task.description')}</h3>
            {editingField === 'description' ? (
              <textarea
                value={editValue}
                onChange={(e) => setEditValue(e.target.value)}
                onBlur={() => saveField('description')}
                rows={4}
                className="w-full rounded border border-blue-300 px-3 py-2 text-sm text-gray-600 focus:outline-none focus:ring-2 focus:ring-blue-500"
                autoFocus
              />
            ) : (
              <button
                type="button"
                className="w-full cursor-pointer rounded px-2 py-1 text-left text-sm text-gray-600 hover:bg-gray-100"
                onClick={() => startEditing('description', task.description ?? '')}
              >
                {task.description || t('common.noDescription')}
              </button>
            )}
          </div>

          {/* Activity */}
          <div className="border-t pt-6">
            <h3 className="mb-3 text-sm font-medium text-gray-700">{t('activity.title')}</h3>
            <TaskTimeline taskId={task.id} />
          </div>
        </div>

        {/* Sidebar */}
        <div className="space-y-6">
          <div>
            <label htmlFor="task-status" className="block text-sm font-medium text-gray-700">
              {t('task.status')}
            </label>
            <select
              id="task-status"
              value={task.status}
              onChange={(e) => handleSelectChange('status', e.target.value as TaskStatus)}
              className="mt-1 block w-full rounded-md border border-gray-300 px-3 py-2 text-sm"
            >
              <option value="backlog">{t('board.status.backlog')}</option>
              <option value="todo">{t('board.status.todo')}</option>
              <option value="in_progress">{t('board.status.in_progress')}</option>
              <option value="in_review">{t('board.status.in_review')}</option>
              <option value="done">{t('board.status.done')}</option>
            </select>
          </div>

          <div>
            <label htmlFor="task-priority" className="block text-sm font-medium text-gray-700">
              {t('task.priority')}
            </label>
            <select
              id="task-priority"
              value={task.priority}
              onChange={(e) => handleSelectChange('priority', e.target.value as TaskPriority)}
              className="mt-1 block w-full rounded-md border border-gray-300 px-3 py-2 text-sm"
            >
              <option value="low">{t('board.priorityLabel.low')}</option>
              <option value="medium">{t('board.priorityLabel.medium')}</option>
              <option value="high">{t('board.priorityLabel.high')}</option>
              <option value="urgent">{t('board.priorityLabel.urgent')}</option>
            </select>
          </div>

          <div>
            <label htmlFor="task-assignee" className="block text-sm font-medium text-gray-700">
              {t('board.assignee')}
            </label>
            <select
              id="task-assignee"
              value={task.assigned_to ?? ''}
              onChange={(e) => handleAssigneeChange(e.target.value)}
              className="mt-1 block w-full rounded-md border border-gray-300 px-3 py-2 text-sm"
            >
              <option value="">{t('common.unassigned')}</option>
              {members?.map((m) => (
                <option key={m.user_id} value={m.user_id}>
                  {m.user_name}
                </option>
              ))}
            </select>
          </div>

          <div className="border-t pt-4">
            <h3 className="mb-2 text-sm font-medium text-gray-700">{t('worktree.worktrees')}</h3>
            <WorktreePanel projectId={projectId ?? ''} />
          </div>

          <div className="border-t pt-4">
            <h3 className="mb-2 text-sm font-medium text-gray-700">{t('task.pullRequests')}</h3>
            <PullRequestList taskId={task.id} />
          </div>

          <div className="border-t pt-4">
            {showDeleteConfirm ? (
              <div className="rounded-md bg-red-50 p-3">
                <p className="mb-2 text-sm text-red-700">{t('task.deleteConfirm')}</p>
                <div className="flex gap-2">
                  <button
                    type="button"
                    onClick={() => setShowDeleteConfirm(false)}
                    className="rounded-md border border-gray-300 px-3 py-1 text-sm text-gray-700 hover:bg-gray-50"
                  >
                    {t('common.cancel')}
                  </button>
                  <button
                    type="button"
                    onClick={handleDelete}
                    className="rounded-md bg-red-600 px-3 py-1 text-sm text-white hover:bg-red-700"
                  >
                    {t('common.confirm')}
                  </button>
                </div>
              </div>
            ) : (
              <button
                type="button"
                onClick={() => setShowDeleteConfirm(true)}
                className="w-full rounded-md border border-red-300 px-4 py-2 text-sm text-red-700 hover:bg-red-50"
              >
                {t('task.deleteTask')}
              </button>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
