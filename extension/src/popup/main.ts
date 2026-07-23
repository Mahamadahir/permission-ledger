import { fetchCategories, getSettings, saveRecord, saveSettings, type Category } from '../shared/api';
import './style.css';

const app = document.querySelector<HTMLDivElement>('#app');
if (!app) throw new Error('Popup root missing');

let categories: Category[] = [];
let currentTab: chrome.tabs.Tab | undefined;
let apiBase = 'http://localhost:3000';
let deviceToken = '';

app.innerHTML = `
  <section class="popup">
    <header><h1>PermissionLedger</h1><span id="status"></span></header>
    <label>API base<input id="apiBase" /></label>
    <label>Device token<input id="deviceToken" type="password" /></label>
    <label>Service<input id="serviceName" /></label>
    <label>Website<input id="websiteUrl" /></label>
    <label>Consent type<input id="consentType" placeholder="Marketing emails, analytics cookies" /></label>
    <label>Category<select id="category"></select></label>
    <div class="grid">
      <label>Risk<select id="risk"><option value="low">Low</option><option value="medium" selected>Medium</option><option value="high">High</option></select></label>
      <label>Review date<input id="reviewDate" type="date" /></label>
    </div>
    <label>Notes<textarea id="notes"></textarea></label>
    <button id="save">Save record</button>
  </section>
`;

const status = getElement<HTMLSpanElement>('status');
const apiBaseInput = getElement<HTMLInputElement>('apiBase');
const tokenInput = getElement<HTMLInputElement>('deviceToken');
const serviceName = getElement<HTMLInputElement>('serviceName');
const websiteUrl = getElement<HTMLInputElement>('websiteUrl');
const consentType = getElement<HTMLInputElement>('consentType');
const categorySelect = getElement<HTMLSelectElement>('category');
const risk = getElement<HTMLSelectElement>('risk');
const reviewDate = getElement<HTMLInputElement>('reviewDate');
const notes = getElement<HTMLTextAreaElement>('notes');
const saveButton = getElement<HTMLButtonElement>('save');

init();
saveButton.addEventListener('click', saveCurrentRecord);
apiBaseInput.addEventListener('change', persistSettings);
tokenInput.addEventListener('change', persistSettings);

async function init() {
  const settings = await getSettings();
  apiBase = settings.apiBase;
  deviceToken = settings.deviceToken;
  apiBaseInput.value = apiBase;
  tokenInput.value = deviceToken;
  const tabs = await chrome.tabs.query({ active: true, currentWindow: true });
  currentTab = tabs[0];
  websiteUrl.value = currentTab?.url ?? '';
  serviceName.value = currentTab?.title ?? hostFromUrl(currentTab?.url ?? '') ?? '';
  reviewDate.value = nextMonth();
  await loadCategories();
}

async function loadCategories() {
  try {
    categories = await fetchCategories(apiBase);
    categorySelect.innerHTML = categories.map((category) => `<option value="${category.id}">${category.name}</option>`).join('');
    setStatus('Ready');
  } catch (error) {
    setStatus(error instanceof Error ? error.message : 'Setup needed');
  }
}

async function saveCurrentRecord() {
  try {
    saveButton.disabled = true;
    await persistSettings();
    if (!deviceToken) throw new Error('Pair the extension and paste a device token');
    await saveRecord(apiBase, deviceToken, {
      service_name: serviceName.value,
      website_url: websiteUrl.value,
      consent_type: consentType.value,
      category_id: categorySelect.value,
      date_given: new Date().toISOString().slice(0, 10),
      review_date: reviewDate.value || null,
      expiry_date: null,
      risk_level: risk.value,
      status: 'active',
      notes: notes.value || null
    });
    setStatus('Saved');
  } catch (error) {
    setStatus(error instanceof Error ? error.message : 'Save failed');
  } finally {
    saveButton.disabled = false;
  }
}

async function persistSettings() {
  apiBase = apiBaseInput.value.trim() || 'http://localhost:3000';
  deviceToken = tokenInput.value.trim();
  await saveSettings(apiBase, deviceToken);
}

function getElement<T extends HTMLElement>(id: string): T {
  const element = document.getElementById(id);
  if (!element) throw new Error(`${id} missing`);
  return element as T;
}

function setStatus(message: string) {
  status.textContent = message;
}

function hostFromUrl(value: string) {
  try {
    return new URL(value).hostname.replace(/^www\./, '');
  } catch {
    return '';
  }
}

function nextMonth() {
  const date = new Date();
  date.setMonth(date.getMonth() + 1);
  return date.toISOString().slice(0, 10);
}
