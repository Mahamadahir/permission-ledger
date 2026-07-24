import { get, writable } from 'svelte/store';
import type { User } from './types';

export type SessionState = { user: User | null; csrfToken: string };

export const session = writable<SessionState>({ user: null, csrfToken: '' });

/** The CSRF token for the current session, read by the api helper. */
export function csrfToken(): string {
  return get(session).csrfToken;
}

export function setSession(user: User, token: string) {
  session.set({ user, csrfToken: token });
}

export function clearSession() {
  session.set({ user: null, csrfToken: '' });
}
