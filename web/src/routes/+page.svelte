<script lang="ts">
  import { buildRecordQuery, formatDate, serviceInitial } from '$lib/records';

  const apiBase = import.meta.env.VITE_API_BASE ?? 'http://localhost:3000';

  type User = { id: string; email: string; display_name: string | null };
  type Category = { id: string; slug: string; name: string };
  type RecordItem = {
    id: string;
    service_name: string;
    website_url: string;
    category_id: string;
    category_name: string;
    consent_type: string;
    date_given: string;
    review_date: string | null;
    expiry_date: string | null;
    status: string;
    risk_level: string;
    source: string;
    notes: string | null;
  };
  type Dashboard = {
    summary: { active: number; review_due: number; expired: number; revoked: number; high_risk: number };
    recent: RecordItem[];
    categories: { name: string; count: number }[];
    services: { name: string; count: number }[];
  };
  type Device = { id: string; name: string; last_used_at: string | null; revoked_at: string | null; created_at: string };

  let user: User | null = null;
  let csrfToken = '';
  let mode: 'login' | 'register' = 'login';
  let email = '';
  let password = '';
  let displayName = '';
  let error = '';
  let notice = '';
  let categories: Category[] = [];
  let records: RecordItem[] = [];
  let devices: Device[] = [];
  let dashboard: Dashboard | null = null;
  let search = '';
  let categoryFilter = '';
  let statusFilter = '';
  let riskFilter = '';
  let sourceFilter = '';
  let reviewDueOnly = false;
  let deviceName = 'Chrome extension';
  let pairedToken = '';
  let showEditor = false;

  const navigation = ['Dashboard', 'Records', 'Extension', 'Exports', 'Settings'];

  let form = emptyForm();

  $: canSave = form.service_name && form.website_url && form.consent_type && form.category_id;
  $: summary = dashboard?.summary ?? { active: 0, review_due: 0, expired: 0, revoked: 0, high_risk: 0 };

  function emptyForm() {
    return {
      id: '',
      service_name: '',
      website_url: '',
      consent_type: '',
      category_id: '',
      date_given: new Date().toISOString().slice(0, 10),
      review_date: '',
      expiry_date: '',
      status: 'active',
      risk_level: 'medium',
      notes: ''
    };
  }

  async function api(path: string, options: RequestInit = {}) {
    const headers = new Headers(options.headers);
    if (!headers.has('content-type') && options.body) headers.set('content-type', 'application/json');
    if (csrfToken && ['POST', 'PUT', 'DELETE'].includes(options.method ?? 'GET')) headers.set('x-csrf-token', csrfToken);
    const response = await fetch(`${apiBase}${path}`, { ...options, headers, credentials: 'include' });
    if (!response.ok) {
      const body = await response.json().catch(() => ({ error: response.statusText }));
      throw new Error(body.error ?? response.statusText);
    }
    if (response.status === 204) return null;
    return response.json();
  }

  async function loadSession() {
    try {
      const data = await api('/api/auth/me');
      user = data.user;
      csrfToken = data.csrf_token;
      await loadData();
    } catch {
      user = null;
    }
  }

  async function authenticate() {
    error = '';
    notice = '';
    try {
      const path = mode === 'login' ? '/api/auth/login' : '/api/auth/register';
      const data = await api(path, { method: 'POST', body: JSON.stringify({ email, password, display_name: displayName || null }) });
      user = data.user;
      csrfToken = data.csrf_token;
      await loadData();
    } catch (err) {
      error = err instanceof Error ? err.message : 'Authentication failed';
    }
  }

  async function logout() {
    await api('/api/auth/logout', { method: 'POST' });
    user = null;
    csrfToken = '';
    records = [];
    dashboard = null;
    devices = [];
  }

  async function loadData() {
    const query = buildRecordQuery({ search, categoryFilter, statusFilter, riskFilter, sourceFilter, reviewDueOnly });
    [categories, records, dashboard, devices] = await Promise.all([
      api('/api/categories'),
      api(`/api/records?${query}`),
      api('/api/dashboard'),
      api('/api/extension/devices')
    ]);
    if (!form.category_id && categories[0]) form.category_id = categories[0].id;
  }

  async function saveRecord() {
    if (!canSave) return;
    error = '';
    const payload = { ...form, review_date: form.review_date || null, expiry_date: form.expiry_date || null, notes: form.notes || null };
    const path = form.id ? `/api/records/${form.id}` : '/api/records';
    await api(path, { method: form.id ? 'PUT' : 'POST', body: JSON.stringify(payload) });
    resetForm();
    showEditor = false;
    await loadData();
    notice = 'Record saved';
  }

  function startNewRecord() {
    resetForm();
    showEditor = true;
  }

  function editRecord(record: RecordItem) {
    form = {
      id: record.id,
      service_name: record.service_name,
      website_url: record.website_url,
      consent_type: record.consent_type,
      category_id: record.category_id,
      date_given: record.date_given,
      review_date: record.review_date ?? '',
      expiry_date: record.expiry_date ?? '',
      status: record.status,
      risk_level: record.risk_level,
      notes: record.notes ?? ''
    };
    showEditor = true;
  }

  async function revokeRecord(id: string) {
    await api(`/api/records/${id}/revoke`, { method: 'POST' });
    await loadData();
  }

  async function deleteRecord(id: string) {
    await api(`/api/records/${id}`, { method: 'DELETE' });
    await loadData();
  }

  async function pairExtension() {
    const data = await api('/api/extension/pair', { method: 'POST', body: JSON.stringify({ device_name: deviceName }) });
    pairedToken = data.token;
    await loadData();
  }

  async function revokeDevice(id: string) {
    await api(`/api/extension/devices/${id}`, { method: 'DELETE' });
    await loadData();
  }

  function resetForm() {
    form = emptyForm();
    form.category_id = categories[0]?.id ?? '';
  }

  function clearFilters() {
    search = '';
    categoryFilter = '';
    statusFilter = '';
    riskFilter = '';
    sourceFilter = '';
    reviewDueOnly = false;
    loadData();
  }

  loadSession();
