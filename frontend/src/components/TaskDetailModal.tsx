import { useEffect, useState } from 'react';
import { useDeleteTask, useGetTask, useUpdateTask } from '../api/generated/endpoints/tasks/tasks';
import type { TaskPriority, TaskStatus } from '../api/generated/model';
import { useUiStore } from '../stores/uiStore';

export function TaskDetailModal() {
  const selectedTaskId = useUiStore((s) => s.selectedTaskId);
  const isOpen = useUiStore((s) => s.isTaskDetailOpen);

  if (!isOpen || !selectedTaskId) return null;

  return <TaskDetailContent taskId={selectedTaskId} />;
}

function TaskDetailContent({ taskId }: { taskId: string }) {
  const closeTaskDetail = useUiStore((s) => s.closeTaskDetail);
  const { data: task, isLoading } = useGetTask(taskId);
  const updateTask = useUpdateTask();

  const deleteTask = useDeleteTask();

  const [editingField, setEditingField] = useState<'title' | 'description' | null>(null);
  const [editValue, setEditValue] = useState('');
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        if (editingField) {
          setEditingField(null);
        } else {
          closeTaskDetail();
        }
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [closeTaskDetail, editingField]);

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
    if (trimmed !== (currentValue ?? '')) {
      await updateTask.mutateAsync({
        id: taskId,
        data: { [field]: trimmed },
      });
    }
    setEditingField(null);
  };

  const handleDelete = async () => {
    await deleteTask.mutateAsync({ id: taskId });
    closeTaskDetail();
  };

  const handleSelectChange = async (field: 'status' | 'priority', value: string) => {
    await updateTask.mutateAsync({
      id: taskId,
      data: { [field]: value },
    });
  };

  return (
    <div
      role="dialog"
      aria-modal="true"
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
      onClick={(e) => {
        if (e.target === e.currentTarget) closeTaskDetail();
      }}
    >
      <div className="w-full max-w-lg rounded-lg bg-white p-6 shadow-xl">
        {isLoading || !task ? (
          <p className="text-sm text-gray-500">Loading...</p>
        ) : (
          <div className="space-y-4">
            <div className="flex items-start justify-between">
              {editingField === 'title' ? (
                <input
                  type="text"
                  value={editValue}
                  onChange={(e) => setEditValue(e.target.value)}
                  onBlur={() => saveField('title')}
                  className="flex-1 rounded border border-blue-300 px-2 py-1 text-lg font-semibold text-gray-900 focus:outline-none focus:ring-2 focus:ring-blue-500"
                  autoFocus
                />
              ) : (
                <h2
                  className="cursor-pointer text-lg font-semibold text-gray-900 hover:bg-gray-100 rounded px-1"
                  onClick={() => startEditing('title', task.title)}
                >
                  {task.title}
                </h2>
              )}
              <button
                type="button"
                onClick={closeTaskDetail}
                className="text-gray-400 hover:text-gray-600"
                aria-label="Close"
              >
                &times;
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
                <p
                  className="mt-1 cursor-pointer text-sm text-gray-600 hover:bg-gray-100 rounded px-1"
                  onClick={() => startEditing('description', task.description ?? '')}
                >
                  {task.description || 'No description'}
                </p>
              )}
            </div>

            <div className="grid grid-cols-2 gap-4">
              <div>
                <label htmlFor="task-detail-status" className="block text-sm font-medium text-gray-700">
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
                <label htmlFor="task-detail-priority" className="block text-sm font-medium text-gray-700">
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
