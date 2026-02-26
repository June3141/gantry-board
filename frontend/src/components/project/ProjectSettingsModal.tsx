import { useQueryClient } from '@tanstack/react-query';
import { useCallback, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useListMembers } from '@/api/generated/endpoints/project-members/project-members';
import {
  getGetProjectQueryKey,
  getListProjectsQueryKey,
  useDeleteProject,
  useGetProject,
  useUpdateProject,
} from '@/api/generated/endpoints/projects/projects';
import { MemberRole } from '@/api/generated/model';
import { Button } from '@/components/ui/button';
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Textarea } from '@/components/ui/textarea';
import { useAuthStore } from '@/stores/authStore';
import { useToastStore } from '@/stores/toastStore';
import { useUiStore } from '@/stores/uiStore';
import { GitHubLinkSettings } from '../github/GitHubLinkSettings';

export function ProjectSettingsModal({
  projectId,
  onProjectDeleted,
}: {
  projectId: string;
  onProjectDeleted: () => void;
}) {
  const isOpen = useUiStore((s) => s.isProjectSettingsOpen);
  if (!isOpen) return null;
  return <ProjectSettingsContent projectId={projectId} onProjectDeleted={onProjectDeleted} />;
}

function ProjectSettingsContent({
  projectId,
  onProjectDeleted,
}: {
  projectId: string;
  onProjectDeleted: () => void;
}) {
  const { t } = useTranslation();
  const closeProjectSettings = useUiStore((s) => s.closeProjectSettings);
  const { data: project, isLoading, isError } = useGetProject(projectId);
  const { data: members } = useListMembers(projectId);
  const updateProject = useUpdateProject();
  const deleteProject = useDeleteProject();
  const queryClient = useQueryClient();
  const currentUser = useAuthStore((s) => s.user);
  const addToast = useToastStore((s) => s.addToast);

  const [editingField, setEditingField] = useState<
    'name' | 'description' | 'repository_path' | null
  >(null);
  const [editValue, setEditValue] = useState('');
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);

  const currentUserRole = members?.find((m) => m.user_id === currentUser?.id)?.role;
  const canEdit = currentUserRole === MemberRole.owner || currentUserRole === MemberRole.admin;
  const isOwner = currentUserRole === MemberRole.owner;

  const handleEscapeKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (editingField) {
        e.preventDefault();
        setEditingField(null);
      }
    },
    [editingField],
  );

  const startEditing = (field: 'name' | 'description' | 'repository_path', value: string) => {
    setEditingField(field);
    setEditValue(value);
  };

  const saveField = async (field: 'name' | 'description' | 'repository_path') => {
    const trimmed = editValue.trim();
    if (field === 'name' && !trimmed) {
      setEditingField(null);
      return;
    }
    const currentValue =
      field === 'name'
        ? project?.name
        : field === 'description'
          ? project?.description
          : project?.repository_path;
    try {
      if (trimmed !== (currentValue ?? '')) {
        await updateProject.mutateAsync({
          id: projectId,
          data: { [field]: trimmed },
        });
        queryClient.invalidateQueries({
          queryKey: getListProjectsQueryKey(),
        });
        queryClient.invalidateQueries({
          queryKey: getGetProjectQueryKey(projectId),
        });
        const fieldLabelKey =
          field === 'repository_path' ? 'project.repositoryPath' : `project.${field}`;
        addToast('success', t('project.fieldUpdated', { field: t(fieldLabelKey) }));
      }
    } catch {
      const fieldLabelKey =
        field === 'repository_path' ? 'project.repositoryPath' : `project.${field}`;
      addToast('error', t('project.fieldUpdateFailed', { field: t(fieldLabelKey) }));
    } finally {
      setEditingField(null);
    }
  };

  const handleDelete = async () => {
    try {
      await deleteProject.mutateAsync({ id: projectId });
      queryClient.invalidateQueries({
        queryKey: getListProjectsQueryKey(),
      });
      closeProjectSettings();
      onProjectDeleted();
      addToast('success', t('project.deleted'));
    } catch {
      addToast('error', t('project.deleteFailed'));
    }
  };

  return (
    <Dialog
      open={true}
      onOpenChange={(open) => {
        if (!open) closeProjectSettings();
      }}
    >
      <DialogContent
        className="max-w-md"
        onEscapeKeyDown={handleEscapeKeyDown}
        aria-describedby={undefined}
      >
        <DialogHeader>
          <DialogTitle>{t('project.settings')}</DialogTitle>
        </DialogHeader>

        {isLoading ? (
          <p className="text-sm text-muted-foreground">{t('common.loading')}</p>
        ) : isError || !project ? (
          <p className="text-sm text-destructive">{t('project.loadFailed')}</p>
        ) : (
          <div className="space-y-4">
            <div>
              <h3 className="text-sm font-medium text-foreground">{t('project.name')}</h3>
              {editingField === 'name' ? (
                <Input
                  type="text"
                  value={editValue}
                  onChange={(e) => setEditValue(e.target.value)}
                  onBlur={() => saveField('name')}
                  className="mt-1"
                  autoFocus
                />
              ) : canEdit ? (
                <button
                  type="button"
                  className="mt-1 cursor-pointer rounded px-1 text-left text-sm text-foreground hover:bg-accent"
                  onClick={() => startEditing('name', project.name)}
                >
                  {project.name}
                </button>
              ) : (
                <p className="mt-1 px-1 text-sm text-foreground">{project.name}</p>
              )}
            </div>

            <div>
              <h3 className="text-sm font-medium text-foreground">{t('project.description')}</h3>
              {editingField === 'description' ? (
                <Textarea
                  value={editValue}
                  onChange={(e) => setEditValue(e.target.value)}
                  onBlur={() => saveField('description')}
                  rows={3}
                  className="mt-1"
                  autoFocus
                />
              ) : canEdit ? (
                <button
                  type="button"
                  className="mt-1 cursor-pointer rounded px-1 text-left text-sm text-muted-foreground hover:bg-accent"
                  onClick={() => startEditing('description', project.description ?? '')}
                >
                  {project.description || t('common.noDescription')}
                </button>
              ) : (
                <p className="mt-1 px-1 text-sm text-muted-foreground">
                  {project.description || t('common.noDescription')}
                </p>
              )}
            </div>

            <div>
              <h3 className="text-sm font-medium text-foreground">{t('project.repositoryPath')}</h3>
              {editingField === 'repository_path' ? (
                <Input
                  type="text"
                  value={editValue}
                  onChange={(e) => setEditValue(e.target.value)}
                  onBlur={() => saveField('repository_path')}
                  placeholder="/path/to/git/repo"
                  className="mt-1"
                  autoFocus
                />
              ) : canEdit ? (
                <button
                  type="button"
                  className="mt-1 cursor-pointer rounded px-1 text-left text-sm text-muted-foreground hover:bg-accent"
                  onClick={() => startEditing('repository_path', project.repository_path ?? '')}
                >
                  {project.repository_path || t('project.notSetGlobal')}
                </button>
              ) : (
                <p className="mt-1 px-1 text-sm text-muted-foreground">
                  {project.repository_path || t('project.notSetGlobal')}
                </p>
              )}
            </div>

            {canEdit && (
              <div className="border-t pt-4">
                <h3 className="text-sm font-medium text-foreground mb-2">{t('github.github')}</h3>
                <GitHubLinkSettings projectId={projectId} />
              </div>
            )}

            {isOwner && (
              <div className="border-t pt-4">
                {showDeleteConfirm ? (
                  <div className="flex items-center justify-between rounded-md bg-destructive/10 p-3">
                    <p className="text-sm text-destructive">{t('project.deleteConfirm')}</p>
                    <div className="flex gap-2">
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() => setShowDeleteConfirm(false)}
                      >
                        {t('common.cancel')}
                      </Button>
                      <Button variant="destructive" size="sm" onClick={handleDelete}>
                        {t('common.confirm')}
                      </Button>
                    </div>
                  </div>
                ) : (
                  <Button
                    variant="outline"
                    className="border-destructive/30 text-destructive hover:bg-destructive/10"
                    onClick={() => setShowDeleteConfirm(true)}
                  >
                    {t('project.deleteProject')}
                  </Button>
                )}
              </div>
            )}
          </div>
        )}
      </DialogContent>
    </Dialog>
  );
}
