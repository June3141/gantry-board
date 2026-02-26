import { FolderPlus } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useListProjects } from '@/api/generated/endpoints/projects/projects';
import { Button } from '@/components/ui/button';
import { useUiStore } from '@/stores/uiStore';
import { ProjectCard } from './ProjectCard';

export function ProjectListPage() {
  const { t } = useTranslation();
  const { data: projectsResponse, isLoading, isError } = useListProjects();
  const openProjectModal = useUiStore((s) => s.openProjectModal);
  const projects = projectsResponse?.data;

  if (isLoading) {
    return (
      <div className="flex items-center justify-center p-12">
        <p className="text-muted-foreground">{t('project.loadingProjects')}</p>
      </div>
    );
  }

  if (isError) {
    return (
      <div className="flex items-center justify-center p-12">
        <p className="text-destructive">{t('project.loadFailed')}</p>
      </div>
    );
  }

  return (
    <div className="mx-auto max-w-5xl px-4 py-8">
      <div className="mb-6 flex items-center justify-between">
        <h2 className="text-xl font-semibold text-foreground">{t('project.projects')}</h2>
        <Button size="sm" onClick={openProjectModal}>
          <FolderPlus className="h-4 w-4" /> {t('project.newProject')}
        </Button>
      </div>

      {!projects || projects.length === 0 ? (
        <div className="flex flex-col items-center justify-center rounded-lg border-2 border-dashed border-border py-16">
          <p className="text-muted-foreground">{t('project.noProjects')}</p>
          <Button className="mt-4" onClick={openProjectModal}>
            <FolderPlus className="h-4 w-4" /> {t('project.createFirst')}
          </Button>
        </div>
      ) : (
        <div className="grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3">
          {projects.map((project) => (
            <ProjectCard key={project.id} project={project} />
          ))}
        </div>
      )}
    </div>
  );
}
