import { useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  getGetGithubLinkQueryKey,
  useCreateGithubLink,
  useDeleteGithubLink,
  useGetGithubLink,
  useSyncGithubLink,
} from '@/api/generated/endpoints/github-links/github-links';
import { useToastStore } from '@/stores/toastStore';

export function GitHubLinkSettings({ projectId }: { projectId: string }) {
  const { t } = useTranslation();
  const { data: link, isLoading, isError } = useGetGithubLink(projectId);
  const createLink = useCreateGithubLink();
  const deleteLink = useDeleteGithubLink();
  const syncLink = useSyncGithubLink();
  const queryClient = useQueryClient();
  const addToast = useToastStore((s) => s.addToast);

  const [repoOwner, setRepoOwner] = useState('');
  const [repoName, setRepoName] = useState('');
  const [showUnlinkConfirm, setShowUnlinkConfirm] = useState(false);

  const invalidate = () =>
    queryClient.invalidateQueries({ queryKey: getGetGithubLinkQueryKey(projectId) });

  const handleCreate = async () => {
    try {
      await createLink.mutateAsync({
        projectId,
        data: { repo_owner: repoOwner.trim(), repo_name: repoName.trim() },
      });
      setRepoOwner('');
      setRepoName('');
      await invalidate();
      addToast('success', t('github.linked'));
    } catch {
      addToast('error', t('github.linkFailed'));
    }
  };

  const handleSync = async () => {
    try {
      const result = await syncLink.mutateAsync({ projectId });
      await invalidate();
      addToast(
        'success',
        t('github.syncComplete', { pushed: result.pushed, pulled: result.pulled }),
      );
    } catch {
      addToast('error', t('github.syncFailed'));
    }
  };

  const handleUnlink = async () => {
    try {
      await deleteLink.mutateAsync({ projectId });
      setShowUnlinkConfirm(false);
      await invalidate();
      addToast('success', t('github.unlinked'));
    } catch {
      addToast('error', t('github.unlinkFailed'));
    }
  };

  if (isLoading) {
    return <p className="text-sm text-gray-500">{t('common.loading')}</p>;
  }

  if (isError || !link) {
    return (
      <div className="space-y-2">
        <div className="flex gap-2">
          <div className="flex-1">
            <label htmlFor="github-owner" className="block text-xs font-medium text-gray-600">
              {t('github.owner')}
            </label>
            <input
              id="github-owner"
              type="text"
              value={repoOwner}
              onChange={(e) => setRepoOwner(e.target.value)}
              placeholder={t('github.ownerPlaceholder')}
              className="mt-0.5 block w-full rounded-md border border-gray-300 px-2 py-1.5 text-sm focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500"
            />
          </div>
          <div className="flex-1">
            <label htmlFor="github-repo" className="block text-xs font-medium text-gray-600">
              {t('github.repository')}
            </label>
            <input
              id="github-repo"
              type="text"
              value={repoName}
              onChange={(e) => setRepoName(e.target.value)}
              placeholder={t('github.repoPlaceholder')}
              className="mt-0.5 block w-full rounded-md border border-gray-300 px-2 py-1.5 text-sm focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500"
            />
          </div>
        </div>
        <button
          type="button"
          onClick={handleCreate}
          disabled={!repoOwner.trim() || !repoName.trim() || createLink.isPending}
          className="rounded-md bg-blue-600 px-3 py-1.5 text-sm text-white hover:bg-blue-700 disabled:opacity-50"
        >
          {t('github.link')}
        </button>
      </div>
    );
  }

  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between rounded-md border border-gray-200 px-3 py-2">
        <span className="text-sm font-medium text-gray-900">
          {link.repo_owner}/{link.repo_name}
        </span>
        <div className="flex gap-2">
          <button
            type="button"
            onClick={handleSync}
            disabled={syncLink.isPending}
            className="rounded border border-blue-300 px-2 py-1 text-xs text-blue-700 hover:bg-blue-50 disabled:opacity-50"
          >
            {t('github.sync')}
          </button>
          {showUnlinkConfirm ? (
            <div className="flex items-center gap-2">
              <span className="text-xs text-red-600">{t('common.areYouSure')}</span>
              <button
                type="button"
                onClick={() => setShowUnlinkConfirm(false)}
                disabled={deleteLink.isPending}
                className="rounded border border-gray-300 px-2 py-0.5 text-xs text-gray-700 hover:bg-gray-50 disabled:opacity-50"
              >
                {t('common.cancel')}
              </button>
              <button
                type="button"
                onClick={handleUnlink}
                disabled={deleteLink.isPending}
                className="rounded bg-red-600 px-2 py-0.5 text-xs text-white hover:bg-red-700 disabled:opacity-50"
              >
                {t('common.confirm')}
              </button>
            </div>
          ) : (
            <button
              type="button"
              onClick={() => setShowUnlinkConfirm(true)}
              className="rounded border border-red-300 px-2 py-1 text-xs text-red-700 hover:bg-red-50"
            >
              {t('github.unlink')}
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
