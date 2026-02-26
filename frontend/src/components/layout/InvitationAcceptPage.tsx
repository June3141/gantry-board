import { useTranslation } from 'react-i18next';
import { useNavigate, useParams } from 'react-router-dom';
import {
  useAcceptInvitation,
  useGetInvitationByToken,
} from '@/api/generated/endpoints/invitations/invitations';
import { Button } from '@/components/ui/button';
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
      <div className="flex min-h-screen items-center justify-center bg-muted">
        <p className="text-muted-foreground">{t('invitation.invalidLink')}</p>
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
    <div className="flex min-h-screen items-center justify-center bg-muted">
      <div className="w-full max-w-md rounded-lg bg-background p-8 shadow-md">
        <h1 className="mb-6 text-2xl font-bold text-foreground">{t('invitation.title')}</h1>

        {isLoading ? (
          <p className="text-muted-foreground">{t('invitation.loadingInvitation')}</p>
        ) : isError ? (
          <p className="text-destructive">{t('invitation.expiredOrInvalid')}</p>
        ) : info ? (
          <div className="space-y-4">
            <div className="rounded-md bg-primary/10 p-4">
              <p className="text-sm text-foreground">
                <span className="font-medium">{info.invited_by_name}</span>{' '}
                {t('invitation.invitedYou')}
              </p>
              <p className="mt-1 text-lg font-semibold text-foreground">{info.project_name}</p>
              <p className="mt-1 text-sm text-muted-foreground">
                {t('invitation.role')} <span className="font-medium">{info.role}</span>
              </p>
            </div>

            {info.accepted ? (
              <p className="text-sm text-success">{t('invitation.alreadyAccepted')}</p>
            ) : info.expired ? (
              <p className="text-sm text-destructive">{t('invitation.expired')}</p>
            ) : !isAuthenticated ? (
              <div className="space-y-2">
                <p className="text-sm text-muted-foreground">{t('invitation.loginRequired')}</p>
                <Button
                  className="w-full"
                  onClick={() => navigate(`/login?redirect=/invite/${token}`)}
                >
                  {t('invitation.logIn')}
                </Button>
                <Button
                  variant="outline"
                  className="w-full"
                  onClick={() => navigate(`/register?redirect=/invite/${token}`)}
                >
                  {t('auth.createAccountBtn')}
                </Button>
              </div>
            ) : (
              <div className="space-y-2">
                <Button
                  className="w-full"
                  onClick={handleAccept}
                  disabled={acceptMutation.isPending}
                >
                  {acceptMutation.isPending ? t('invitation.accepting') : t('invitation.accept')}
                </Button>
                {acceptMutation.isError && (
                  <p className="text-sm text-destructive">{t('invitation.acceptFailed')}</p>
                )}
              </div>
            )}
          </div>
        ) : null}
      </div>
    </div>
  );
}
