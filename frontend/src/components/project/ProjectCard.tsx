import { Settings } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { Link } from 'react-router-dom';
import type { Project } from '@/api/generated/model';

interface ProjectCardProps {
  project: Project;
  onSettings?: (projectId: string) => void;
}

export function ProjectCard({ project, onSettings }: ProjectCardProps) {
  const { t } = useTranslation();
  return (
    <div className="group relative rounded-lg border border-gray-200 bg-white p-5 shadow-sm transition-shadow hover:shadow-md">
      <Link to={`/projects/${project.id}`} className="block">
        <h3 className="text-lg font-semibold text-gray-900">{project.name}</h3>
        {project.description && (
          <p className="mt-1 line-clamp-2 text-sm text-gray-500">{project.description}</p>
        )}
      </Link>
      {onSettings && (
        <button
          type="button"
          onClick={(e) => {
            e.stopPropagation();
            onSettings(project.id);
          }}
          className="absolute right-3 top-3 rounded p-1 text-gray-400 opacity-0 hover:bg-gray-100 hover:text-gray-600 group-hover:opacity-100"
          aria-label={t('project.projectSettings')}
        >
          <Settings className="h-4 w-4" />
        </button>
      )}
    </div>
  );
}
