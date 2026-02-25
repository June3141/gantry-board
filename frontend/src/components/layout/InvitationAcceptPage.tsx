import { useTranslation } from 'react-i18next';
import { useNavigate, useParams } from 'react-router-dom';
import {
  useAcceptInvitation,
  useGetInvitationByToken,
} from '@/api/generated/endpoints/invitations/invitations';
import { useAuthStore } from '@/stores/authStore';

export function InvitationAcceptPage() {
  const { t } = useTranslation();
  const { token } = useParams<{ token: string }>();
  const navigate = useNavigate();
  const isAuthenticated = useAuthStore((s) => s.isAuthenticated);
  const {
    data: info,
    isLoading,
    isError,
  } = useGetInvitationByToken(token ?? '', {
    query: { enabled: !!token },
  });
  const acceptMutation = useAcceptInvitation();

  if (!token) {
    return (
      <div className="flex min-h-screen items-center justify-center bg-gray-100">
        <p className="text-gray-500">{t('invitation.invalidLink')}</p>
      </div>
    );
  }

  const handleAccept = async () => {
    try {
      const result = await acceptMutation.mutateAsync({ token });
      navigate(`/projects/${result.project_id}`);
    } catch {
      // Error is shown via mutation state
    }
  };

  return (
    <div className="flex min-h-screen items-center justify-center bg-gray-100">
      <div className="w-full max-w-md rounded-lg bg-white p-8 shadow-md">
        <h1 className="mb-6 text-2xl font-bold text-gray-900">{t('invitation.title')}</h1>

        {isLoading ? (
          <p className="text-gray-500">{t('invitation.loadingInvitation')}</p>
        ) : isError ? (
          <p className="text-red-500">{t('invitation.expiredOrInvalid')}</p>
        ) : info ? (
          <div className="space-y-4">
            <div className="rounded-md bg-blue-50 p-4">
              <p className="text-sm text-gray-700">
                <span className="font-medium">{info.invited_by_name}</span>{' '}
                {t('invitation.invitedYou')}
              </p>
              <p className="mt-1 text-lg font-semibold text-gray-900">{info.project_name}</p>
              <p className="mt-1 text-sm text-gray-500">
                {t('invitation.role')} <span className="font-medium">{info.role}</span>
              </p>
            </div>

            {info.accepted ? (
              <p className="text-sm text-green-600">{t('invitation.alreadyAccepted')}</p>
            ) : info.expired ? (
              <p className="text-sm text-red-600">{t('invitation.expired')}</p>
            ) : !isAuthenticated ? (
              <div className="space-y-2">
                <p className="text-sm text-gray-600">{t('invitation.loginRequired')}</p>
                <button
                  type="button"
                  onClick={() => navigate(`/login?redirect=/invite/${token}`)}
                  className="w-full rounded-md bg-blue-600 px-4 py-2 text-white hover:bg-blue-700"
                >
                  {t('invitation.logIn')}
                </button>
                <button
                  type="button"
                  onClick={() => navigate(`/register?redirect=/invite/${token}`)}
                  className="w-full rounded-md border border-gray-300 px-4 py-2 text-gray-700 hover:bg-gray-50"
                >
                  {t('auth.createAccountBtn')}
                </button>
              </div>
            ) : (
              <div className="space-y-2">
                <button
                  type="button"
                  onClick={handleAccept}
                  disabled={acceptMutation.isPending}
                  className="w-full rounded-md bg-blue-600 px-4 py-2 text-white hover:bg-blue-700 disabled:opacity-50"
                >
                  {acceptMutation.isPending ? t('invitation.accepting') : t('invitation.accept')}
                </button>
                {acceptMutation.isError && (
                  <p className="text-sm text-red-600">{t('invitation.acceptFailed')}</p>
                )}
              </div>
            )}
          </div>
        ) : null}
      </div>
    </div>
  );
}
