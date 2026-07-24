import { render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import Page from './+page.svelte';
import { mockApi, signIn, signOut } from '$lib/test-helpers';

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  signOut();
});

describe('exports page', () => {
  it('links to the backend export endpoints', () => {
    mockApi();
    signIn();
    render(Page);

    expect(screen.getByRole('link', { name: 'Export CSV' })).toHaveAttribute(
      'href',
      'http://localhost:3000/api/export.csv'
    );
    expect(screen.getByRole('link', { name: 'Export JSON' })).toHaveAttribute(
      'href',
      'http://localhost:3000/api/export.json'
    );
  });

  it('documents the CSV columns', () => {
    mockApi();
    signIn();
    render(Page);
    expect(screen.getByText(/id, service_name, website_url/)).toBeInTheDocument();
  });
});
