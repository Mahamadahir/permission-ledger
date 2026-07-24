import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const categories = [{ id: 'c1', name: 'Cookies', slug: 'cookies' }];

function stubChrome(settings = { apiBase: 'http://localhost:3000', deviceToken: '' }) {
  const store = {
    get: vi.fn().mockResolvedValue(settings),
    set: vi.fn().mockResolvedValue(undefined)
  };
  vi.stubGlobal('chrome', {
    storage: { local: store },
    tabs: { query: vi.fn().mockResolvedValue([{ url: 'https://www.acme.com/pricing', title: 'Acme Pricing' }]) }
  });
  return store;
}

function stubFetch(recordOk = true) {
  vi.stubGlobal(
    'fetch',
    vi.fn(async (input: string, options: RequestInit = {}) => {
      const path = new URL(input).pathname;
      if (path === '/api/categories') return { ok: true, json: async () => categories } as Response;
      if (path === '/api/extension/records') {
        return { ok: recordOk, statusText: 'Bad', json: async () => ({ error: 'nope' }) } as Response;
      }
      throw new Error(`unexpected ${options.method} ${path}`);
    })
  );
}

async function loadPopup() {
  document.body.innerHTML = '<main id="app"></main>';
  vi.resetModules();
  await import('./main');
  // Let the async init() (settings, tab query, categories) settle.
  await vi.waitFor(() => expect(document.getElementById('status')?.textContent).toBe('Ready'));
}

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  document.body.innerHTML = '';
});

describe('popup init', () => {
  beforeEach(() => {
    stubChrome();
    stubFetch();
  });

  it('prefills the form from the active tab and defaults', async () => {
    await loadPopup();
    expect((document.getElementById('websiteUrl') as HTMLInputElement).value).toBe('https://www.acme.com/pricing');
    expect((document.getElementById('serviceName') as HTMLInputElement).value).toBe('Acme Pricing');
    expect((document.getElementById('reviewDate') as HTMLInputElement).value).toMatch(/^\d{4}-\d{2}-\d{2}$/);
  });

  it('populates the category select from the API', async () => {
    await loadPopup();
    const select = document.getElementById('category') as HTMLSelectElement;
    expect(select.options).toHaveLength(1);
    expect(select.options[0].value).toBe('c1');
    expect(select.options[0].textContent).toBe('Cookies');
  });
});

describe('popup save flow', () => {
  it('refuses to save without a device token', async () => {
    stubChrome({ apiBase: 'http://localhost:3000', deviceToken: '' });
    stubFetch();
    await loadPopup();

    (document.getElementById('save') as HTMLButtonElement).click();
    await vi.waitFor(() =>
      expect(document.getElementById('status')?.textContent).toBe('Pair the extension and paste a device token')
    );
  });

  it('saves a record when a token is present', async () => {
    stubChrome({ apiBase: 'http://localhost:3000', deviceToken: 'tok-123' });
    stubFetch();
    await loadPopup();

    (document.getElementById('save') as HTMLButtonElement).click();
    await vi.waitFor(() => expect(document.getElementById('status')?.textContent).toBe('Saved'));
    const calls = (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls.map(
      ([url]) => new URL(url).pathname
    );
    expect(calls).toContain('/api/extension/records');
  });
});
