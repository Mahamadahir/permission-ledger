import { clearSession, csrfToken, setSession } from './session';

export const apiBase = import.meta.env.VITE_API_BASE ?? 'http://localhost:3000';

const MUTATING = ['POST', 'PUT', 'DELETE'];

export async function api(path: string, options: RequestInit = {}) {
  const headers = new Headers(options.headers);
  if (!headers.has('content-type') && options.body) headers.set('content-type', 'application/json');
  const token = csrfToken();
  if (token && MUTATING.includes(options.method ?? 'GET')) headers.set('x-csrf-token', token);

  const response = await fetch(`${apiBase}${path}`, { ...options, headers, credentials: 'include' });
  if (!response.ok) {
    const body = await response.json().catch(() => ({ error: response.statusText }));
    throw new Error(body.error ?? response.statusText);
  }
  if (response.status === 204) return null;
  return response.json();
}

/** Restore the session from the cookie. Returns false when not signed in. */
export async function loadSession(): Promise<boolean> {
  try {
    const data = await api('/api/auth/me');
    setSession(data.user, data.csrf_token);
    return true;
  } catch {
    clearSession();
    return false;
  }
}

export async function authenticate(
  mode: 'login' | 'register',
  credentials: { email: string; password: string; displayName?: string }
) {
  const path = mode === 'login' ? '/api/auth/login' : '/api/auth/register';
  const data = await api(path, {
    method: 'POST',
    body: JSON.stringify({
      email: credentials.email,
      password: credentials.password,
      display_name: credentials.displayName || null
    })
  });
  setSession(data.user, data.csrf_token);
}

export async function logout() {
  await api('/api/auth/logout', { method: 'POST' });
  clearSession();
}
