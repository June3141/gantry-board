import { useQueryClient } from '@tanstack/react-query';
import { X } from 'lucide-react';
import { useCallback, useState } from 'react';
import { useListMembers } from '@/api/generated/endpoints/project-members/project-members';
import {
  getGetProjectQueryKey,
  getListProjectsQueryKey,
  useDeleteProject,
  useGetProject,
  useUpdateProject,
} from '@/api/generated/endpoints/projects/projects';
import { MemberRole } from '@/api/generated/model';
import { useEscapeKey } from '@/hooks/useEscapeKey';
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
  const closeProjectSettings = useUiStore((s) => s.closeProjectSettings);
  const { data: project, isLoading, isError } = useGetProject(projectId);
  const { data: members } = useListMembers(projectId);
  const updateProject = useUpdateProject();
  const deleteProject = useDeleteProject();
  const queryClient = useQueryClient();
  const currentUser = useAuthStore((s) => s.user);
  const addToast = useToastStore((s) => s.addToast);

  const [editingField, setEditingField] = useState<'name' | 'description' | null>(null);
  const [editValue, setEditValue] = useState('');
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);

  const currentUserRole = members?.find((m) => m.user_id === currentUser?.id)?.role;
  const canEdit = currentUserRole === MemberRole.owner || currentUserRole === MemberRole.admin;
  const isOwner = currentUserRole === MemberRole.owner;

  const escapeGuard = useCallback(() => {
    if (editingField) {
      setEditingField(null);
      return true;
    }
    return false;
  }, [editingField]);
  useEscapeKey(closeProjectSettings, escapeGuard);

  const startEditing = (field: 'name' | 'description', value: string) => {
    setEditingField(field);
    setEditValue(value);
  };

  const saveField = async (field: 'name' | 'description') => {
    const trimmed = editValue.trim();
    if (field === 'name' && !trimmed) {
      setEditingField(null);
      return;
    }
    const currentValue = field === 'name' ? project?.name : project?.description;
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
        addToast('success', `Project ${field} updated.`);
      }
    } catch {
      addToast('error', `Failed to update ${field}.`);
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
      addToast('success', 'Project deleted.');
    } catch {
      addToast('error', 'Failed to delete project.');
    }
  };

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="project-settings-title"
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
      onClick={(e) => {
        if (e.target === e.currentTarget) closeProjectSettings();
      }}
    >
      <div className="w-full max-w-md rounded-lg bg-white p-6 shadow-xl">
        <div className="mb-4 flex items-center justify-between">
          <h2 id="project-settings-title" className="text-lg font-semibold text-gray-900">
            Project Settings
          </h2>
          <button
            type="button"
            onClick={closeProjectSettings}
            className="text-gray-400 hover:text-gray-600"
            aria-label="Close"
          >
            <X className="h-5 w-5" />
          </button>
        </div>

        {isLoading ? (
          <p className="text-sm text-gray-500">Loading...</p>
        ) : isError || !project ? (
          <p className="text-sm text-red-500">Failed to load project.</p>
        ) : (
          <div className="space-y-4">
            <div>
              <h3 className="text-sm font-medium text-gray-700">Name</h3>
              {editingField === 'name' ? (
                <input
                  type="text"
                  value={editValue}
                  onChange={(e) => setEditValue(e.target.value)}
                  onBlur={() => saveField('name')}
                  className="mt-1 block w-full rounded border border-blue-300 px-2 py-1 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
                  autoFocus
                />
              ) : canEdit ? (
                <button
                  type="button"
                  className="mt-1 cursor-pointer rounded px-1 text-left text-sm text-gray-900 hover:bg-gray-100"
                  onClick={() => startEditing('name', project.name)}
                >
                  {project.name}
                </button>
              ) : (
                <p className="mt-1 px-1 text-sm text-gray-900">{project.name}</p>
              )}
            </div>

            <div>
              <h3 className="text-sm font-medium text-gray-700">Description</h3>
              {editingField === 'description' ? (
                <textarea
                  value={editValue}
                  onChange={(e) => setEditValue(e.target.value)}
                  onBlur={() => saveField('description')}
                  rows={3}
                  className="mt-1 block w-full rounded border border-blue-300 px-2 py-1 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
                  autoFocus
                />
              ) : canEdit ? (
                <button
                  type="button"
                  className="mt-1 cursor-pointer rounded px-1 text-left text-sm text-gray-600 hover:bg-gray-100"
                  onClick={() => startEditing('description', project.description ?? '')}
                >
                  {project.description || 'No description'}
                </button>
              ) : (
                <p className="mt-1 px-1 text-sm text-gray-600">
                  {project.description || 'No description'}
                </p>
              )}
            </div>

            {canEdit && (
              <div className="border-t pt-4">
                <h3 className="text-sm font-medium text-gray-700 mb-2">GitHub</h3>
                <GitHubLinkSettings projectId={projectId} />
              </div>
            )}

            {isOwner && (
              <div className="border-t pt-4">
                {showDeleteConfirm ? (
                  <div className="flex items-center justify-between rounded-md bg-red-50 p-3">
                    <p className="text-sm text-red-700">Are you sure? This cannot be undone.</p>
                    <div className="flex gap-2">
                      <button
                        type="button"
                        onClick={() => setShowDeleteConfirm(false)}
                        className="rounded-md border border-gray-300 px-3 py-1 text-sm text-gray-700 hover:bg-gray-50"
                      >
                        Cancel
                      </button>
                      <button
                        type="button"
                        onClick={handleDelete}
                        className="rounded-md bg-red-600 px-3 py-1 text-sm text-white hover:bg-red-700"
                      >
                        Confirm
                      </button>
                    </div>
                  </div>
                ) : (
                  <button
                    type="button"
                    onClick={() => setShowDeleteConfirm(true)}
                    className="rounded-md border border-red-300 px-4 py-2 text-sm text-red-700 hover:bg-red-50"
                  >
                    Delete Project
                  </button>
                )}
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
