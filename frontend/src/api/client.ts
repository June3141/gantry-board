const BASE_URL = import.meta.env.VITE_API_BASE_URL ?? 'http://localhost:3000';

export const customInstance = async <T>({
  url,
  method,
  params,
  data,
  headers,
  signal,
}: {
  url: string;
  method: 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE';
  params?: Record<string, string | number | boolean | undefined>;
  data?: unknown;
  headers?: Record<string, string>;
  signal?: AbortSignal;
}): Promise<T> => {
  const searchParams = new URLSearchParams();
  if (params) {
    for (const [key, value] of Object.entries(params)) {
      if (value !== undefined) {
        searchParams.set(key, String(value));
      }
    }
  }
  const fullUrl = `${BASE_URL}${url}${searchParams.toString() ? `?${searchParams}` : ''}`;

  const response = await fetch(fullUrl, {
    method,
    credentials: 'include', // Include cookies for session auth
    headers: {
      'Content-Type': 'application/json',
      ...headers,
    },
    ...(data ? { body: JSON.stringify(data) } : {}),
    signal,
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
