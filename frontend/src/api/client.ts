const BASE_URL = import.meta.env.VITE_API_BASE_URL ?? 'http://localhost:3000';

export const customInstance = async <T>(
  url: string,
  options?: RequestInit,
): Promise<T> => {
  const fullUrl = `${BASE_URL}${url}`;

  const response = await fetch(fullUrl, {
    credentials: 'include', // Include cookies for session auth
    ...options,
    headers: {
      'Content-Type': 'application/json',
      'X-Requested-With': 'XMLHttpRequest',
      ...options?.headers,
    },
  });

  if (!response.ok) {
    const error = await response.json().catch(() => ({ error: response.statusText }));

    // Handle 401 Unauthorized - redirect to login with loop prevention
    if (response.status === 401) {
      const isAuthPage =
        window.location.pathname.startsWith('/login') ||
        window.location.pathname.startsWith('/register');

      if (!isAuthPage) {
        const REDIRECT_KEY = 'auth_redirect_ts';
        const COOLDOWN_MS = 5000;
        const now = Date.now();
        const lastRedirect = Number(sessionStorage.getItem(REDIRECT_KEY) ?? '0');

        if (now - lastRedirect > COOLDOWN_MS) {
          sessionStorage.setItem(REDIRECT_KEY, String(now));
          window.location.href = '/login';
        }
      }
    }

    throw new Error(error.error ?? 'Request failed');
  }

  if (response.status === 204) {
    return undefined as T;
  }

  const contentType = response.headers.get('content-type') ?? '';
  if (contentType.includes('application/json')) {
    return response.json();
  }

  return response.text() as Promise<T>;
};
