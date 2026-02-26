import { useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useCreateProject } from '@/api/generated/endpoints/projects/projects';
import { useEscapeKey } from '@/hooks/useEscapeKey';
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

  useEscapeKey(closeProjectModal);

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
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="project-create-title"
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
      onClick={(e) => {
        if (e.target === e.currentTarget) closeProjectModal();
      }}
    >
      <div
        data-testid="project-create-dialog"
        className="w-full max-w-md rounded-lg bg-white p-6 shadow-xl"
      >
        <h2 id="project-create-title" className="mb-4 text-lg font-semibold">
          {t('project.createProject')}
        </h2>
        {error && <div className="mb-4 rounded-md bg-red-50 p-3 text-sm text-red-700">{error}</div>}
        <form onSubmit={handleSubmit} className="space-y-4">
          <div>
            <label htmlFor="project-name" className="block text-sm font-medium text-gray-700">
              {t('project.name')}
            </label>
            <input
              id="project-name"
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              className="mt-1 block w-full rounded-md border border-gray-300 px-3 py-2 text-sm focus:border-blue-500 focus:ring-1 focus:ring-blue-500"
              autoFocus
            />
          </div>
          <div>
            <label
              htmlFor="project-description"
              className="block text-sm font-medium text-gray-700"
            >
              {t('project.description')}
            </label>
            <textarea
              id="project-description"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              rows={3}
              className="mt-1 block w-full rounded-md border border-gray-300 px-3 py-2 text-sm focus:border-blue-500 focus:ring-1 focus:ring-blue-500"
            />
          </div>
          <div>
            <label
              htmlFor="project-repository-path"
              className="block text-sm font-medium text-gray-700"
            >
              {t('project.repositoryPath')}
            </label>
            <input
              id="project-repository-path"
              type="text"
              value={repositoryPath}
              onChange={(e) => setRepositoryPath(e.target.value)}
              placeholder={t('project.repositoryPathPlaceholder')}
              className="mt-1 block w-full rounded-md border border-gray-300 px-3 py-2 text-sm focus:border-blue-500 focus:ring-1 focus:ring-blue-500"
            />
          </div>
          <div className="flex justify-end gap-3 pt-2">
            <button
              type="button"
              onClick={closeProjectModal}
              className="rounded-md border border-gray-300 px-4 py-2 text-sm text-gray-700 hover:bg-gray-50"
            >
              {t('common.cancel')}
            </button>
            <button
              type="submit"
              disabled={createProject.isPending}
              className="rounded-md bg-blue-600 px-4 py-2 text-sm text-white hover:bg-blue-700 disabled:opacity-50"
            >
              {t('common.create')}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
