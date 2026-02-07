import { useEffect } from 'react';
import { useGetTask } from '../api/generated/endpoints/tasks/tasks';
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

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') closeTaskDetail();
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [closeTaskDetail]);

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
              <h2 className="text-lg font-semibold text-gray-900">{task.title}</h2>
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
              <p className="mt-1 text-sm text-gray-600">
                {task.description || 'No description'}
              </p>
            </div>

            <div className="grid grid-cols-2 gap-4">
              <div>
                <h3 className="text-sm font-medium text-gray-700">Status</h3>
                <p className="mt-1 text-sm text-gray-600">{task.status}</p>
              </div>
              <div>
                <h3 className="text-sm font-medium text-gray-700">Priority</h3>
                <p className="mt-1 text-sm text-gray-600">{task.priority}</p>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
