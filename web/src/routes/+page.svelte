<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '$lib/api';
  import { formatDate, serviceInitial } from '$lib/records';
  import type { Dashboard } from '$lib/types';

  let dashboard: Dashboard | null = null;
  let error = '';

  $: summary = dashboard?.summary ?? { active: 0, review_due: 0, expired: 0, revoked: 0, high_risk: 0 };

  onMount(load);

  async function load() {
    try {
      dashboard = await api('/api/dashboard');
    } catch (err) {
      error = err instanceof Error ? err.message : 'Could not load the dashboard';
    }
  }
</script>

<section class="page-heading">
  <div><p class="eyebrow">Dashboard</p><h1>Privacy permissions</h1></div>
  <div class="heading-actions"><a class="secondary" href="/records">View all records</a></div>
</section>

{#if error}<p class="message error">{error}</p>{/if}

<section class="metrics" aria-label="Summary metrics">
  <article><span>Active permissions</span><strong>{summary.active}</strong></article>
  <article><span>Review due</span><strong>{summary.review_due}</strong></article>
  <article><span>Expired</span><strong>{summary.expired}</strong></article>
  <article><span>Revoked</span><strong>{summary.revoked}</strong></article>
  <article><span>High risk</span><strong>{summary.high_risk}</strong></article>
</section>

<section class="panel recent-panel">
  <div class="panel-header"><div><h2>Recently added</h2><p>The latest consent records you saved.</p></div></div>
  {#if dashboard?.recent.length}
    <ul class="recent-list">
      {#each dashboard.recent as record}
        <li>
          <div class="service-cell">
            <span>{serviceInitial(record.service_name)}</span>
            <div><strong>{record.service_name}</strong><small>{record.consent_type}</small></div>
          </div>
          <div class="recent-meta">
            <span class={`badge risk-${record.risk_level}`}>{record.risk_level}</span>
            <small>{formatDate(record.date_given)}</small>
          </div>
        </li>
      {/each}
    </ul>
  {:else}
    <div class="small-empty">No records yet. Add one from the Records page.</div>
  {/if}
</section>

<style>
  .metrics { display: grid; grid-template-columns: repeat(5, minmax(0, 1fr)); gap: 12px; }
  .metrics span { color: #64748b; font-size: 12px; font-weight: 800; text-transform: uppercase; }
  .metrics strong { font-size: 28px; line-height: 34px; }

  .recent-panel { padding: 16px; display: grid; gap: 14px; }
  .recent-list { display: grid; gap: 10px; margin: 0; padding: 0; list-style: none; }
  .recent-list li {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    border-top: 1px solid #f1f5f9;
    padding-top: 10px;
  }
  .service-cell { display: flex; align-items: center; gap: 10px; min-width: 0; }
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
  .service-cell strong, .service-cell small { display: block; }
  .service-cell small, .recent-meta small, .small-empty { color: #64748b; font-size: 12px; }
  .recent-meta { display: flex; align-items: center; gap: 10px; }

  @media (max-width: 760px) {
    .metrics { grid-template-columns: 1fr; }
  }
</style>
