import { useState } from 'react';
import {
  useCreatePreview,
  useDeletePreview,
  useListPreviews,
  useRestartPreview,
  useStartPreview,
  useStopPreview,
} from '@/api/generated/endpoints/previews/previews';
import type { DockerPreview } from '@/api/generated/model';

const statusColors: Record<string, string> = {
  pending: 'bg-yellow-100 text-yellow-800',
  building: 'bg-blue-100 text-blue-800',
  running: 'bg-green-100 text-green-800',
  stopped: 'bg-gray-100 text-gray-800',
  failed: 'bg-red-100 text-red-800',
};

function PreviewActions({
  preview,
  onStart,
  onStop,
  onRestart,
  onDelete,
}: {
  preview: DockerPreview;
  onStart: (id: string) => void;
  onStop: (id: string) => void;
  onRestart: (id: string) => void;
  onDelete: (id: string) => void;
}) {
  const { status, id } = preview;

  return (
    <div className="flex gap-1">
      {(status === 'pending' || status === 'stopped' || status === 'failed') && (
        <button
          type="button"
          onClick={() => onStart(id)}
          className="rounded bg-green-600 px-2 py-1 text-xs text-white hover:bg-green-700"
        >
          Start
        </button>
      )}
      {status === 'running' && (
        <>
          <button
            type="button"
            onClick={() => onStop(id)}
            className="rounded bg-yellow-600 px-2 py-1 text-xs text-white hover:bg-yellow-700"
          >
            Stop
          </button>
          <button
            type="button"
            onClick={() => onRestart(id)}
            className="rounded bg-blue-600 px-2 py-1 text-xs text-white hover:bg-blue-700"
          >
            Restart
          </button>
        </>
      )}
      <button
        type="button"
        onClick={() => onDelete(id)}
        className="rounded bg-red-600 px-2 py-1 text-xs text-white hover:bg-red-700"
      >
        Delete
      </button>
    </div>
  );
}

export function PreviewPanel() {
  const [worktreeName, setWorktreeName] = useState('');
  const [error, setError] = useState<string | null>(null);

  const { data: previews, isLoading, isError } = useListPreviews();
  const { mutateAsync: createPreview, isPending: isCreating } = useCreatePreview();
  const { mutateAsync: deletePreview } = useDeletePreview();
  const { mutateAsync: startPreview } = useStartPreview();
  const { mutateAsync: stopPreview } = useStopPreview();
  const { mutateAsync: restartPreview } = useRestartPreview();

  const handleCreate = async () => {
    setError(null);
    try {
      await createPreview({ data: { worktree_name: worktreeName } });
      setWorktreeName('');
    } catch {
      setError('Failed to create preview');
    }
  };

  const handleStart = async (id: string) => {
    setError(null);
    try {
      await startPreview({ id });
    } catch {
      setError('Failed to start preview');
    }
  };

  const handleStop = async (id: string) => {
    setError(null);
    try {
      await stopPreview({ id });
    } catch {
      setError('Failed to stop preview');
    }
  };

  const handleRestart = async (id: string) => {
    setError(null);
    try {
      await restartPreview({ id });
    } catch {
      setError('Failed to restart preview');
    }
  };

  const handleDelete = async (id: string) => {
    setError(null);
    try {
      await deletePreview({ id });
    } catch {
      setError('Failed to delete preview');
    }
  };

  if (isLoading) {
    return <div className="p-4 text-gray-500">Loading previews...</div>;
  }

  if (isError) {
    return <div className="p-4 text-red-500">Failed to load previews</div>;
  }

  return (
    <div className="space-y-4 p-4">
      <h2 className="text-lg font-semibold">Docker Previews</h2>

      {/* Create form */}
      <div className="flex gap-2">
        <label className="sr-only" htmlFor="preview-worktree-name">
          Worktree Name
        </label>
        <input
          id="preview-worktree-name"
          type="text"
          value={worktreeName}
          onChange={(e) => setWorktreeName(e.target.value)}
          placeholder="Worktree name"
          className="flex-1 rounded border px-3 py-1 text-sm"
        />
        <button
          type="button"
          onClick={handleCreate}
          disabled={!worktreeName.trim() || isCreating}
          className="rounded bg-indigo-600 px-3 py-1 text-sm text-white hover:bg-indigo-700 disabled:opacity-50"
        >
          Create
        </button>
      </div>

      {error && <div className="text-sm text-red-500">{error}</div>}

      {/* Preview list */}
      {previews && previews.length === 0 && <p className="text-sm text-gray-500">No previews</p>}

      {previews?.map((preview) => (
        <div key={preview.id} className="flex items-center justify-between rounded border p-3">
          <div className="space-y-1">
            <div className="flex items-center gap-2">
              <span className="font-medium text-sm">{preview.worktree_name}</span>
              <span
                className={`rounded-full px-2 py-0.5 text-xs font-medium ${statusColors[preview.status] ?? ''}`}
              >
                {preview.status}
              </span>
            </div>
            {preview.status === 'running' && preview.preview_url && (
              <a
                href={preview.preview_url}
                target="_blank"
                rel="noopener noreferrer"
                className="text-xs text-blue-600 hover:underline"
              >
                {preview.preview_url}
              </a>
            )}
            {preview.status === 'failed' && preview.error_message && (
              <p className="text-xs text-red-500">{preview.error_message}</p>
            )}
          </div>
          <PreviewActions
            preview={preview}
            onStart={handleStart}
            onStop={handleStop}
            onRestart={handleRestart}
            onDelete={handleDelete}
          />
        </div>
      ))}
    </div>
  );
}
