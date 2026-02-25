import { useQueryClient } from '@tanstack/react-query';
import { X } from 'lucide-react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  getListInvitationsQueryKey,
  useCreateInvitation,
  useDeleteInvitation,
  useListInvitations,
} from '@/api/generated/endpoints/invitations/invitations';
import {
  getListMembersQueryKey,
  useAddMember,
  useListMembers,
  useRemoveMember,
  useUpdateMember,
} from '@/api/generated/endpoints/project-members/project-members';
import { useSearchUsers } from '@/api/generated/endpoints/users/users';
import type { MemberRole as MemberRoleType } from '@/api/generated/model';
import { MemberRole } from '@/api/generated/model';
import { useEscapeKey } from '@/hooks/useEscapeKey';
import { useAuthStore } from '@/stores/authStore';
import { useToastStore } from '@/stores/toastStore';
import { useUiStore } from '@/stores/uiStore';

export function ProjectMembersPanel({ projectId }: { projectId: string }) {
  const isOpen = useUiStore((s) => s.isProjectMembersOpen);
  if (!isOpen) return null;
  return <ProjectMembersContent projectId={projectId} />;
}

function ProjectMembersContent({ projectId }: { projectId: string }) {
  const { t } = useTranslation();
  const closeProjectMembers = useUiStore((s) => s.closeProjectMembers);
  const { data: members, isLoading, isError } = useListMembers(projectId);
  const addMember = useAddMember();
  const updateMember = useUpdateMember();
  const removeMember = useRemoveMember();
  const queryClient = useQueryClient();
  const currentUser = useAuthStore((s) => s.user);
  const addToast = useToastStore((s) => s.addToast);

  const [searchQuery, setSearchQuery] = useState('');
  const [selectedUser, setSelectedUser] = useState<{
    id: string;
    name: string;
  } | null>(null);
  const [inviteRole, setInviteRole] = useState<MemberRoleType>(MemberRole.member);
  const [removingUserId, setRemovingUserId] = useState<string | null>(null);

  const { data: searchResults } = useSearchUsers(
    { q: searchQuery, limit: 10 },
    { query: { enabled: searchQuery.length >= 2 } },
  );

  const currentUserRole = members?.find((m) => m.user_id === currentUser?.id)?.role;
  const canManage = currentUserRole === MemberRole.owner || currentUserRole === MemberRole.admin;

  const filteredResults = searchResults?.filter((u) => !members?.some((m) => m.user_id === u.id));

  const invalidateMembers = () => {
    queryClient.invalidateQueries({
      queryKey: getListMembersQueryKey(projectId),
    });
  };

  useEscapeKey(closeProjectMembers);

  const handleInvite = async () => {
    if (!selectedUser) return;
    try {
      await addMember.mutateAsync({
        projectId,
        data: { user_id: selectedUser.id, role: inviteRole },
      });
      invalidateMembers();
      setSelectedUser(null);
      setSearchQuery('');
      addToast('success', t('members.addedSuccess', { name: selectedUser.name }));
    } catch {
      addToast('error', t('members.addFailed'));
    }
  };

  const handleRoleChange = async (userId: string, role: MemberRoleType) => {
    try {
      await updateMember.mutateAsync({
        projectId,
        userId,
        data: { role },
      });
      invalidateMembers();
    } catch {
      addToast('error', t('members.roleUpdateFailed'));
    }
  };

  const handleRemove = async (userId: string) => {
    try {
      await removeMember.mutateAsync({ projectId, userId });
      invalidateMembers();
      setRemovingUserId(null);
      addToast('success', t('members.removed'));
    } catch {
      addToast('error', t('members.removeFailed'));
    }
  };

  const canManageMember = (memberRole: MemberRoleType) => {
    if (!canManage) return false;
    if (currentUserRole === MemberRole.admin && memberRole === MemberRole.owner) return false;
    return true;
  };

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="project-members-title"
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
      onClick={(e) => {
        if (e.target === e.currentTarget) closeProjectMembers();
      }}
    >
      <div className="w-full max-w-lg rounded-lg bg-white p-6 shadow-xl">
        <div className="mb-4 flex items-center justify-between">
          <h2 id="project-members-title" className="text-lg font-semibold text-gray-900">
            {t('members.members')}
          </h2>
          <button
            type="button"
            onClick={closeProjectMembers}
            className="text-gray-400 hover:text-gray-600"
            aria-label={t('common.close')}
          >
            <X className="h-5 w-5" />
          </button>
        </div>

        {isLoading ? (
          <p className="text-sm text-gray-500">{t('common.loading')}</p>
        ) : isError ? (
          <p className="text-sm text-red-500">{t('members.loadFailed')}</p>
        ) : (
          <div className="space-y-3">
            {members?.map((m) => (
              <div
                key={m.user_id}
                className="flex items-center gap-3 rounded-md border border-gray-200 px-3 py-2"
              >
                <div className="min-w-0 flex-1">
                  <p className="text-sm font-medium text-gray-900">{m.user_name}</p>
                  <p className="text-xs text-gray-500">{m.user_email}</p>
                </div>

                {m.user_id === currentUser?.id || !canManageMember(m.role) ? (
                  <span className="rounded-full bg-gray-100 px-2 py-0.5 text-xs font-medium text-gray-600">
                    {t(`members.role.${m.role}`)}
                  </span>
                ) : removingUserId === m.user_id ? (
                  <div className="flex items-center gap-1">
                    <button
                      type="button"
                      onClick={() => handleRemove(m.user_id)}
                      className="rounded bg-red-600 px-2 py-0.5 text-xs text-white hover:bg-red-700"
                    >
                      {t('common.confirm')}
                    </button>
                    <button
                      type="button"
                      onClick={() => setRemovingUserId(null)}
                      className="rounded border border-gray-300 px-2 py-0.5 text-xs text-gray-700 hover:bg-gray-50"
                    >
                      {t('common.cancel')}
                    </button>
                  </div>
                ) : (
                  <div className="flex items-center gap-2">
                    <select
                      value={m.role}
                      onChange={(e) =>
                        handleRoleChange(m.user_id, e.target.value as MemberRoleType)
                      }
                      className="rounded border border-gray-300 px-1 py-0.5 text-xs"
                    >
                      <option value="owner">{t('members.role.owner')}</option>
                      <option value="admin">{t('members.role.admin')}</option>
                      <option value="member">{t('members.role.member')}</option>
                    </select>
                    <button
                      type="button"
                      onClick={() => setRemovingUserId(m.user_id)}
                      className="text-xs text-gray-400 hover:text-red-600"
                      aria-label={t('common.remove')}
                    >
                      {t('common.remove')}
                    </button>
                  </div>
                )}
              </div>
            ))}
          </div>
        )}

        {canManage && <InvitationSection projectId={projectId} />}

        {canManage && (
          <div className="mt-4 border-t pt-4">
            <h3 className="mb-2 text-sm font-medium text-gray-700">
              {t('members.addExistingUser')}
            </h3>
            <div className="relative">
              <input
                type="text"
                value={selectedUser ? selectedUser.name : searchQuery}
                onChange={(e) => {
                  setSearchQuery(e.target.value);
                  setSelectedUser(null);
                }}
                placeholder={t('members.searchPlaceholder')}
                className="block w-full rounded border border-gray-300 px-3 py-2 text-sm focus:border-blue-500 focus:ring-1 focus:ring-blue-500"
              />
              {!selectedUser && filteredResults && filteredResults.length > 0 && (
                <div className="absolute z-10 mt-1 w-full rounded-md border border-gray-200 bg-white shadow-lg">
                  {filteredResults.map((u) => (
                    <button
                      key={u.id}
                      type="button"
                      data-testid="search-result"
                      onClick={() => {
                        setSelectedUser({ id: u.id, name: u.name });
                        setSearchQuery('');
                      }}
                      className="flex w-full items-center gap-2 px-3 py-2 text-left text-sm hover:bg-gray-50"
                    >
                      <span className="font-medium">{u.name}</span>
                      <span className="text-xs text-gray-500">{u.email}</span>
                    </button>
                  ))}
                </div>
              )}
            </div>
            <div className="mt-2 flex gap-2">
              <select
                value={inviteRole}
                onChange={(e) => setInviteRole(e.target.value as MemberRoleType)}
                className="rounded border border-gray-300 px-2 py-1.5 text-sm"
              >
                <option value="member">{t('members.role.member')}</option>
                <option value="admin">{t('members.role.admin')}</option>
              </select>
              <button
                type="button"
                onClick={handleInvite}
                disabled={!selectedUser || addMember.isPending}
                className="rounded bg-blue-600 px-3 py-1.5 text-sm text-white hover:bg-blue-700 disabled:opacity-50"
              >
                {t('common.add')}
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

function InvitationSection({ projectId }: { projectId: string }) {
  const { t } = useTranslation();
  const { data: invitations } = useListInvitations(projectId);
  const createInvitation = useCreateInvitation();
  const deleteInvitation = useDeleteInvitation();
  const queryClient = useQueryClient();
  const addToast = useToastStore((s) => s.addToast);

  const invalidateInvitations = () => {
    queryClient.invalidateQueries({
      queryKey: getListInvitationsQueryKey(projectId),
    });
  };

  const handleCreate = async () => {
    try {
      const result = await createInvitation.mutateAsync({
        projectId,
        data: {},
      });
      invalidateInvitations();
      try {
        await navigator.clipboard.writeText(result.invite_url);
        addToast('success', t('members.inviteCopied'));
      } catch {
        addToast('success', t('members.inviteCreated', { url: result.invite_url }));
      }
    } catch {
      addToast('error', t('members.inviteFailed'));
    }
  };

  const handleDelete = async (invitationId: string) => {
    try {
      await deleteInvitation.mutateAsync({ projectId, invitationId });
      invalidateInvitations();
    } catch {
      addToast('error', t('members.revokeFailed'));
    }
  };

  // Token is only available at creation time — it's copied to clipboard
  // automatically when the invitation is created. Re-retrieval is not
  // possible since only the SHA-256 hash is stored in the database.

  const pendingInvitations = invitations?.filter((i) => !i.accepted_at) ?? [];

  return (
    <div className="mt-4 border-t pt-4">
      <div className="mb-2 flex items-center justify-between">
        <h3 className="text-sm font-medium text-gray-700">{t('members.invitationLinks')}</h3>
        <button
          type="button"
          onClick={handleCreate}
          disabled={createInvitation.isPending}
          className="rounded bg-green-600 px-2 py-1 text-xs text-white hover:bg-green-700 disabled:opacity-50"
          data-testid="create-invitation-btn"
        >
          {t('members.createLink')}
        </button>
      </div>

      {pendingInvitations.length > 0 ? (
        <div className="space-y-2">
          {pendingInvitations.map((inv) => {
            const isExpired = new Date(inv.expires_at) < new Date();
            return (
              <div
                key={inv.id}
                data-testid="invitation-item"
                className="flex items-center gap-2 rounded border border-gray-200 px-2 py-1.5"
              >
                <div className="min-w-0 flex-1">
                  <p className="text-xs text-gray-600">
                    {inv.role} &middot; by {inv.invited_by_name}
                  </p>
                  <p className={`text-xs ${isExpired ? 'text-red-500' : 'text-gray-400'}`}>
                    {isExpired
                      ? t('members.expired')
                      : t('members.expires', {
                          date: new Date(inv.expires_at).toLocaleDateString(),
                        })}
                  </p>
                </div>
                <button
                  type="button"
                  onClick={() => handleDelete(inv.id)}
                  className="text-xs text-gray-400 hover:text-red-600"
                  aria-label={t('members.revoke')}
                >
                  {t('members.revoke')}
                </button>
              </div>
            );
          })}
        </div>
      ) : (
        <p className="text-xs text-gray-500">{t('members.noPendingInvitations')}</p>
      )}
    </div>
  );
}
