import { fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import Page from './+page.svelte';
import { bodyOf, mockApi, signIn, signOut } from '$lib/test-helpers';

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  signOut();
});

const settings = { display_name: 'Sam', timezone: 'Europe/London', review_reminder_days: 30 };

describe('settings page', () => {
  it('loads the current settings into the form', async () => {
    mockApi({ 'GET /api/auth/settings': { body: settings } });
    signIn();
    render(Page);

    expect(((await screen.findByLabelText('Display name')) as HTMLInputElement).value).toBe('Sam');
    expect((screen.getByLabelText('Timezone') as HTMLInputElement).value).toBe('Europe/London');
    expect((screen.getByLabelText(/Review reminder days/) as HTMLInputElement).value).toBe('30');
  });

  it('saves changes and confirms', async () => {
    const fetchMock = mockApi({
      'GET /api/auth/settings': { body: settings },
      'PUT /api/auth/settings': { body: { ...settings, display_name: 'Sammy' } }
    });
    signIn();
    render(Page);

    await fireEvent.input(await screen.findByLabelText('Display name'), { target: { value: 'Sammy' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Save settings' }));

    expect(await screen.findByText('Settings saved')).toBeInTheDocument();
    expect(bodyOf(fetchMock, 'PUT /api/auth/settings')).toMatchObject({ display_name: 'Sammy' });
  });

  it('constrains the reminder window to the range the backend accepts', async () => {
    mockApi({ 'GET /api/auth/settings': { body: settings } });
    signIn();
    render(Page);

    // The backend clamps to 1-365, so the input refuses out-of-range values
    // rather than letting the user submit something that silently changes.
    const days = (await screen.findByLabelText(/Review reminder days/)) as HTMLInputElement;
    expect(days).toHaveAttribute('min', '1');
    expect(days).toHaveAttribute('max', '365');
  });

  it('adopts the reminder window the server stored', async () => {
    mockApi({
      'GET /api/auth/settings': { body: settings },
      // The server is the authority: whatever it returns wins.
      'PUT /api/auth/settings': { body: { ...settings, review_reminder_days: 365 } }
    });
    signIn();
    render(Page);

    const days = (await screen.findByLabelText(/Review reminder days/)) as HTMLInputElement;
    await fireEvent.input(days, { target: { value: '120' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Save settings' }));

    await screen.findByText('Settings saved');
    expect((screen.getByLabelText(/Review reminder days/) as HTMLInputElement).value).toBe('365');
  });

  it('surfaces a load failure', async () => {
    mockApi({ 'GET /api/auth/settings': { status: 500, body: { error: 'internal server error' } } });
    signIn();
    render(Page);
    expect(await screen.findByText('internal server error')).toBeInTheDocument();
  });
});
