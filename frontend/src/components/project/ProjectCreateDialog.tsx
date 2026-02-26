import { useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useCreateProject } from '@/api/generated/endpoints/projects/projects';
import { Button } from '@/components/ui/button';
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Textarea } from '@/components/ui/textarea';
import { invalidateProjects } from '@/services/queryInvalidation';
import { useUiStore } from '@/stores/uiStore';

export function ProjectCreateDialog() {
  const isOpen = useUiStore((s) => s.isProjectModalOpen);

  if (!isOpen) return null;

  return <ProjectCreateForm />;
}

function ProjectCreateForm() {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const closeProjectModal = useUiStore((s) => s.closeProjectModal);
  const createProject = useCreateProject();

  const [name, setName] = useState('');
  const [description, setDescription] = useState('');
  const [repositoryPath, setRepositoryPath] = useState('');
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim()) return;
    setError(null);

    try {
      await createProject.mutateAsync({
        data: {
          name: name.trim(),
          description: description.trim() || undefined,
          repository_path: repositoryPath.trim() || undefined,
        },
      });
      invalidateProjects(queryClient);
      closeProjectModal();
    } catch {
      setError(t('project.createFailed'));
    }
  };

  return (
    <Dialog
      open={true}
      onOpenChange={(open) => {
        if (!open) closeProjectModal();
      }}
    >
      <DialogContent
        className="max-w-md"
        data-testid="project-create-dialog"
        aria-describedby={undefined}
      >
        <DialogHeader>
          <DialogTitle>{t('project.createProject')}</DialogTitle>
        </DialogHeader>
        {error && (
          <div className="rounded-md bg-destructive/10 p-3 text-sm text-destructive">{error}</div>
        )}
        <form onSubmit={handleSubmit} className="space-y-4">
          <div>
            <label htmlFor="project-name" className="block text-sm font-medium text-foreground">
              {t('project.name')}
            </label>
            <Input
              id="project-name"
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              className="mt-1"
            />
          </div>
          <div>
            <label
              htmlFor="project-description"
              className="block text-sm font-medium text-foreground"
            >
              {t('project.description')}
            </label>
            <Textarea
              id="project-description"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              rows={3}
              className="mt-1"
            />
          </div>
          <div>
            <label
              htmlFor="project-repository-path"
              className="block text-sm font-medium text-foreground"
            >
              {t('project.repositoryPath')}
            </label>
            <Input
              id="project-repository-path"
              type="text"
              value={repositoryPath}
              onChange={(e) => setRepositoryPath(e.target.value)}
              placeholder={t('project.repositoryPathPlaceholder')}
              className="mt-1"
            />
          </div>
          <div className="flex justify-end gap-3 pt-2">
            <Button type="button" variant="outline" onClick={closeProjectModal}>
              {t('common.cancel')}
            </Button>
            <Button type="submit" disabled={createProject.isPending}>
              {t('common.create')}
            </Button>
          </div>
        </form>
      </DialogContent>
    </Dialog>
  );
}
