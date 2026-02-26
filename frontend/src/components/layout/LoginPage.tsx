import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Link, useNavigate, useSearchParams } from 'react-router-dom';
import { useLogin } from '@/api/generated/endpoints/auth/auth';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { useAuthStore } from '@/stores/authStore';

export function LoginPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const redirectTo = searchParams.get('redirect');
  const setUser = useAuthStore((state) => state.setUser);
  const login = useLogin();

  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);

    try {
      const response = await login.mutateAsync({
        data: { email, password },
      });
      setUser(response.user);
      navigate(redirectTo ?? '/', { replace: true });
    } catch {
      setError(t('auth.invalidCredentials'));
    }
  };

  return (
    <div className="flex min-h-screen items-center justify-center bg-gray-100">
      <div className="w-full max-w-md rounded-lg bg-white p-8 shadow-md">
        <h1 className="mb-6 text-center text-2xl font-bold text-gray-900">{t('auth.signInTo')}</h1>

        {error && (
          <div
            data-testid="login-error"
            className="mb-4 rounded-md bg-red-50 p-3 text-sm text-red-700"
          >
            {error}
          </div>
        )}

        <form onSubmit={handleSubmit} data-testid="login-form" className="space-y-4">
          <div>
            <label htmlFor="email" className="block text-sm font-medium text-gray-700">
              {t('auth.email')}
            </label>
            <Input
              id="email"
              type="email"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              required
              className="mt-1"
              placeholder={t('auth.emailPlaceholder')}
            />
          </div>

          <div>
            <label htmlFor="password" className="block text-sm font-medium text-gray-700">
              {t('auth.password')}
            </label>
            <Input
              id="password"
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              required
              className="mt-1"
              placeholder={t('auth.passwordPlaceholder')}
            />
          </div>

          <Button type="submit" disabled={login.isPending} className="w-full">
            {login.isPending ? t('auth.signingIn') : t('auth.signIn')}
          </Button>
        </form>

        <p className="mt-4 text-center text-sm text-gray-600">
          {t('auth.noAccount')}{' '}
          <Link
            to={redirectTo ? `/register?redirect=${encodeURIComponent(redirectTo)}` : '/register'}
            className="text-primary hover:text-primary/80"
          >
            {t('auth.signUp')}
          </Link>
        </p>
      </div>
    </div>
  );
}
