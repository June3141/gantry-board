import { MessageSquare, Settings, Users } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useNavigate, useParams } from 'react-router';
import { ProjectChatPanel, ProjectMembersPanel, ProjectSettingsModal } from '@/components/project';
import { TaskCreateDialog } from '@/components/task';
import { Button } from '@/components/ui/button';
import { useUiStore } from '@/stores/uiStore';
import { KanbanBoard } from './KanbanBoard';

export function ProjectBoardPage() {
  const { t } = useTranslation();
  const { projectId } = useParams<{ projectId: string }>();
  const navigate = useNavigate();
  const openProjectSettings = useUiStore((s) => s.openProjectSettings);
  const openProjectMembers = useUiStore((s) => s.openProjectMembers);
  const openProjectChat = useUiStore((s) => s.openProjectChat);

  if (!projectId) return null;

  return (
    <>
      <div className="mx-auto flex max-w-7xl items-center gap-3 px-4 pt-4">
        <Button variant="outline" size="sm" onClick={openProjectSettings}>
          <Settings className="h-4 w-4" /> {t('project.settings')}
        </Button>
        <Button variant="outline" size="sm" onClick={openProjectMembers}>
          <Users className="h-4 w-4" /> {t('members.members')}
        </Button>
        <Button variant="outline" size="sm" onClick={openProjectChat}>
          <MessageSquare className="h-4 w-4" /> {t('chat.projectChat')}
        </Button>
      </div>
      <KanbanBoard projectId={projectId} />
      <TaskCreateDialog projectId={projectId} />
      <ProjectSettingsModal projectId={projectId} onProjectDeleted={() => navigate('/')} />
      <ProjectMembersPanel projectId={projectId} />
      <ProjectChatPanel projectId={projectId} />
    </>
  );
}
