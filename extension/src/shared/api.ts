export type Category = { id: string; name: string; slug: string };

export async function getSettings() {
  return chrome.storage.local.get({ apiBase: 'http://localhost:3000', deviceToken: '' });
}

export async function saveSettings(apiBase: string, deviceToken: string) {
  await chrome.storage.local.set({ apiBase, deviceToken });
}

export async function fetchCategories(apiBase: string): Promise<Category[]> {
  const response = await fetch(`${apiBase}/api/categories`);
  if (!response.ok) throw new Error('Could not load categories');
  return response.json();
}

export async function saveRecord(apiBase: string, token: string, body: unknown) {
  const response = await fetch(`${apiBase}/api/extension/records`, {
    method: 'POST',
    headers: { 'content-type': 'application/json', authorization: `Bearer ${token}` },
    body: JSON.stringify(body)
  });
  if (!response.ok) {
    const payload = await response.json().catch(() => ({ error: response.statusText }));
    throw new Error(payload.error ?? response.statusText);
  }
  return response.json();
}
