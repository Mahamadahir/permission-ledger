import { fireEvent, render, screen, within } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import Page from './+page.svelte';
import { bodyOf, fixtures, lastUrl, mockApi, type Routes } from '$lib/test-helpers';

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

const authed: Routes = { 'GET /api/auth/me': { body: { user: fixtures.user, csrf_token: 'csrf1' } } };

async function renderAuthed(routes: Routes = {}) {
  const fetchMock = mockApi({ ...authed, ...routes });
  render(Page);
  await screen.findByRole('button', { name: 'Log out' });
  return fetchMock;
}

describe('auth gate', () => {
  it('shows the login panel when unauthenticated', async () => {
    mockApi({ 'GET /api/auth/me': { status: 401 } });
    render(Page);
    expect(await screen.findByRole('heading', { name: /Privacy decisions/i })).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Add record' })).not.toBeInTheDocument();
  });

  it('reveals the display name field only on the register tab', async () => {
    mockApi({ 'GET /api/auth/me': { status: 401 } });
    render(Page);
    await screen.findByRole('heading', { name: /Privacy decisions/i });
    expect(screen.queryByLabelText('Display name')).not.toBeInTheDocument();

    await fireEvent.click(screen.getByRole('button', { name: 'Register' }));
    expect(screen.getByLabelText('Display name')).toBeInTheDocument();
  });

  it('logs in and renders the dashboard on success', async () => {
    const fetchMock = mockApi({
      'GET /api/auth/me': { status: 401 },
      'POST /api/auth/login': { body: { user: fixtures.user, csrf_token: 'csrf1' } }
    });
    render(Page);
    await fireEvent.input(await screen.findByLabelText('Email'), { target: { value: 'sam@example.com' } });
    await fireEvent.input(screen.getByLabelText('Password'), { target: { value: 'correcthorse1' } });
    await fireEvent.click(screen.getAllByRole('button', { name: 'Login' }).at(-1)!);

    expect(await screen.findByRole('button', { name: 'Log out' })).toBeInTheDocument();
    expect(bodyOf(fetchMock, 'POST /api/auth/login')).toMatchObject({ email: 'sam@example.com', password: 'correcthorse1' });
  });

  it('surfaces a failed login error', async () => {
    mockApi({
      'GET /api/auth/me': { status: 401 },
      'POST /api/auth/login': { status: 401, body: { error: 'authentication required' } }
    });
    render(Page);
    await fireEvent.input(await screen.findByLabelText('Email'), { target: { value: 'sam@example.com' } });
    await fireEvent.input(screen.getByLabelText('Password'), { target: { value: 'wrong' } });
    await fireEvent.click(screen.getAllByRole('button', { name: 'Login' }).at(-1)!);

    expect(await screen.findByText('authentication required')).toBeInTheDocument();
  });
});

describe('dashboard rendering', () => {
  it('shows the summary metrics', async () => {
    await renderAuthed();
    const metrics = screen.getByRole('region', { name: 'Summary metrics' });
    expect(within(within(metrics).getByText('Active permissions').closest('article')!).getByText('3')).toBeInTheDocument();
    expect(within(within(metrics).getByText('Revoked').closest('article')!).getByText('2')).toBeInTheDocument();
  });

  it('renders a record row with formatted date and badges', async () => {
    await renderAuthed({ 'GET /api/records': { body: [fixtures.record] } });
    const row = (await screen.findByText('Acme')).closest('tr')!;
    expect(within(row).getByText('01 Jun 2026')).toBeInTheDocument();
    expect(within(row).getByText('high')).toBeInTheDocument();
    expect(within(row).getByText('active')).toBeInTheDocument();
    expect(screen.getByText('1 visible record')).toBeInTheDocument();
  });

  it('shows the empty state when there are no records', async () => {
    await renderAuthed();
    expect(screen.getByRole('heading', { name: 'No records yet' })).toBeInTheDocument();
  });

  it('exports link to the backend export endpoints', async () => {
    await renderAuthed();
    expect(screen.getByRole('link', { name: 'Export CSV' })).toHaveAttribute('href', 'http://localhost:3000/api/export.csv');
    expect(screen.getByRole('link', { name: 'Export JSON' })).toHaveAttribute('href', 'http://localhost:3000/api/export.json');
  });
});

