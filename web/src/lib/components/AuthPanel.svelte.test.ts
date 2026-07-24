import { fireEvent, render, screen } from '@testing-library/svelte';
import { get } from 'svelte/store';
import { afterEach, describe, expect, it, vi } from 'vitest';
import AuthPanel from './AuthPanel.svelte';
import { bodyOf, fixtures, mockApi, signOut } from '$lib/test-helpers';
import { session } from '$lib/session';

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  signOut();
});

/** The submit button label changes with the tab, so target it by role + class. */
function submit() {
  return screen.getAllByRole('button').find((b) => b.classList.contains('primary'))!;
}

describe('AuthPanel', () => {
  it('renders the login form by default', () => {
    mockApi();
    render(AuthPanel);
    expect(screen.getByRole('heading', { name: /Privacy decisions/i })).toBeInTheDocument();
    expect(screen.queryByLabelText('Display name')).not.toBeInTheDocument();
  });

  it('reveals the display name field on the register tab', async () => {
    mockApi();
    render(AuthPanel);
    await fireEvent.click(screen.getByRole('button', { name: 'Register' }));
    expect(screen.getByLabelText('Display name')).toBeInTheDocument();
  });

  it('signs in and stores the session on success', async () => {
    const fetchMock = mockApi({
      'POST /api/auth/login': { body: { user: fixtures.user, csrf_token: 'csrf1' } }
    });
    render(AuthPanel);
    await fireEvent.input(screen.getByLabelText('Email'), { target: { value: 'sam@example.com' } });
    await fireEvent.input(screen.getByLabelText('Password'), { target: { value: 'correcthorse1' } });
    await fireEvent.click(submit());

    expect(bodyOf(fetchMock, 'POST /api/auth/login')).toMatchObject({
      email: 'sam@example.com',
      password: 'correcthorse1'
    });
    expect(get(session).user).toEqual(fixtures.user);
    expect(get(session).csrfToken).toBe('csrf1');
  });

  it('registers through the register endpoint', async () => {
    const fetchMock = mockApi({
      'POST /api/auth/register': { body: { user: fixtures.user, csrf_token: 'csrf1' } }
    });
    render(AuthPanel);
    await fireEvent.click(screen.getByRole('button', { name: 'Register' }));
    await fireEvent.input(screen.getByLabelText('Display name'), { target: { value: 'Sam' } });
    await fireEvent.input(screen.getByLabelText('Email'), { target: { value: 'sam@example.com' } });
    await fireEvent.input(screen.getByLabelText('Password'), { target: { value: 'correcthorse1' } });
    await fireEvent.click(submit());

    expect(bodyOf(fetchMock, 'POST /api/auth/register')).toMatchObject({ display_name: 'Sam' });
  });

  it('surfaces a failed login error', async () => {
    mockApi({ 'POST /api/auth/login': { status: 401, body: { error: 'authentication required' } } });
    render(AuthPanel);
    await fireEvent.input(screen.getByLabelText('Email'), { target: { value: 'sam@example.com' } });
    await fireEvent.input(screen.getByLabelText('Password'), { target: { value: 'wrong' } });
    await fireEvent.click(submit());

    expect(await screen.findByText('authentication required')).toBeInTheDocument();
    expect(get(session).user).toBeNull();
  });
});
