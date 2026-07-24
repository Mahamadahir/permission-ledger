import { render, screen, within } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import Page from './+page.svelte';
import { fixtures, mockApi, signIn, signOut } from '$lib/test-helpers';

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  signOut();
});

describe('dashboard page', () => {
  it('shows the summary metrics', async () => {
    mockApi();
    signIn();
    render(Page);

    const metrics = await screen.findByRole('region', { name: 'Summary metrics' });
    const tile = (label: string) => within(metrics).getByText(label).closest('article')!;
    // The tiles render with zeroes until the dashboard request resolves.
    await within(tile('Active permissions')).findByText('3');
    expect(within(tile('Revoked')).getByText('2')).toBeInTheDocument();
    expect(within(tile('High risk')).getByText('1')).toBeInTheDocument();
  });

  it('lists recently added records', async () => {
    mockApi({
      'GET /api/dashboard': {
        body: { ...fixtures.dashboard, recent: [fixtures.record] }
      }
    });
    signIn();
    render(Page);

    expect(await screen.findByText('Acme')).toBeInTheDocument();
    expect(screen.getByText('analytics cookies')).toBeInTheDocument();
  });

  it('shows an empty message when nothing has been recorded', async () => {
    mockApi();
    signIn();
    render(Page);
    expect(await screen.findByText(/No records yet/)).toBeInTheDocument();
  });
});
