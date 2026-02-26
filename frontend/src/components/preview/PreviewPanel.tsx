import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  useCreatePreview,
  useDeletePreview,
  useListPreviews,
  useRestartPreview,
  useStartPreview,
  useStopPreview,
} from '@/api/generated/endpoints/previews/previews';
import type { DockerPreview } from '@/api/generated/model';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';

const statusColors: Record<string, string> = {
  pending: 'bg-warning/15 text-warning',
  building: 'bg-primary/15 text-primary',
  running: 'bg-success/15 text-success',
  stopped: 'bg-muted text-muted-foreground',
  failed: 'bg-destructive/15 text-destructive',
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
  const { t } = useTranslation();
  const { status, id } = preview;

  return (
    <div className="flex gap-1">
      {(status === 'pending' || status === 'stopped' || status === 'failed') && (
        <Button size="xs" onClick={() => onStart(id)} className="bg-success hover:bg-success/80">
          {t('preview.start')}
        </Button>
      )}
      {status === 'running' && (
        <>
          <Button size="xs" onClick={() => onStop(id)} className="bg-warning hover:bg-warning/80">
            {t('preview.stop')}
          </Button>
          <Button size="xs" onClick={() => onRestart(id)}>
            {t('preview.restart')}
          </Button>
        </>
      )}
      <Button variant="destructive" size="xs" onClick={() => onDelete(id)}>
        {t('preview.delete')}
      </Button>
    </div>
  );
}

export function PreviewPanel() {
  const { t } = useTranslation();
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
      setError(t('preview.createFailed'));
    }
  };

  const handleStart = async (id: string) => {
    setError(null);
    try {
      await startPreview({ id });
    } catch {
      setError(t('preview.startFailed'));
    }
  };

  const handleStop = async (id: string) => {
    setError(null);
    try {
      await stopPreview({ id });
    } catch {
      setError(t('preview.stopFailed'));
    }
  };

  const handleRestart = async (id: string) => {
    setError(null);
    try {
      await restartPreview({ id });
    } catch {
      setError(t('preview.restartFailed'));
    }
  };

  const handleDelete = async (id: string) => {
    setError(null);
    try {
      await deletePreview({ id });
    } catch {
      setError(t('preview.deleteFailed'));
    }
  };

  if (isLoading) {
    return <div className="p-4 text-muted-foreground">{t('preview.loadingPreviews')}</div>;
  }

  if (isError) {
    return <div className="p-4 text-destructive">{t('preview.loadFailed')}</div>;
  }

  return (
    <div className="space-y-4 p-4">
      <h2 className="text-lg font-semibold">{t('preview.dockerPreviews')}</h2>

      {/* Create form */}
      <div className="flex gap-2">
        <label className="sr-only" htmlFor="preview-worktree-name">
          {t('preview.worktreeNameLabel')}
        </label>
        <Input
          id="preview-worktree-name"
          type="text"
          value={worktreeName}
          onChange={(e) => setWorktreeName(e.target.value)}
          placeholder={t('preview.worktreeName')}
          className="flex-1"
        />
        <Button
          onClick={handleCreate}
          disabled={!worktreeName.trim() || isCreating}
          className="bg-primary hover:bg-primary/80"
        >
          {t('common.create')}
        </Button>
      </div>

      {error && <div className="text-sm text-destructive">{error}</div>}

      {/* Preview list */}
      {previews && previews.length === 0 && (
        <p className="text-sm text-muted-foreground">{t('preview.noPreviews')}</p>
      )}

      {previews?.map((preview) => (
        <div key={preview.id} className="flex items-center justify-between rounded border p-3">
          <div className="space-y-1">
            <div className="flex items-center gap-2">
              <span className="font-medium text-sm">{preview.worktree_name}</span>
              <Badge className={statusColors[preview.status] ?? ''}>{preview.status}</Badge>
            </div>
            {preview.status === 'running' && preview.preview_url && (
              <a
                href={preview.preview_url}
                target="_blank"
                rel="noopener noreferrer"
                className="text-xs text-primary hover:underline"
              >
                {preview.preview_url}
              </a>
            )}
            {preview.status === 'failed' && preview.error_message && (
              <p className="text-xs text-destructive">{preview.error_message}</p>
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
