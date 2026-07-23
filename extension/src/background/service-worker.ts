chrome.runtime.onInstalled.addListener(() => {
  chrome.storage.local.set({ apiBase: 'http://localhost:3000' });
});
