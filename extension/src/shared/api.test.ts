import { afterEach, describe, expect, it, vi } from 'vitest';
import { fetchCategories, getSettings, saveRecord, saveSettings } from './api';

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

function stubFetch(response: Partial<Response> & { json: () => Promise<unknown> }) {
  const fetchMock = vi.fn().mockResolvedValue(response);
  vi.stubGlobal('fetch', fetchMock);
  return fetchMock;
}

describe('fetchCategories', () => {
  it('requests the categories endpoint and returns the parsed body', async () => {
    const fetchMock = stubFetch({ ok: true, json: async () => [{ id: '1', name: 'Cookies', slug: 'cookies' }] });
    const categories = await fetchCategories('http://api.test');
    expect(fetchMock).toHaveBeenCalledWith('http://api.test/api/categories');
    expect(categories).toEqual([{ id: '1', name: 'Cookies', slug: 'cookies' }]);
  });

  it('throws when the response is not ok', async () => {
    stubFetch({ ok: false, json: async () => ({}) });
    await expect(fetchCategories('http://api.test')).rejects.toThrow('Could not load categories');
  });
});

describe('saveRecord', () => {
  it('posts with a bearer token, JSON content type and serialised body', async () => {
    const fetchMock = stubFetch({ ok: true, json: async () => ({ id: 'r1' }) });
    const body = { service_name: 'Acme' };
    const result = await saveRecord('http://api.test', 'tok123', body);

    expect(result).toEqual({ id: 'r1' });
    const [url, init] = fetchMock.mock.calls[0];
    expect(url).toBe('http://api.test/api/extension/records');
    expect(init.method).toBe('POST');
    expect(init.headers.authorization).toBe('Bearer tok123');
    expect(init.headers['content-type']).toBe('application/json');
    expect(JSON.parse(init.body)).toEqual(body);
  });

  it('surfaces the server error message', async () => {
    stubFetch({ ok: false, statusText: 'Bad', json: async () => ({ error: 'invalid risk level' }) });
    await expect(saveRecord('http://api.test', 'tok', {})).rejects.toThrow('invalid risk level');
  });

  it('falls back to the status text when the error body is not JSON', async () => {
    stubFetch({
      ok: false,
      statusText: 'Unauthorized',
      json: async () => {
        throw new Error('not json');
      }
    });
    await expect(saveRecord('http://api.test', 'tok', {})).rejects.toThrow('Unauthorized');
  });
});

describe('settings storage', () => {
  it('reads settings with defaults and writes them back', async () => {
    const store = {
      get: vi.fn().mockResolvedValue({ apiBase: 'http://saved', deviceToken: 'd' }),
      set: vi.fn().mockResolvedValue(undefined)
    };
    vi.stubGlobal('chrome', { storage: { local: store } });

    const settings = await getSettings();
    expect(store.get).toHaveBeenCalledWith({ apiBase: 'http://localhost:3000', deviceToken: '' });
    expect(settings).toEqual({ apiBase: 'http://saved', deviceToken: 'd' });

    await saveSettings('http://new', 'token');
    expect(store.set).toHaveBeenCalledWith({ apiBase: 'http://new', deviceToken: 'token' });
  });
});
