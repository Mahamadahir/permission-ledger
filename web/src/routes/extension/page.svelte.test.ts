import { fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import Page from './+page.svelte';
import { fixtures, mockApi, requestLog, signIn, signOut } from '$lib/test-helpers';

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  signOut();
});

describe('extension page', () => {
  it('pairs a device and shows the returned token once', async () => {
    mockApi({
      'POST /api/extension/pair': {
        body: { id: 'd9', name: 'Chrome extension', token: 'tok-abc', created_at: 'x' }
      }
    });
    signIn();
    render(Page);

    await fireEvent.click(await screen.findByRole('button', { name: 'Pair extension' }));
    expect(await screen.findByText('tok-abc')).toBeInTheDocument();
  });

  it('lists paired devices', async () => {
    mockApi({ 'GET /api/extension/devices': { body: [fixtures.device] } });
    signIn();
    render(Page);

    expect(await screen.findByText('Chrome extension')).toBeInTheDocument();
    expect(screen.getByText('1 device')).toBeInTheDocument();
  });

  it('revokes a device', async () => {
    const fetchMock = mockApi({
      'GET /api/extension/devices': { body: [fixtures.device] },
      'DELETE /api/extension/devices/d1': { body: { revoked: true } }
    });
    signIn();
    render(Page);

    await fireEvent.click(await screen.findByRole('button', { name: 'Revoke' }));
    expect(requestLog(fetchMock)).toContain('DELETE /api/extension/devices/d1');
  });

  it('shows an empty message with no devices', async () => {
    mockApi();
    signIn();
    render(Page);
    expect(await screen.findByText('No paired devices.')).toBeInTheDocument();
  });
});
