import { FolderPlus } from 'lucide-react';
import { useListProjects } from '@/api/generated/endpoints/projects/projects';
import { useUiStore } from '@/stores/uiStore';
import { ProjectCard } from './ProjectCard';

export function ProjectListPage() {
  const { data: projectsResponse, isLoading, isError } = useListProjects();
  const openProjectModal = useUiStore((s) => s.openProjectModal);
  const projects = projectsResponse?.data;

  if (isLoading) {
    return (
      <div className="flex items-center justify-center p-12">
        <p className="text-gray-500">Loading projects...</p>
      </div>
    );
  }

  if (isError) {
    return (
      <div className="flex items-center justify-center p-12">
        <p className="text-red-500">Failed to load projects.</p>
      </div>
    );
  }

  return (
    <div className="mx-auto max-w-5xl px-4 py-8">
      <div className="mb-6 flex items-center justify-between">
        <h2 className="text-xl font-semibold text-gray-900">Projects</h2>
        <button
          type="button"
          onClick={openProjectModal}
          className="flex items-center gap-1.5 rounded-md bg-blue-600 px-3 py-1.5 text-sm text-white hover:bg-blue-700"
        >
          <FolderPlus className="h-4 w-4" /> New Project
        </button>
      </div>

      {!projects || projects.length === 0 ? (
        <div className="flex flex-col items-center justify-center rounded-lg border-2 border-dashed border-gray-300 py-16">
          <p className="text-gray-500">No projects yet</p>
          <button
            type="button"
            onClick={openProjectModal}
            className="mt-4 flex items-center gap-1.5 rounded-md bg-blue-600 px-4 py-2 text-sm text-white hover:bg-blue-700"
          >
            <FolderPlus className="h-4 w-4" /> Create your first project
          </button>
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
