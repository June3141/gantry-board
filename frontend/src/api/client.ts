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
  params?: Record<string, string>;
  data?: unknown;
  headers?: Record<string, string>;
  signal?: AbortSignal;
}): Promise<T> => {
  const searchParams = new URLSearchParams(params);
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

    // Handle 401 Unauthorized - redirect to login
    if (response.status === 401) {
      // Only redirect if not already on auth pages
      if (
        !window.location.pathname.startsWith('/login') &&
        !window.location.pathname.startsWith('/register')
      ) {
        window.location.href = '/login';
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
