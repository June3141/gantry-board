import { useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useListMembers } from '@/api/generated/endpoints/project-members/project-members';
import { useCreateTask } from '@/api/generated/endpoints/tasks/tasks';
import { TaskPriority, TaskStatus } from '@/api/generated/model';
import { Button } from '@/components/ui/button';
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Textarea } from '@/components/ui/textarea';
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
    <Dialog
      open={true}
      onOpenChange={(open) => {
        if (!open) closeTaskModal();
      }}
    >
      <DialogContent
        className="max-w-md"
        data-testid="task-create-dialog"
        aria-describedby={undefined}
      >
        <DialogHeader>
          <DialogTitle>{t('task.createTask')}</DialogTitle>
        </DialogHeader>
        {error && (
          <div className="rounded-md bg-destructive/10 p-3 text-sm text-destructive">{error}</div>
        )}
        <form onSubmit={handleSubmit} className="space-y-4">
          <div>
            <label htmlFor="task-title" className="block text-sm font-medium text-foreground">
              {t('task.title')}
            </label>
            <Input
              id="task-title"
              type="text"
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              className="mt-1"
            />
          </div>
          <div>
            <label htmlFor="task-description" className="block text-sm font-medium text-foreground">
              {t('task.description')}
            </label>
            <Textarea
              id="task-description"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              rows={3}
              className="mt-1"
            />
          </div>
          <div className="grid grid-cols-3 gap-4">
            <div>
              <label htmlFor="task-status" className="block text-sm font-medium text-foreground">
                {t('task.status')}
              </label>
              <select
                id="task-status"
                value={status}
                onChange={(e) => setStatus(e.target.value as TaskStatus)}
                className="mt-1 block w-full rounded-md border border-input px-3 py-2 text-sm"
              >
                <option value={TaskStatus.backlog}>{t('board.status.backlog')}</option>
                <option value={TaskStatus.todo}>{t('board.status.todo')}</option>
                <option value={TaskStatus.in_progress}>{t('board.status.in_progress')}</option>
                <option value={TaskStatus.in_review}>{t('board.status.in_review')}</option>
                <option value={TaskStatus.done}>{t('board.status.done')}</option>
              </select>
            </div>
            <div>
              <label htmlFor="task-priority" className="block text-sm font-medium text-foreground">
                {t('task.priority')}
              </label>
              <select
                id="task-priority"
                value={priority}
                onChange={(e) => setPriority(e.target.value as TaskPriority)}
                className="mt-1 block w-full rounded-md border border-input px-3 py-2 text-sm"
              >
                <option value={TaskPriority.low}>{t('board.priorityLabel.low')}</option>
                <option value={TaskPriority.medium}>{t('board.priorityLabel.medium')}</option>
                <option value={TaskPriority.high}>{t('board.priorityLabel.high')}</option>
                <option value={TaskPriority.urgent}>{t('board.priorityLabel.urgent')}</option>
              </select>
            </div>
            <div>
              <label htmlFor="task-assignee" className="block text-sm font-medium text-foreground">
                {t('board.assignee')}
              </label>
              <select
                id="task-assignee"
                value={assignedTo}
                onChange={(e) => setAssignedTo(e.target.value)}
                className="mt-1 block w-full rounded-md border border-input px-3 py-2 text-sm"
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
            <Button type="button" variant="outline" onClick={closeTaskModal}>
              {t('common.cancel')}
            </Button>
            <Button type="submit" disabled={createTask.isPending}>
              {t('common.create')}
            </Button>
          </div>
        </form>
      </DialogContent>
    </Dialog>
  );
}
