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
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
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
            <Input
              id="github-owner"
              type="text"
              value={repoOwner}
              onChange={(e) => setRepoOwner(e.target.value)}
              placeholder={t('github.ownerPlaceholder')}
              className="mt-0.5"
            />
          </div>
          <div className="flex-1">
            <label htmlFor="github-repo" className="block text-xs font-medium text-gray-600">
              {t('github.repository')}
            </label>
            <Input
              id="github-repo"
              type="text"
              value={repoName}
              onChange={(e) => setRepoName(e.target.value)}
              placeholder={t('github.repoPlaceholder')}
              className="mt-0.5"
            />
          </div>
        </div>
        <Button
          size="sm"
          onClick={handleCreate}
          disabled={!repoOwner.trim() || !repoName.trim() || createLink.isPending}
        >
          {t('github.link')}
        </Button>
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
          <Button
            variant="outline"
            size="xs"
            onClick={handleSync}
            disabled={syncLink.isPending}
          >
            {t('github.sync')}
          </Button>
          {showUnlinkConfirm ? (
            <div className="flex items-center gap-2">
              <span className="text-xs text-red-600">{t('common.areYouSure')}</span>
              <Button
                variant="outline"
                size="xs"
                onClick={() => setShowUnlinkConfirm(false)}
                disabled={deleteLink.isPending}
              >
                {t('common.cancel')}
              </Button>
              <Button
                variant="destructive"
                size="xs"
                onClick={handleUnlink}
                disabled={deleteLink.isPending}
              >
                {t('common.confirm')}
              </Button>
            </div>
          ) : (
            <Button
              variant="outline"
              size="xs"
              onClick={() => setShowUnlinkConfirm(true)}
              className="border-red-300 text-red-700 hover:bg-red-50"
            >
              {t('github.unlink')}
            </Button>
          )}
        </div>
      </div>
    </div>
  );
}
