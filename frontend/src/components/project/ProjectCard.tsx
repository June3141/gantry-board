import { Settings } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { Link } from 'react-router';
import type { Project } from '@/api/generated/model';
import { Button } from '@/components/ui/button';

interface ProjectCardProps {
  project: Project;
  onSettings?: (projectId: string) => void;
}

export function ProjectCard({ project, onSettings }: ProjectCardProps) {
  const { t } = useTranslation();
  return (
    <div className="group relative rounded-lg border border-border bg-background p-5 shadow-sm transition-shadow hover:shadow-md">
      <Link to={`/projects/${project.id}`} className="block">
        <h3 className="text-lg font-semibold text-foreground">{project.name}</h3>
        {project.description && (
          <p className="mt-1 line-clamp-2 text-sm text-muted-foreground">{project.description}</p>
        )}
      </Link>
      {onSettings && (
        <Button
          variant="ghost"
          size="icon-xs"
          onClick={(e) => {
            e.stopPropagation();
            onSettings(project.id);
          }}
          className="absolute right-3 top-3 opacity-0 group-hover:opacity-100"
          aria-label={t('project.projectSettings')}
        >
          <Settings className="h-4 w-4" />
        </Button>
      )}
    </div>
  );
}
