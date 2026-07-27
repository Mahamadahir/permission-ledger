<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { emptyRecordForm, type Category, type RecordForm } from '$lib/types';

  export let form: RecordForm;
  export let categories: Category[] = [];
  export let error = '';

  const dispatch = createEventDispatcher<{ save: void; close: void }>();

  $: canSave = form.service_name && form.website_url && form.consent_type && form.category_id;

  function clear() {
    form = { ...emptyRecordForm(), category_id: categories[0]?.id ?? '' };
  }
</script>

<div class="modal-backdrop" role="presentation">
  <form class="record-modal" on:submit|preventDefault={() => dispatch('save')}>
    <div class="modal-header">
      <div><p class="eyebrow">Consent record</p><h2>{form.id ? 'Edit record' : 'Add record'}</h2></div>
      <button class="icon-button" type="button" aria-label="Close" on:click={() => dispatch('close')}>CL</button>
    </div>
    <div class="form-grid">
      <label>Service name<input bind:value={form.service_name} /></label>
      <label>Website URL<input bind:value={form.website_url} placeholder="https://example.com" /></label>
      <label>Category<select bind:value={form.category_id}>{#each categories as category}<option value={category.id}>{category.name}</option>{/each}</select></label>
      <label>Consent type<input bind:value={form.consent_type} placeholder="Marketing emails, analytics cookies" /></label>
      <label>Date given<input bind:value={form.date_given} type="date" /></label>
      <label>Review date<input bind:value={form.review_date} type="date" /></label>
      <label>Expiry date<input bind:value={form.expiry_date} type="date" /></label>
      <label>Risk level<select bind:value={form.risk_level}><option value="low">Low</option><option value="medium">Medium</option><option value="high">High</option></select></label>
      <label>Status<select bind:value={form.status}><option value="active">Active</option><option value="needs_review">Review due</option><option value="expired">Expired</option><option value="revoked">Revoked</option></select></label>
      <label class="wide">Notes<textarea bind:value={form.notes}></textarea></label>
    </div>
    <div class="modal-actions">
      <button class="secondary" type="button" on:click={clear}>Clear</button>
      <button class="primary" disabled={!canSave}>Save record</button>
    </div>
    {#if error}<p class="message error">{error}</p>{/if}
  </form>
</div>

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    display: grid;
    place-items: center;
    overflow: auto;
    background: rgb(15 23 42 / 42%);
    padding: 24px;
    z-index: 10;
  }
  .record-modal {
    display: grid;
    gap: 18px;
    width: min(760px, 100%);
    padding: 18px;
    box-shadow: 0 4px 6px -1px rgb(0 0 0 / 8%), 0 2px 4px -2px rgb(0 0 0 / 8%);
  }
  .modal-header { display: flex; align-items: flex-start; justify-content: space-between; gap: 16px; }
  .form-grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 12px; }
  .wide { grid-column: 1 / -1; }
  .modal-actions { justify-content: flex-end; }

  @media (max-width: 760px) {
    .form-grid { grid-template-columns: 1fr; }
  }
</style>
