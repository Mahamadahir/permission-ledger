import { fireEvent, render, screen } from '@testing-library/svelte';
import { get } from 'svelte/store';
import { afterEach, describe, expect, it, vi } from 'vitest';
import AppShell from './AppShell.svelte';
import { mockApi, signIn, signOut } from '$lib/test-helpers';
import { session } from '$lib/session';

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  signOut();
});

describe('AppShell', () => {
  it('links to every destination in the sidebar', () => {
    mockApi();
    signIn();
    render(AppShell, { pathname: '/' });

    for (const [name, href] of [
      ['Dashboard', '/'],
      ['Records', '/records'],
      ['Extension', '/extension'],
      ['Exports', '/exports'],
      ['Settings', '/settings']
    ]) {
      expect(screen.getByRole('link', { name })).toHaveAttribute('href', href);
    }
  });

  it('marks the current route as active', () => {
    mockApi();
    signIn();
    render(AppShell, { pathname: '/records' });
    expect(screen.getByRole('link', { name: 'Records' })).toHaveClass('active');
    expect(screen.getByRole('link', { name: 'Dashboard' })).not.toHaveClass('active');
  });

  it('shows the signed-in user', () => {
    mockApi();
    signIn();
    render(AppShell, { pathname: '/' });
    expect(screen.getAllByText('sam@example.com').length).toBeGreaterThan(0);
  });

  it('clears the session on logout', async () => {
    mockApi({ 'POST /api/auth/logout': { status: 204 } });
    signIn();
    render(AppShell, { pathname: '/' });

    await fireEvent.click(screen.getByRole('button', { name: 'Log out' }));
    expect(get(session).user).toBeNull();
  });
});
