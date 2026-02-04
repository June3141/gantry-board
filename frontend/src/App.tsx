import { useQueryClient } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import { useListProjects } from './api/generated/endpoints/projects/projects';
import { KanbanBoard } from './components/KanbanBoard';
import { connectEventSource } from './hooks/useEventSource';

function App() {
  const queryClient = useQueryClient();
  const [selectedProjectId, setSelectedProjectId] = useState<string | null>(null);
  const { data: projects, isLoading: projectsLoading } = useListProjects();

  // Connect to SSE for real-time updates (queryClient is stable from provider)
  useEffect(() => {
    const cleanup = connectEventSource(queryClient);
    return cleanup;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <div className="min-h-screen bg-gray-100">
      <header className="bg-white shadow">
        <div className="mx-auto flex max-w-7xl items-center justify-between px-4 py-4">
          <h1 className="text-2xl font-bold text-gray-900">Gantry Board</h1>
          <div className="flex items-center gap-4">
            <label htmlFor="project-select" className="text-sm font-medium text-gray-700">
              Project:
            </label>
            <select
              id="project-select"
              value={selectedProjectId ?? ''}
              onChange={(e) => setSelectedProjectId(e.target.value || null)}
              className="rounded-md border border-gray-300 px-3 py-1.5 text-sm focus:border-blue-500 focus:ring-1 focus:ring-blue-500"
              disabled={projectsLoading}
            >
              <option value="">Select a project...</option>
              {projects?.map((project) => (
                <option key={project.id} value={project.id}>
                  {project.name}
                </option>
              ))}
            </select>
          </div>
        </div>
      </header>
      <main>
        {selectedProjectId ? (
          <KanbanBoard projectId={selectedProjectId} />
        ) : (
          <div className="flex items-center justify-center p-12">
            <p className="text-gray-500">Select a project to view its tasks</p>
          </div>
        )}
      </main>
    </div>
  );
}

export default App;
