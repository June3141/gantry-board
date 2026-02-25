import { useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useListMembers } from '@/api/generated/endpoints/project-members/project-members';
import { useCreateTask } from '@/api/generated/endpoints/tasks/tasks';
import { TaskPriority, TaskStatus } from '@/api/generated/model';
import { useEscapeKey } from '@/hooks/useEscapeKey';
import { invalidateTasks } from '@/services/queryInvalidation';
import { useUiStore } from '@/stores/uiStore';

interface TaskCreateDialogProps {
  projectId: string;
}

export function TaskCreateDialog({ projectId }: TaskCreateDialogProps) {
  const isOpen = useUiStore((s) => s.isTaskModalOpen);
  const defaultStatus = useUiStore((s) => s.defaultStatus);

  if (!isOpen) return null;

  return <TaskCreateForm projectId={projectId} defaultStatus={defaultStatus} />;
}

function TaskCreateForm({
  projectId,
  defaultStatus,
}: {
  projectId: string;
  defaultStatus: TaskStatus | null;
}) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const closeTaskModal = useUiStore((s) => s.closeTaskModal);
  const createTask = useCreateTask();
  const { data: members } = useListMembers(projectId);

  const [title, setTitle] = useState('');
  const [description, setDescription] = useState('');
  const [status, setStatus] = useState<TaskStatus>(defaultStatus ?? TaskStatus.backlog);
  const [priority, setPriority] = useState<TaskPriority>(TaskPriority.medium);
  const [assignedTo, setAssignedTo] = useState('');
  const [error, setError] = useState<string | null>(null);

  useEscapeKey(closeTaskModal);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!title.trim()) return;
    setError(null);

    try {
      await createTask.mutateAsync({
        data: {
          project_id: projectId,
          title: title.trim(),
          description: description.trim() || undefined,
          status,
          priority,
          ...(assignedTo ? { assigned_to: assignedTo } : {}),
        },
      });
      invalidateTasks(queryClient);
      closeTaskModal();
    } catch {
      setError(t('task.createFailed'));
    }
  };

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="task-create-title"
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
      onClick={(e) => {
        if (e.target === e.currentTarget) closeTaskModal();
      }}
    >
      <div
        data-testid="task-create-dialog"
        className="w-full max-w-md rounded-lg bg-white p-6 shadow-xl"
      >
        <h2 id="task-create-title" className="mb-4 text-lg font-semibold">
          {t('task.createTask')}
        </h2>
        {error && <div className="mb-4 rounded-md bg-red-50 p-3 text-sm text-red-700">{error}</div>}
        <form onSubmit={handleSubmit} className="space-y-4">
          <div>
            <label htmlFor="task-title" className="block text-sm font-medium text-gray-700">
              {t('task.title')}
            </label>
            <input
              id="task-title"
              type="text"
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              className="mt-1 block w-full rounded-md border border-gray-300 px-3 py-2 text-sm focus:border-blue-500 focus:ring-1 focus:ring-blue-500"
              autoFocus
            />
          </div>
          <div>
            <label htmlFor="task-description" className="block text-sm font-medium text-gray-700">
              {t('task.description')}
            </label>
            <textarea
              id="task-description"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              rows={3}
              className="mt-1 block w-full rounded-md border border-gray-300 px-3 py-2 text-sm focus:border-blue-500 focus:ring-1 focus:ring-blue-500"
            />
          </div>
          <div className="grid grid-cols-3 gap-4">
            <div>
              <label htmlFor="task-status" className="block text-sm font-medium text-gray-700">
                {t('task.status')}
              </label>
              <select
                id="task-status"
                value={status}
                onChange={(e) => setStatus(e.target.value as TaskStatus)}
                className="mt-1 block w-full rounded-md border border-gray-300 px-3 py-2 text-sm"
              >
                <option value={TaskStatus.backlog}>{t('board.status.backlog')}</option>
                <option value={TaskStatus.todo}>{t('board.status.todo')}</option>
                <option value={TaskStatus.in_progress}>{t('board.status.in_progress')}</option>
                <option value={TaskStatus.in_review}>{t('board.status.in_review')}</option>
                <option value={TaskStatus.done}>{t('board.status.done')}</option>
              </select>
            </div>
            <div>
              <label htmlFor="task-priority" className="block text-sm font-medium text-gray-700">
                {t('task.priority')}
              </label>
              <select
                id="task-priority"
                value={priority}
                onChange={(e) => setPriority(e.target.value as TaskPriority)}
                className="mt-1 block w-full rounded-md border border-gray-300 px-3 py-2 text-sm"
              >
                <option value={TaskPriority.low}>{t('board.priorityLabel.low')}</option>
                <option value={TaskPriority.medium}>{t('board.priorityLabel.medium')}</option>
                <option value={TaskPriority.high}>{t('board.priorityLabel.high')}</option>
                <option value={TaskPriority.urgent}>{t('board.priorityLabel.urgent')}</option>
              </select>
            </div>
            <div>
              <label htmlFor="task-assignee" className="block text-sm font-medium text-gray-700">
                {t('board.assignee')}
              </label>
              <select
                id="task-assignee"
                value={assignedTo}
                onChange={(e) => setAssignedTo(e.target.value)}
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
          </div>
          <div className="flex justify-end gap-3 pt-2">
            <button
              type="button"
              onClick={closeTaskModal}
              className="rounded-md border border-gray-300 px-4 py-2 text-sm text-gray-700 hover:bg-gray-50"
            >
              {t('common.cancel')}
            </button>
            <button
              type="submit"
              disabled={createTask.isPending}
              className="rounded-md bg-blue-600 px-4 py-2 text-sm text-white hover:bg-blue-700 disabled:opacity-50"
            >
              {t('common.create')}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
