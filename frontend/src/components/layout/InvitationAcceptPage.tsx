import { useNavigate, useParams } from 'react-router-dom';
import {
  useAcceptInvitation,
  useGetInvitationByToken,
} from '@/api/generated/endpoints/invitations/invitations';
import { useAuthStore } from '@/stores/authStore';

export function InvitationAcceptPage() {
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
        <p className="text-gray-500">Invalid invitation link.</p>
      </div>
    );
  }

  const handleAccept = async () => {
    try {
      await acceptMutation.mutateAsync({ token });
      navigate('/');
    } catch {
      // Error is shown via mutation state
    }
  };

  return (
    <div className="flex min-h-screen items-center justify-center bg-gray-100">
      <div className="w-full max-w-md rounded-lg bg-white p-8 shadow-md">
        <h1 className="mb-6 text-2xl font-bold text-gray-900">Project Invitation</h1>

        {isLoading ? (
          <p className="text-gray-500">Loading invitation...</p>
        ) : isError ? (
          <p className="text-red-500">Invalid or expired invitation link.</p>
        ) : info ? (
          <div className="space-y-4">
            <div className="rounded-md bg-blue-50 p-4">
              <p className="text-sm text-gray-700">
                <span className="font-medium">{info.invited_by_name}</span> invited you to join
              </p>
              <p className="mt-1 text-lg font-semibold text-gray-900">{info.project_name}</p>
              <p className="mt-1 text-sm text-gray-500">
                Role: <span className="font-medium">{info.role}</span>
              </p>
            </div>

            {info.accepted ? (
              <p className="text-sm text-green-600">This invitation has already been accepted.</p>
            ) : info.expired ? (
              <p className="text-sm text-red-600">This invitation has expired.</p>
            ) : !isAuthenticated ? (
              <div className="space-y-2">
                <p className="text-sm text-gray-600">Please log in to accept this invitation.</p>
                <button
                  type="button"
                  onClick={() => navigate(`/login?redirect=/invite/${token}`)}
                  className="w-full rounded-md bg-blue-600 px-4 py-2 text-white hover:bg-blue-700"
                >
                  Log In
                </button>
                <button
                  type="button"
                  onClick={() => navigate(`/register?redirect=/invite/${token}`)}
                  className="w-full rounded-md border border-gray-300 px-4 py-2 text-gray-700 hover:bg-gray-50"
                >
                  Create Account
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
                  {acceptMutation.isPending ? 'Accepting...' : 'Accept Invitation'}
                </button>
                {acceptMutation.isError && (
                  <p className="text-sm text-red-600">
                    Failed to accept invitation. It may have expired or already been used.
                  </p>
                )}
              </div>
            )}
          </div>
        ) : null}
      </div>
    </div>
  );
}
