import { useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  getListProjectWorktreesQueryKey,
  useCreateProjectWorktree,
  useDeleteProjectWorktree,
  useListProjectWorktrees,
} from '@/api/generated/endpoints/worktrees/worktrees';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';

export function WorktreePanel({ projectId }: { projectId: string }) {
  const { t } = useTranslation();
  const [name, setName] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [deletingName, setDeletingName] = useState<string | null>(null);

  const queryClient = useQueryClient();
  const {
    data: worktrees,
    isLoading,
    isError,
  } = useListProjectWorktrees(projectId, { query: { enabled: !!projectId } });
  const createWorktree = useCreateProjectWorktree();
  const deleteWorktree = useDeleteProjectWorktree();

  const invalidateList = () =>
    queryClient.invalidateQueries({ queryKey: getListProjectWorktreesQueryKey(projectId) });

  const handleCreate = async () => {
    setError(null);
    try {
      await createWorktree.mutateAsync({ projectId, data: { name: name.trim() } });
      setName('');
      await invalidateList();
    } catch {
      setError(t('worktree.createFailed'));
    }
  };

  const handleDelete = async (worktreeName: string) => {
    setError(null);
    try {
      await deleteWorktree.mutateAsync({ projectId, name: worktreeName });
      setDeletingName(null);
      await invalidateList();
    } catch {
      setError(t('worktree.deleteFailed'));
    }
  };

  if (isLoading) {
    return <p className="text-sm text-gray-500">{t('worktree.loadingWorktrees')}</p>;
  }

  if (isError) {
    return <p className="text-sm text-red-500">{t('worktree.loadFailed')}</p>;
  }

  return (
    <div className="space-y-3">
      {error && <div className="rounded-md bg-red-50 p-3 text-sm text-red-700">{error}</div>}

      {worktrees && worktrees.length > 0 ? (
        <div className="space-y-1">
          {worktrees.map((wt) => (
            <div
              key={wt.name}
              className="flex items-center justify-between rounded-md border border-gray-200 px-3 py-2"
            >
              <div className="flex items-center gap-2 text-sm">
                <span className="font-medium text-gray-900">{wt.name}</span>
                {wt.branch && <span className="text-gray-500">{wt.branch}</span>}
                {!wt.is_valid && <Badge variant="destructive">{t('worktree.invalid')}</Badge>}
              </div>
              {deletingName === wt.name ? (
                <div className="flex items-center gap-2">
                  <span className="text-xs text-red-600">{t('worktree.deleteConfirm')}</span>
                  <Button
                    variant="outline"
                    size="xs"
                    onClick={() => setDeletingName(null)}
                    disabled={deleteWorktree.isPending}
                  >
                    {t('common.cancel')}
                  </Button>
                  <Button
                    variant="destructive"
                    size="xs"
                    onClick={() => handleDelete(wt.name)}
                    disabled={deleteWorktree.isPending}
                  >
                    {t('common.confirm')}
                  </Button>
                </div>
              ) : (
                <Button
                  variant="outline"
                  size="xs"
                  onClick={() => setDeletingName(wt.name)}
                  className="border-red-300 text-red-700 hover:bg-red-50"
                >
                  {t('common.delete')}
                </Button>
              )}
            </div>
          ))}
        </div>
      ) : (
        <p className="text-sm text-gray-500">{t('worktree.noWorktrees')}</p>
      )}

      <div className="flex gap-2">
        <div className="flex-1">
          <label htmlFor="worktree-name-input" className="sr-only">
            {t('worktree.worktreeName')}
          </label>
          <Input
            id="worktree-name-input"
            type="text"
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder={t('worktree.namePlaceholder')}
          />
        </div>
        <Button onClick={handleCreate} disabled={!name.trim() || createWorktree.isPending}>
          {t('common.create')}
        </Button>
      </div>
    </div>
  );
}
