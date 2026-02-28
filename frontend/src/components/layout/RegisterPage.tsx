import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Link, useNavigate, useSearchParams } from 'react-router';
import { useRegister } from '@/api/generated/endpoints/auth/auth';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { useAuthStore } from '@/stores/authStore';

export function RegisterPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const redirectTo = searchParams.get('redirect');
  const setUser = useAuthStore((state) => state.setUser);
  const register = useRegister();

  const [name, setName] = useState('');
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);

    if (password.length < 8) {
      setError(t('auth.passwordTooShort'));
      return;
    }

    try {
      const response = await register.mutateAsync({
        data: { name, email, password },
      });
      setUser(response.user);
      navigate(redirectTo ?? '/', { replace: true });
    } catch {
      setError(t('auth.registrationFailed'));
    }
  };

  return (
    <div className="flex min-h-screen items-center justify-center bg-muted">
      <div className="w-full max-w-md rounded-lg bg-background p-8 shadow-md">
        <h1 className="mb-6 text-center text-2xl font-bold text-foreground">
          {t('auth.createAccount')}
        </h1>

        {error && (
          <div
            data-testid="register-error"
            className="mb-4 rounded-md bg-destructive/10 p-3 text-sm text-destructive"
          >
            {error}
          </div>
        )}

        <form onSubmit={handleSubmit} data-testid="register-form" className="space-y-4">
          <div>
            <label htmlFor="name" className="block text-sm font-medium text-foreground">
              {t('auth.name')}
            </label>
            <Input
              id="name"
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              required
              className="mt-1"
              placeholder={t('auth.namePlaceholder')}
            />
          </div>

          <div>
            <label htmlFor="email" className="block text-sm font-medium text-foreground">
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
            <label htmlFor="password" className="block text-sm font-medium text-foreground">
              {t('auth.password')}
            </label>
            <Input
              id="password"
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              required
              minLength={8}
              className="mt-1"
              placeholder={t('auth.passwordMinLength')}
            />
          </div>

          <Button type="submit" disabled={register.isPending} className="w-full">
            {register.isPending ? t('auth.creatingAccount') : t('auth.createAccountBtn')}
          </Button>
        </form>

        <p className="mt-4 text-center text-sm text-muted-foreground">
          {t('auth.hasAccount')}{' '}
          <Link
            to={redirectTo ? `/login?redirect=${encodeURIComponent(redirectTo)}` : '/login'}
            className="text-primary hover:text-primary/80"
          >
            {t('auth.signIn')}
          </Link>
        </p>
      </div>
    </div>
  );
}
