import { vi, type Mock } from 'vitest';
import { clearSession, setSession } from './session';

export type MockResponse = { status?: number; body?: unknown };
export type Responder = MockResponse | (() => MockResponse);
export type Routes = Record<string, Responder>;

export const fixtures = {
  user: { id: 'u1', email: 'sam@example.com', display_name: null },
  category: { id: 'c1', slug: 'cookies', name: 'Cookies' },
  record: {
    id: 'r1',
    service_name: 'Acme',
    website_url: 'https://acme.example',
    category_id: 'c1',
    category_name: 'Cookies',
    consent_type: 'analytics cookies',
    date_given: '2026-01-01',
    review_date: '2026-06-01',
    expiry_date: null,
    status: 'active',
    risk_level: 'high',
    source: 'manual',
    notes: null
  },
  device: {
    id: 'd1',
    name: 'Chrome extension',
    last_used_at: null,
    revoked_at: null,
    created_at: '2026-01-01T00:00:00Z'
  },
  dashboard: {
    summary: { active: 3, review_due: 1, expired: 0, revoked: 2, high_risk: 1 },
    recent: [],
    categories: [],
    services: []
  }
};

const defaults: Routes = {
  'GET /api/auth/me': { status: 401 },
  'POST /api/auth/logout': { status: 204 },
  'GET /api/categories': { body: [fixtures.category] },
  'GET /api/records': { body: [] },
  'GET /api/dashboard': { body: fixtures.dashboard },
  'GET /api/extension/devices': { body: [] }
};

// Requests are relative now that the app and API share an origin, so a base
// is needed to parse them.
function pathOf(input: string): string {
  return new URL(input, 'http://localhost').pathname;
}

/**
 * Install a routing fetch mock. Routes are keyed "METHOD /path" (query string
 * ignored for matching); unlisted routes fall back to sensible defaults.
 */
export function mockApi(routes: Routes = {}): Mock {
  const table = { ...defaults, ...routes };
  const fetchMock = vi.fn(async (input: string, options: RequestInit = {}) => {
    const method = (options.method ?? 'GET').toUpperCase();
    const path = pathOf(input);
    const responder = table[`${method} ${path}`];
    const { status = 200, body = null } = typeof responder === 'function' ? responder() : (responder ?? {});
    return {
      ok: status < 400,
      status,
      statusText: `status ${status}`,
      json: async () => body
    } as Response;
  });
  vi.stubGlobal('fetch', fetchMock);
  return fetchMock;
}

/** The RequestInit body of the most recent call to a "METHOD /path" route. */
export function bodyOf(fetchMock: Mock, key: string): Record<string, unknown> | undefined {
  const [method, path] = key.split(' ');
  const call = [...fetchMock.mock.calls]
    .reverse()
    .find(([input, options = {}]) => (options.method ?? 'GET').toUpperCase() === method && pathOf(input) === path);
  if (!call) return undefined;
  const raw = (call[1] ?? {}).body;
  return typeof raw === 'string' ? JSON.parse(raw) : undefined;
}

/** Seed an authenticated session so page components render signed in. */
export function signIn() {
  setSession(fixtures.user, 'csrf1');
}

export function signOut() {
  clearSession();
}

/** Every request made, as "METHOD /path" strings, in call order. */
export function requestLog(fetchMock: Mock): string[] {
  return fetchMock.mock.calls.map(
    ([input, options = {}]) => `${(options.method ?? 'GET').toUpperCase()} ${pathOf(input)}`
  );
}

/** The most recent request URL (with query string) matching a path. */
export function lastUrl(fetchMock: Mock, path: string): string | undefined {
  const call = [...fetchMock.mock.calls].reverse().find(([input]) => pathOf(input) === path);
  return call?.[0];
}