describe('filters and search', () => {
  it('requeries records when the risk filter changes', async () => {
    const fetchMock = await renderAuthed();
    await fireEvent.change(screen.getByLabelText('Risk'), { target: { value: 'high' } });
    expect(lastUrl(fetchMock, '/api/records')).toContain('risk_level=high');
  });

  it('adds review_due when the checkbox is ticked', async () => {
    const fetchMock = await renderAuthed();
    await fireEvent.click(screen.getByLabelText('Review due'));
    expect(lastUrl(fetchMock, '/api/records')).toContain('review_due=true');
  });

  it('searches on change of the global search box', async () => {
    const fetchMock = await renderAuthed();
    const box = screen.getByLabelText('Search');
    await fireEvent.input(box, { target: { value: 'acme' } });
    await fireEvent.change(box);
    expect(lastUrl(fetchMock, '/api/records')).toContain('q=acme');
  });

  it('clears filters back to an empty query', async () => {
    const fetchMock = await renderAuthed();
    await fireEvent.change(screen.getByLabelText('Risk'), { target: { value: 'high' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Clear' }));
    expect(lastUrl(fetchMock, '/api/records')).toMatch(/\/api\/records\?$/);
  });
});

describe('record modal and CRUD', () => {
  async function openEditor() {
    await fireEvent.click(screen.getAllByRole('button', { name: 'Add record' })[0]);
  }

  it('keeps Save disabled until the required fields are filled', async () => {
    await renderAuthed();
    await openEditor();
    const save = screen.getByRole('button', { name: 'Save record' });
    expect(save).toBeDisabled();

    await fireEvent.input(screen.getByLabelText('Service name'), { target: { value: 'Acme' } });
    await fireEvent.input(screen.getByLabelText('Website URL'), { target: { value: 'https://acme.example' } });
    await fireEvent.input(screen.getByLabelText('Consent type'), { target: { value: 'cookies' } });
    expect(save).toBeEnabled();
  });

  it('creates a record, closes the modal and shows a notice', async () => {
    const fetchMock = await renderAuthed({ 'POST /api/records': { body: fixtures.record } });
    await openEditor();
    await fireEvent.input(screen.getByLabelText('Service name'), { target: { value: 'Acme' } });
    await fireEvent.input(screen.getByLabelText('Website URL'), { target: { value: 'https://acme.example' } });
    await fireEvent.input(screen.getByLabelText('Consent type'), { target: { value: 'cookies' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Save record' }));

    expect(await screen.findByText('Record saved')).toBeInTheDocument();
    expect(screen.queryByRole('heading', { name: 'Add record' })).not.toBeInTheDocument();
    expect(bodyOf(fetchMock, 'POST /api/records')).toMatchObject({ service_name: 'Acme', website_url: 'https://acme.example' });
  });

  it('edits an existing record with a PUT', async () => {
    const fetchMock = await renderAuthed({
      'GET /api/records': { body: [fixtures.record] },
      'PUT /api/records/r1': { body: fixtures.record }
    });
    await fireEvent.click(await screen.findByTitle('Edit'));
    const serviceName = screen.getByLabelText('Service name') as HTMLInputElement;
    expect(serviceName.value).toBe('Acme');
    await fireEvent.click(screen.getByRole('button', { name: 'Save record' }));
    expect(bodyOf(fetchMock, 'PUT /api/records/r1')).toMatchObject({ id: 'r1' });
  });

  it('revokes and deletes a record via its row actions', async () => {
    const fetchMock = await renderAuthed({
      'GET /api/records': { body: [fixtures.record] },
      'POST /api/records/r1/revoke': { body: fixtures.record },
      'DELETE /api/records/r1': { body: { deleted: true } }
    });
    await fireEvent.click(await screen.findByTitle('Revoke'));
    await fireEvent.click(screen.getByTitle('Delete'));
    const calls = fetchMock.mock.calls.map(([url, opt = {}]) => `${(opt.method ?? 'GET').toUpperCase()} ${new URL(url).pathname}`);
    expect(calls).toContain('POST /api/records/r1/revoke');
    expect(calls).toContain('DELETE /api/records/r1');
  });
});

describe('extension pairing', () => {
  it('pairs a device and shows the returned token', async () => {
    await renderAuthed({ 'POST /api/extension/pair': { body: { id: 'd9', name: 'Chrome extension', token: 'tok-abc', created_at: 'x' } } });
    await fireEvent.click(screen.getByRole('button', { name: 'Pair extension' }));
    expect(await screen.findByText('tok-abc')).toBeInTheDocument();
  });

  it('lists a paired device and revokes it', async () => {
    const fetchMock = await renderAuthed({
      'GET /api/extension/devices': { body: [fixtures.device] },
      'DELETE /api/extension/devices/d1': { body: { revoked: true } }
    });
    await fireEvent.click(await screen.findByRole('button', { name: 'Revoke' }));
    const calls = fetchMock.mock.calls.map(([url, opt = {}]) => `${(opt.method ?? 'GET').toUpperCase()} ${new URL(url).pathname}`);
    expect(calls).toContain('DELETE /api/extension/devices/d1');
  });
});

describe('session lifecycle', () => {
  it('logs out and returns to the auth screen', async () => {
    await renderAuthed();
    await fireEvent.click(screen.getByRole('button', { name: 'Log out' }));
    expect(await screen.findByRole('heading', { name: /Privacy decisions/i })).toBeInTheDocument();
  });
});
