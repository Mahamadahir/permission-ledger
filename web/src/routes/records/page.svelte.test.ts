import { fireEvent, render, screen, within } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import Page from './+page.svelte';
import { bodyOf, fixtures, lastUrl, mockApi, requestLog, signIn, signOut, type Routes } from '$lib/test-helpers';

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  signOut();
});

async function renderRecords(routes: Routes = {}) {
  const fetchMock = mockApi(routes);
  signIn();
  render(Page);
  // Wait for the initial categories + records load to settle.
  await screen.findByRole('button', { name: 'Clear' });
  return fetchMock;
}

describe('records table', () => {
  it('renders a record row with formatted date and badges', async () => {
    await renderRecords({ 'GET /api/records': { body: [fixtures.record] } });
    const row = (await screen.findByText('Acme')).closest('tr')!;
    expect(within(row).getByText('01 Jun 2026')).toBeInTheDocument();
    expect(within(row).getByText('high')).toBeInTheDocument();
    expect(within(row).getByText('active')).toBeInTheDocument();
    expect(screen.getByText('1 visible record')).toBeInTheDocument();
  });

  it('shows the empty state when there are no records', async () => {
    await renderRecords();
    expect(screen.getByRole('heading', { name: 'No records yet' })).toBeInTheDocument();
  });
});

describe('filters and search', () => {
  it('requeries when the risk filter changes', async () => {
    const fetchMock = await renderRecords();
    await fireEvent.change(screen.getByLabelText('Risk'), { target: { value: 'high' } });
    expect(lastUrl(fetchMock, '/api/records')).toContain('risk_level=high');
  });

  it('adds review_due when the checkbox is ticked', async () => {
    const fetchMock = await renderRecords();
    await fireEvent.click(screen.getByLabelText('Review due'));
    expect(lastUrl(fetchMock, '/api/records')).toContain('review_due=true');
  });

  it('searches on change of the search box', async () => {
    const fetchMock = await renderRecords();
    const box = screen.getByLabelText('Search');
    await fireEvent.input(box, { target: { value: 'acme' } });
    await fireEvent.change(box);
    expect(lastUrl(fetchMock, '/api/records')).toContain('q=acme');
  });

  it('clears filters back to an empty query', async () => {
    const fetchMock = await renderRecords();
    await fireEvent.change(screen.getByLabelText('Risk'), { target: { value: 'high' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Clear' }));
    expect(lastUrl(fetchMock, '/api/records')).toMatch(/\/api\/records\?$/);
  });
});

describe('record modal and CRUD', () => {
  async function openEditor() {
    await fireEvent.click(screen.getAllByRole('button', { name: 'Add record' })[0]);
  }

  async function fillRequired() {
    await fireEvent.input(screen.getByLabelText('Service name'), { target: { value: 'Acme' } });
    await fireEvent.input(screen.getByLabelText('Website URL'), { target: { value: 'https://acme.example' } });
    await fireEvent.input(screen.getByLabelText('Consent type'), { target: { value: 'cookies' } });
  }

  it('keeps Save disabled until the required fields are filled', async () => {
    await renderRecords();
    await openEditor();
    const save = screen.getByRole('button', { name: 'Save record' });
    expect(save).toBeDisabled();
    await fillRequired();
    expect(save).toBeEnabled();
  });

  it('creates a record, closes the modal and shows a notice', async () => {
    const fetchMock = await renderRecords({ 'POST /api/records': { body: fixtures.record } });
    await openEditor();
    await fillRequired();
    await fireEvent.click(screen.getByRole('button', { name: 'Save record' }));

    expect(await screen.findByText('Record saved')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Save record' })).not.toBeInTheDocument();
    expect(bodyOf(fetchMock, 'POST /api/records')).toMatchObject({
      service_name: 'Acme',
      website_url: 'https://acme.example'
    });
  });

  it('edits an existing record with a PUT', async () => {
    const fetchMock = await renderRecords({
      'GET /api/records': { body: [fixtures.record] },
      'PUT /api/records/r1': { body: fixtures.record }
    });
    await fireEvent.click(await screen.findByTitle('Edit'));
    expect((screen.getByLabelText('Service name') as HTMLInputElement).value).toBe('Acme');
    await fireEvent.click(screen.getByRole('button', { name: 'Save record' }));
    expect(bodyOf(fetchMock, 'PUT /api/records/r1')).toMatchObject({ id: 'r1' });
  });

  it('revokes and deletes a record from its row actions', async () => {
    const fetchMock = await renderRecords({
      'GET /api/records': { body: [fixtures.record] },
      'POST /api/records/r1/revoke': { body: fixtures.record },
      'DELETE /api/records/r1': { body: { deleted: true } }
    });
    await fireEvent.click(await screen.findByTitle('Revoke'));
    await fireEvent.click(screen.getByTitle('Delete'));

    const calls = requestLog(fetchMock);
    expect(calls).toContain('POST /api/records/r1/revoke');
    expect(calls).toContain('DELETE /api/records/r1');
  });
});