</script>

<svelte:head><title>PermissionLedger Privacy Dashboard</title></svelte:head>

{#if !user}
  <main class="auth-shell">
    <section class="auth-copy">
      <p class="eyebrow">PermissionLedger</p>
      <h1>Privacy decisions, kept accountable.</h1>
      <p>Track consent records, app permissions, OAuth connections and review dates from one private dashboard.</p>
    </section>
    <section class="auth-panel" aria-label="Authentication">
      <div class="tabs">
        <button class:active={mode === 'login'} on:click={() => (mode = 'login')}>Login</button>
        <button class:active={mode === 'register'} on:click={() => (mode = 'register')}>Register</button>
      </div>
      {#if mode === 'register'}<label>Display name<input bind:value={displayName} autocomplete="name" /></label>{/if}
      <label>Email<input bind:value={email} autocomplete="email" type="email" /></label>
      <label>Password<input bind:value={password} autocomplete="current-password" type="password" /></label>
      <button class="primary full" on:click={authenticate}>{mode === 'login' ? 'Login' : 'Create account'}</button>
      {#if error}<p class="message error">{error}</p>{/if}
    </section>
  </main>
{:else}
  <main class="app-shell">
    <aside class="sidebar" aria-label="Primary">
      <div class="brand"><span class="brand-mark">PL</span><div><strong>PermissionLedger</strong><span>Privacy Dashboard</span></div></div>
      <nav>{#each navigation as item}<a class:active={item === 'Dashboard'} href={`#${item.toLowerCase()}`}>{item}</a>{/each}</nav>
    </aside>

    <section class="dashboard">
      <header class="topbar">
        <label class="global-search"><span>Search</span><input bind:value={search} on:change={loadData} placeholder="Search services, websites, consent types" /></label>
        <div class="topbar-actions">
          <button class="icon-button" aria-label="Settings">ST</button>
          <div class="user-chip"><span>{serviceInitial(user.email)}</span><div><strong>{user.display_name ?? user.email}</strong><small>{user.email}</small></div></div>
          <button class="secondary" on:click={logout}>Log out</button>
        </div>
      </header>

      <div class="content">
        <section class="page-heading">
          <div><p class="eyebrow">Dashboard</p><h1>Privacy permissions</h1></div>
          <div class="heading-actions">
            <a class="secondary" href={`${apiBase}/api/export.csv`}>Export CSV</a>
            <a class="secondary" href={`${apiBase}/api/export.json`}>Export JSON</a>
            <button class="primary" on:click={startNewRecord}>Add record</button>
          </div>
        </section>

        <section class="metrics" aria-label="Summary metrics">
          <article><span>Active permissions</span><strong>{summary.active}</strong></article>
          <article><span>Review due</span><strong>{summary.review_due}</strong></article>
          <article><span>Expired</span><strong>{summary.expired}</strong></article>
          <article><span>Revoked</span><strong>{summary.revoked}</strong></article>
          <article><span>High risk</span><strong>{summary.high_risk}</strong></article>
        </section>

        <section class="layout-grid">
          <div class="records-panel" id="records">
            <div class="panel-header"><div><h2>Consent records</h2><p>{records.length} visible record{records.length === 1 ? '' : 's'}</p></div>{#if notice}<p class="message success">{notice}</p>{/if}</div>
            <div class="filters" aria-label="Record filters">
              <label>Category<select bind:value={categoryFilter} on:change={loadData}><option value="">All categories</option>{#each categories as category}<option value={category.id}>{category.name}</option>{/each}</select></label>
              <label>Status<select bind:value={statusFilter} on:change={loadData}><option value="">All statuses</option><option value="active">Active</option><option value="needs_review">Review due</option><option value="expired">Expired</option><option value="revoked">Revoked</option></select></label>
              <label>Risk<select bind:value={riskFilter} on:change={loadData}><option value="">All risk</option><option value="low">Low</option><option value="medium">Medium</option><option value="high">High</option></select></label>
              <label>Source<select bind:value={sourceFilter} on:change={loadData}><option value="">All sources</option><option value="manual">Manual</option><option value="extension">Extension</option></select></label>
              <label class="check-filter"><input bind:checked={reviewDueOnly} on:change={loadData} type="checkbox" />Review due</label>
              <button class="ghost" on:click={clearFilters}>Clear</button>
            </div>
            <div class="table-wrap">
              {#if records.length}
                <table>
                  <thead><tr><th>Service</th><th>Category</th><th>Consent type</th><th>Date given</th><th>Review date</th><th>Risk</th><th>Status</th><th>Source</th><th>Actions</th></tr></thead>
                  <tbody>{#each records as record}<tr>
                    <td><div class="service-cell"><span>{serviceInitial(record.service_name)}</span><div><strong>{record.service_name}</strong><a href={record.website_url}>{record.website_url}</a></div></div></td>
                    <td><span class="badge neutral">{record.category_name}</span></td>
                    <td>{record.consent_type}</td>
                    <td>{formatDate(record.date_given)}</td>
                    <td>{formatDate(record.review_date)}</td>
                    <td><span class={`badge risk-${record.risk_level}`}>{record.risk_level}</span></td>
                    <td><span class={`badge status-${record.status}`}>{record.status.replace('_', ' ')}</span></td>
                    <td>{record.source}</td>
                    <td><div class="row-actions"><button class="icon-button" title="Edit" on:click={() => editRecord(record)}>ED</button><button class="icon-button" title="Revoke" on:click={() => revokeRecord(record.id)}>RV</button><button class="icon-button danger-text" title="Delete" on:click={() => deleteRecord(record.id)}>DL</button></div></td>
                  </tr>{/each}</tbody>
                </table>
              {:else}
                <div class="empty-state"><h3>No records yet</h3><p>Add the first consent or permission record to start tracking review dates.</p><button class="primary" on:click={startNewRecord}>Add record</button></div>
              {/if}
            </div>
          </div>

          <aside class="utility-column" id="extension">
            <section class="utility-panel"><div class="panel-header compact"><div><h2>Extension pairing</h2><p>Pair a browser device with a limited token.</p></div></div><label>Device name<input bind:value={deviceName} /></label><button class="primary full" on:click={pairExtension}>Pair extension</button>{#if pairedToken}<div class="token-box"><span>New token</span><code>{pairedToken}</code></div>{/if}</section>
            <section class="utility-panel"><div class="panel-header compact"><div><h2>Paired devices</h2><p>{devices.length} device{devices.length === 1 ? '' : 's'}</p></div></div>{#if devices.length}<ul class="device-list">{#each devices as device}<li><div><strong>{device.name}</strong><span>{device.last_used_at ? `Last used ${formatDate(device.last_used_at)}` : 'Never used'}</span></div><button class="ghost danger-text" on:click={() => revokeDevice(device.id)}>Revoke</button></li>{/each}</ul>{:else}<div class="small-empty">No paired devices.</div>{/if}</section>
            <section class="utility-panel"><div class="panel-header compact"><div><h2>No recent alerts</h2><p>Privacy policy monitoring has no active alerts.</p></div></div></section>
          </aside>
        </section>
      </div>
    </section>
  </main>

  {#if showEditor}
    <div class="modal-backdrop" role="presentation">
      <form class="record-modal" on:submit|preventDefault={saveRecord}>
        <div class="modal-header"><div><p class="eyebrow">Consent record</p><h2>{form.id ? 'Edit record' : 'Add record'}</h2></div><button class="icon-button" type="button" aria-label="Close" on:click={() => (showEditor = false)}>CL</button></div>
        <div class="form-grid">
          <label>Service name<input bind:value={form.service_name} /></label><label>Website URL<input bind:value={form.website_url} placeholder="https://example.com" /></label>
          <label>Category<select bind:value={form.category_id}>{#each categories as category}<option value={category.id}>{category.name}</option>{/each}</select></label><label>Consent type<input bind:value={form.consent_type} /></label>
          <label>Date given<input bind:value={form.date_given} type="date" /></label><label>Review date<input bind:value={form.review_date} type="date" /></label>
          <label>Expiry date<input bind:value={form.expiry_date} type="date" /></label><label>Risk level<select bind:value={form.risk_level}><option value="low">Low</option><option value="medium">Medium</option><option value="high">High</option></select></label>
          <label>Status<select bind:value={form.status}><option value="active">Active</option><option value="needs_review">Review due</option><option value="expired">Expired</option><option value="revoked">Revoked</option></select></label><label class="wide">Notes<textarea bind:value={form.notes}></textarea></label>
        </div>
        <div class="modal-actions"><button class="secondary" type="button" on:click={resetForm}>Clear</button><button class="primary" disabled={!canSave}>Save record</button></div>
        {#if error}<p class="message error">{error}</p>{/if}
      </form>
    </div>
  {/if}
{/if}

<style>
  :global(body) { margin: 0; background: #f8fafc; color: #1e293b; font-family: Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; }
  :global(*) { box-sizing: border-box; }
  h1, h2, h3, p { margin: 0; } h1 { font-size: 24px; line-height: 32px; } h2 { font-size: 18px; line-height: 26px; }
  button, input, select, textarea, a.secondary { border-radius: 6px; font: inherit; } button, a.secondary { min-height: 36px; cursor: pointer; text-decoration: none; }
  input, select, textarea { width: 100%; min-height: 38px; border: 1px solid #cbd5e1; background: #ffffff; color: #1e293b; padding: 8px 10px; }
  input:focus, select:focus, textarea:focus { border-color: #2563eb; box-shadow: 0 0 0 3px rgb(37 99 235 / 16%); outline: none; } textarea { min-height: 94px; resize: vertical; }
  label { display: grid; gap: 6px; color: #475569; font-size: 12px; font-weight: 700; }
  .primary, .secondary, .ghost, .icon-button { display: inline-flex; align-items: center; justify-content: center; gap: 8px; border: 1px solid transparent; padding: 0 14px; font-size: 13px; font-weight: 700; }
  .primary { background: #2563eb; color: #ffffff; } .secondary { background: #ffffff; border-color: #e2e8f0; color: #1e293b; } .ghost { background: transparent; border-color: #e2e8f0; color: #334155; }
  .icon-button { width: 34px; min-height: 34px; padding: 0; background: #ffffff; border-color: #e2e8f0; color: #475569; font-size: 11px; } .full { width: 100%; } .danger-text { color: #b91c1c; } button:disabled { cursor: not-allowed; opacity: 0.55; }
  .eyebrow { color: #2563eb; font-size: 12px; font-weight: 800; letter-spacing: 0; text-transform: uppercase; }
  .auth-shell { display: grid; grid-template-columns: minmax(0, 1fr) 420px; gap: 48px; min-height: 100vh; align-items: center; max-width: 1120px; margin: 0 auto; padding: 32px; }
  .auth-copy { display: grid; gap: 14px; } .auth-copy h1 { max-width: 620px; font-size: 40px; line-height: 48px; } .auth-copy p:last-child { max-width: 560px; color: #64748b; font-size: 16px; line-height: 26px; }
  .auth-panel, .records-panel, .utility-panel, .metrics article, .record-modal { background: #ffffff; border: 1px solid #e2e8f0; border-radius: 8px; }
  .auth-panel { display: grid; gap: 14px; padding: 18px; } .tabs { display: grid; grid-template-columns: 1fr 1fr; gap: 8px; } .tabs button { background: #f8fafc; border: 1px solid #e2e8f0; color: #475569; } .tabs .active { background: #1e293b; color: #ffffff; }
  .app-shell { display: grid; grid-template-columns: 260px minmax(0, 1fr); min-height: 100vh; } .sidebar { position: sticky; top: 0; height: 100vh; border-right: 1px solid #e2e8f0; background: #ffffff; padding: 20px 16px; }
  .brand { display: flex; gap: 12px; align-items: center; padding: 0 4px 24px; } .brand-mark { display: inline-flex; width: 38px; height: 38px; align-items: center; justify-content: center; border-radius: 8px; background: #2563eb; color: #ffffff; font-size: 13px; font-weight: 800; } .brand strong, .brand span { display: block; } .brand span:not(.brand-mark) { color: #64748b; font-size: 12px; }
  nav { display: grid; gap: 4px; } nav a { border-radius: 6px; color: #475569; padding: 10px 12px; text-decoration: none; font-size: 14px; font-weight: 700; } nav a.active { background: #eff6ff; color: #1d4ed8; }
  .dashboard { min-width: 0; } .topbar { display: flex; min-height: 68px; align-items: center; justify-content: space-between; gap: 16px; border-bottom: 1px solid #e2e8f0; background: #ffffff; padding: 12px 24px; } .global-search { width: min(520px, 100%); } .global-search span { position: absolute; width: 1px; height: 1px; overflow: hidden; clip: rect(0 0 0 0); }
  .topbar-actions, .heading-actions, .row-actions, .modal-actions { display: flex; align-items: center; gap: 8px; } .user-chip { display: flex; align-items: center; gap: 10px; min-width: 0; } .user-chip > span { display: inline-flex; width: 34px; height: 34px; align-items: center; justify-content: center; border-radius: 50%; background: #e0f2fe; color: #0369a1; font-size: 12px; font-weight: 800; } .user-chip strong, .user-chip small { display: block; max-width: 220px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; } .user-chip small { color: #64748b; }
  .content { display: grid; gap: 20px; max-width: 1440px; margin: 0 auto; padding: 24px; } .page-heading, .panel-header { display: flex; align-items: center; justify-content: space-between; gap: 16px; } .panel-header p { color: #64748b; font-size: 13px; } .panel-header.compact { align-items: flex-start; }
  .metrics { display: grid; grid-template-columns: repeat(5, minmax(0, 1fr)); gap: 12px; } .metrics article { display: grid; gap: 8px; padding: 16px; } .metrics span { color: #64748b; font-size: 12px; font-weight: 800; text-transform: uppercase; } .metrics strong { font-size: 28px; line-height: 34px; }
  .layout-grid { display: grid; grid-template-columns: minmax(0, 1fr) 340px; gap: 20px; align-items: start; } .records-panel { min-width: 0; overflow: hidden; } .records-panel > .panel-header { padding: 16px; }
  .filters { display: grid; grid-template-columns: repeat(4, minmax(128px, 1fr)) auto auto; gap: 10px; align-items: end; border-top: 1px solid #f1f5f9; padding: 12px 16px 16px; } .check-filter { display: flex; min-height: 38px; align-items: center; gap: 8px; white-space: nowrap; } .check-filter input { width: 16px; min-height: 16px; }
  .table-wrap { overflow-x: auto; border-top: 1px solid #e2e8f0; } table { width: 100%; min-width: 1040px; border-collapse: collapse; } th, td { border-bottom: 1px solid #f1f5f9; padding: 10px 12px; text-align: left; vertical-align: middle; font-size: 13px; } th { background: #f8fafc; color: #64748b; font-size: 11px; font-weight: 800; text-transform: uppercase; }
  .service-cell { display: flex; align-items: center; gap: 10px; min-width: 220px; } .service-cell > span { display: inline-flex; width: 30px; height: 30px; flex: 0 0 auto; align-items: center; justify-content: center; border-radius: 6px; background: #f1f5f9; color: #334155; font-size: 12px; font-weight: 800; } .service-cell a, .service-cell strong { display: block; max-width: 250px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; } .service-cell a { color: #64748b; font-size: 12px; text-decoration: none; }
  .badge { display: inline-flex; align-items: center; border-radius: 999px; padding: 4px 8px; font-size: 12px; font-weight: 800; text-transform: capitalize; white-space: nowrap; } .neutral { background: #f1f5f9; color: #334155; } .risk-low, .status-active { background: #dcfce7; color: #166534; } .risk-medium, .status-needs_review { background: #fef3c7; color: #92400e; } .risk-high, .status-expired { background: #fee2e2; color: #991b1b; } .status-revoked { background: #e2e8f0; color: #475569; }
  .utility-column { display: grid; gap: 14px; } .utility-panel { display: grid; gap: 14px; padding: 16px; } .token-box { display: grid; gap: 8px; border-radius: 6px; background: #0f172a; color: #e2e8f0; padding: 12px; } .token-box span { color: #94a3b8; font-size: 12px; font-weight: 800; text-transform: uppercase; } code { overflow-wrap: anywhere; font-family: "JetBrains Mono", "SFMono-Regular", Consolas, monospace; font-size: 12px; }
  .device-list { display: grid; gap: 10px; margin: 0; padding: 0; list-style: none; } .device-list li { display: flex; align-items: center; justify-content: space-between; gap: 10px; border-top: 1px solid #f1f5f9; padding-top: 10px; } .device-list strong, .device-list span { display: block; } .device-list span, .small-empty { color: #64748b; font-size: 12px; }
  .empty-state { display: grid; justify-items: center; gap: 10px; padding: 48px 24px; text-align: center; } .empty-state p { max-width: 360px; color: #64748b; } .message { border-radius: 6px; padding: 8px 10px; font-size: 13px; font-weight: 700; } .error { background: #fee2e2; color: #991b1b; } .success { background: #dcfce7; color: #166534; }
  .modal-backdrop { position: fixed; inset: 0; display: grid; place-items: center; overflow: auto; background: rgb(15 23 42 / 42%); padding: 24px; z-index: 10; } .record-modal { display: grid; gap: 18px; width: min(760px, 100%); padding: 18px; box-shadow: 0 4px 6px -1px rgb(0 0 0 / 8%), 0 2px 4px -2px rgb(0 0 0 / 8%); } .modal-header { display: flex; align-items: flex-start; justify-content: space-between; gap: 16px; } .form-grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 12px; } .wide { grid-column: 1 / -1; } .modal-actions { justify-content: flex-end; }
  @media (max-width: 1120px) { .app-shell { grid-template-columns: 1fr; } .sidebar { position: static; height: auto; border-right: 0; border-bottom: 1px solid #e2e8f0; } nav { grid-template-columns: repeat(5, minmax(0, 1fr)); } .layout-grid { grid-template-columns: 1fr; } }
  @media (max-width: 760px) { .auth-shell, .metrics, .filters, .form-grid { grid-template-columns: 1fr; } .auth-shell { padding: 20px; } .auth-copy h1 { font-size: 30px; line-height: 38px; } .topbar, .page-heading, .panel-header, .topbar-actions, .heading-actions { align-items: stretch; flex-direction: column; } .topbar { padding: 16px; } .content { padding: 16px; } nav { grid-template-columns: repeat(2, minmax(0, 1fr)); } .user-chip strong, .user-chip small { max-width: 100%; } }
</style>
