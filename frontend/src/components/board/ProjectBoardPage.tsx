import { MessageSquare, Settings, Users } from 'lucide-react';
import { useNavigate, useParams } from 'react-router-dom';
import { KanbanBoard } from './KanbanBoard';
import { ProjectChatPanel, ProjectMembersPanel, ProjectSettingsModal } from '@/components/project';
import { TaskCreateDialog } from '@/components/task';
import { useUiStore } from '@/stores/uiStore';

export function ProjectBoardPage() {
  const { projectId } = useParams<{ projectId: string }>();
  const navigate = useNavigate();
  const openProjectSettings = useUiStore((s) => s.openProjectSettings);
  const openProjectMembers = useUiStore((s) => s.openProjectMembers);
  const openProjectChat = useUiStore((s) => s.openProjectChat);

  if (!projectId) return null;

  return (
    <>
      <div className="mx-auto flex max-w-7xl items-center gap-3 px-4 pt-4">
        <button
          type="button"
          onClick={openProjectSettings}
          className="flex items-center gap-1.5 rounded-md border border-gray-300 px-3 py-1.5 text-sm text-gray-700 hover:bg-gray-50"
        >
          <Settings className="h-4 w-4" /> Settings
        </button>
        <button
          type="button"
          onClick={openProjectMembers}
          className="flex items-center gap-1.5 rounded-md border border-gray-300 px-3 py-1.5 text-sm text-gray-700 hover:bg-gray-50"
        >
          <Users className="h-4 w-4" /> Members
        </button>
        <button
          type="button"
          onClick={openProjectChat}
          className="flex items-center gap-1.5 rounded-md border border-gray-300 px-3 py-1.5 text-sm text-gray-700 hover:bg-gray-50"
        >
          <MessageSquare className="h-4 w-4" /> Project Chat
        </button>
      </div>
      <KanbanBoard projectId={projectId} />
      <TaskCreateDialog projectId={projectId} />
      <ProjectSettingsModal projectId={projectId} onProjectDeleted={() => navigate('/')} />
      <ProjectMembersPanel projectId={projectId} />
      <ProjectChatPanel projectId={projectId} />
    </>
  );
}
