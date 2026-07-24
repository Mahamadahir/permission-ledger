<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '$lib/api';
  import RecordModal from '$lib/components/RecordModal.svelte';
  import { buildRecordQuery, formatDate, serviceInitial } from '$lib/records';
  import { emptyRecordForm, type Category, type RecordItem } from '$lib/types';

  let categories: Category[] = [];
  let records: RecordItem[] = [];
  let error = '';
  let notice = '';

  let search = '';
  let categoryFilter = '';
  let statusFilter = '';
  let riskFilter = '';
  let sourceFilter = '';
  let reviewDueOnly = false;

  let showEditor = false;
  let form = emptyRecordForm();

  onMount(async () => {
    categories = await api('/api/categories');
    await load();
  });

  async function load() {
    const query = buildRecordQuery({ search, categoryFilter, statusFilter, riskFilter, sourceFilter, reviewDueOnly });
    records = await api(`/api/records?${query}`);
    if (!form.category_id && categories[0]) form.category_id = categories[0].id;
  }

  function resetForm() {
    form = { ...emptyRecordForm(), category_id: categories[0]?.id ?? '' };
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

  async function saveRecord() {
    error = '';
    const payload = {
      ...form,
      review_date: form.review_date || null,
      expiry_date: form.expiry_date || null,
      notes: form.notes || null
    };
    try {
      const path = form.id ? `/api/records/${form.id}` : '/api/records';
      await api(path, { method: form.id ? 'PUT' : 'POST', body: JSON.stringify(payload) });
      resetForm();
      showEditor = false;
      await load();
      notice = 'Record saved';
    } catch (err) {
      error = err instanceof Error ? err.message : 'Could not save the record';
    }
  }

  async function revokeRecord(id: string) {
    await api(`/api/records/${id}/revoke`, { method: 'POST' });
    await load();
  }

  async function deleteRecord(id: string) {
    await api(`/api/records/${id}`, { method: 'DELETE' });
    await load();
  }

  function clearFilters() {
    search = '';
    categoryFilter = '';
    statusFilter = '';
    riskFilter = '';
    sourceFilter = '';
    reviewDueOnly = false;
    load();
  }
</script>

<section class="page-heading">
  <div><p class="eyebrow">Records</p><h1>Consent records</h1></div>
  <div class="heading-actions"><button class="primary" on:click={startNewRecord}>Add record</button></div>
</section>

<section class="panel records-panel">
  <div class="panel-header">
    <div><h2>Consent records</h2><p>{records.length} visible record{records.length === 1 ? '' : 's'}</p></div>
    {#if notice}<p class="message success">{notice}</p>{/if}
  </div>

  <div class="filters" aria-label="Record filters">
    <label class="search-field">Search<input bind:value={search} on:change={load} placeholder="Search services, websites, consent types" /></label>
    <label>Category<select bind:value={categoryFilter} on:change={load}><option value="">All categories</option>{#each categories as category}<option value={category.id}>{category.name}</option>{/each}</select></label>
    <label>Status<select bind:value={statusFilter} on:change={load}><option value="">All statuses</option><option value="active">Active</option><option value="needs_review">Review due</option><option value="expired">Expired</option><option value="revoked">Revoked</option></select></label>
    <label>Risk<select bind:value={riskFilter} on:change={load}><option value="">All risk</option><option value="low">Low</option><option value="medium">Medium</option><option value="high">High</option></select></label>
    <label>Source<select bind:value={sourceFilter} on:change={load}><option value="">All sources</option><option value="manual">Manual</option><option value="extension">Extension</option></select></label>
    <label class="check-filter"><input bind:checked={reviewDueOnly} on:change={load} type="checkbox" />Review due</label>
    <button class="ghost" on:click={clearFilters}>Clear</button>
  </div>

  <div class="table-wrap">
    {#if records.length}
      <table>
        <thead><tr><th>Service</th><th>Category</th><th>Consent type</th><th>Date given</th><th>Review date</th><th>Risk</th><th>Status</th><th>Source</th><th>Actions</th></tr></thead>
        <tbody>
          {#each records as record}
            <tr>
              <td><div class="service-cell"><span>{serviceInitial(record.service_name)}</span><div><strong>{record.service_name}</strong><a href={record.website_url}>{record.website_url}</a></div></div></td>
              <td><span class="badge neutral">{record.category_name}</span></td>
              <td>{record.consent_type}</td>
              <td>{formatDate(record.date_given)}</td>
              <td>{formatDate(record.review_date)}</td>
              <td><span class={`badge risk-${record.risk_level}`}>{record.risk_level}</span></td>
              <td><span class={`badge status-${record.status}`}>{record.status.replace('_', ' ')}</span></td>
              <td>{record.source}</td>
              <td>
                <div class="row-actions">
                  <button class="icon-button" title="Edit" on:click={() => editRecord(record)}>ED</button>
                  <button class="icon-button" title="Revoke" on:click={() => revokeRecord(record.id)}>RV</button>
                  <button class="icon-button danger-text" title="Delete" on:click={() => deleteRecord(record.id)}>DL</button>
                </div>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    {:else}
      <div class="empty-state">
        <h3>No records yet</h3>
        <p>Add the first consent or permission record to start tracking review dates.</p>
        <button class="primary" on:click={startNewRecord}>Add record</button>
      </div>
    {/if}
  </div>
</section>

{#if showEditor}
  <RecordModal bind:form {categories} {error} on:save={saveRecord} on:close={() => (showEditor = false)} />
{/if}

<style>
  .records-panel { min-width: 0; overflow: hidden; }
  .records-panel > .panel-header { padding: 16px; }
  .filters {
    display: grid;
    grid-template-columns: repeat(4, minmax(128px, 1fr)) auto auto;
    gap: 10px;
    align-items: end;
    border-top: 1px solid #f1f5f9;
    padding: 12px 16px 16px;
  }
  .search-field { grid-column: 1 / -1; }
  .check-filter { display: flex; min-height: 38px; align-items: center; gap: 8px; white-space: nowrap; }
  .check-filter input { width: 16px; min-height: 16px; }

  .table-wrap { overflow-x: auto; border-top: 1px solid #e2e8f0; }
  table { width: 100%; min-width: 1040px; border-collapse: collapse; }
  th, td { border-bottom: 1px solid #f1f5f9; padding: 10px 12px; text-align: left; vertical-align: middle; font-size: 13px; }
  th { background: #f8fafc; color: #64748b; font-size: 11px; font-weight: 800; text-transform: uppercase; }

  .service-cell { display: flex; align-items: center; gap: 10px; min-width: 220px; }
  .service-cell > span {
    display: inline-flex;
    width: 30px;
    height: 30px;
    flex: 0 0 auto;
    align-items: center;
    justify-content: center;
    border-radius: 6px;
    background: #f1f5f9;
    color: #334155;
    font-size: 12px;
    font-weight: 800;
  }
  .service-cell a, .service-cell strong {
    display: block;
    max-width: 250px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .service-cell a { color: #64748b; font-size: 12px; text-decoration: none; }

  .empty-state { display: grid; justify-items: center; gap: 10px; padding: 48px 24px; text-align: center; }
  .empty-state p { max-width: 360px; color: #64748b; }

  @media (max-width: 760px) {
    .filters { grid-template-columns: 1fr; }
  }
</style>
