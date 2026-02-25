import { useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  getListProjectWorktreesQueryKey,
  useCreateProjectWorktree,
  useDeleteProjectWorktree,
  useListProjectWorktrees,
} from '@/api/generated/endpoints/worktrees/worktrees';

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
                {!wt.is_valid && (
                  <span className="inline-flex items-center rounded-full bg-red-100 px-2 py-0.5 text-xs font-medium text-red-800">
                    {t('worktree.invalid')}
                  </span>
                )}
              </div>
              {deletingName === wt.name ? (
                <div className="flex items-center gap-2">
                  <span className="text-xs text-red-600">{t('worktree.deleteConfirm')}</span>
                  <button
                    type="button"
                    onClick={() => setDeletingName(null)}
                    disabled={deleteWorktree.isPending}
                    className="rounded border border-gray-300 px-2 py-0.5 text-xs text-gray-700 hover:bg-gray-50 disabled:opacity-50"
                  >
                    {t('common.cancel')}
                  </button>
                  <button
                    type="button"
                    onClick={() => handleDelete(wt.name)}
                    disabled={deleteWorktree.isPending}
                    className="rounded bg-red-600 px-2 py-0.5 text-xs text-white hover:bg-red-700 disabled:opacity-50"
                  >
                    {t('common.confirm')}
                  </button>
                </div>
              ) : (
                <button
                  type="button"
                  onClick={() => setDeletingName(wt.name)}
                  className="rounded border border-red-300 px-2 py-1 text-xs text-red-700 hover:bg-red-50"
                >
                  {t('common.delete')}
                </button>
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
          <input
            id="worktree-name-input"
            type="text"
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder={t('worktree.namePlaceholder')}
            className="block w-full rounded-md border border-gray-300 px-3 py-2 text-sm focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500"
          />
        </div>
        <button
          type="button"
          onClick={handleCreate}
          disabled={!name.trim() || createWorktree.isPending}
          className="rounded-md bg-blue-600 px-4 py-2 text-sm text-white hover:bg-blue-700 disabled:opacity-50"
        >
          {t('common.create')}
        </button>
      </div>
    </div>
  );
}
