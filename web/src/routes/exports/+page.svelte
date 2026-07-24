<script lang="ts">
  import { apiBase } from '$lib/api';

  const columns =
    'id, service_name, website_url, category, consent_type, date_given, review_date, expiry_date, status, risk_level, source, notes, created_at, updated_at';
</script>

<section class="page-heading">
  <div><p class="eyebrow">Exports</p><h1>Download your data</h1></div>
</section>

<section class="panel export-panel">
  <div class="panel-header compact"><div><h2>Export formats</h2><p>Every consent record on your account, nothing from anyone else.</p></div></div>

  <div class="export-actions">
    <a class="secondary" href={`${apiBase}/api/export.csv`}>Export CSV</a>
    <a class="secondary" href={`${apiBase}/api/export.json`}>Export JSON</a>
  </div>

  <div class="format-note">
    <h3>CSV columns</h3>
    <code>{columns}</code>
    <p>JSON exports return the same records under a <code>records</code> key. Every export is written to your audit log.</p>
  </div>
</section>

<style>
  .export-panel { display: grid; gap: 16px; padding: 16px; }
  .export-actions { display: flex; gap: 8px; flex-wrap: wrap; }
  .format-note { display: grid; gap: 8px; border-top: 1px solid #f1f5f9; padding-top: 14px; }
  .format-note h3 { font-size: 13px; color: #475569; }
  .format-note p { color: #64748b; font-size: 13px; }
</style>
