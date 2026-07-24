<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '$lib/api';
  import { formatDate } from '$lib/records';
  import type { Device } from '$lib/types';

  let devices: Device[] = [];
  let deviceName = 'Chrome extension';
  let pairedToken = '';
  let error = '';

  onMount(load);

  async function load() {
    devices = await api('/api/extension/devices');
  }

  async function pairExtension() {
    error = '';
    try {
      const data = await api('/api/extension/pair', {
        method: 'POST',
        body: JSON.stringify({ device_name: deviceName })
      });
      pairedToken = data.token;
      await load();
    } catch (err) {
      error = err instanceof Error ? err.message : 'Could not pair the extension';
    }
  }

  async function revokeDevice(id: string) {
    await api(`/api/extension/devices/${id}`, { method: 'DELETE' });
    pairedToken = '';
    await load();
  }
</script>

<section class="page-heading">
  <div><p class="eyebrow">Extension</p><h1>Browser extension</h1></div>
</section>

{#if error}<p class="message error">{error}</p>{/if}

<div class="extension-grid">
  <section class="panel utility-panel">
    <div class="panel-header compact"><div><h2>Extension pairing</h2><p>Pair a browser device with a limited token.</p></div></div>
    <label>Device name<input bind:value={deviceName} /></label>
    <button class="primary full" on:click={pairExtension}>Pair extension</button>
    {#if pairedToken}
      <div class="token-box">
        <span>New token</span>
        <code>{pairedToken}</code>
        <small>Copy it now. It is shown once and never stored in full.</small>
      </div>
    {/if}
  </section>

  <section class="panel utility-panel">
    <div class="panel-header compact"><div><h2>Paired devices</h2><p>{devices.length} device{devices.length === 1 ? '' : 's'}</p></div></div>
    {#if devices.length}
      <ul class="device-list">
        {#each devices as device}
          <li>
            <div>
              <strong>{device.name}</strong>
              <span>{device.last_used_at ? `Last used ${formatDate(device.last_used_at)}` : 'Never used'}</span>
            </div>
            <button class="ghost danger-text" on:click={() => revokeDevice(device.id)}>Revoke</button>
          </li>
        {/each}
      </ul>
    {:else}
      <div class="small-empty">No paired devices.</div>
    {/if}
  </section>
</div>

<style>
  .extension-grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 14px; align-items: start; }
  .utility-panel { display: grid; gap: 14px; padding: 16px; }
  .token-box { display: grid; gap: 8px; border-radius: 6px; background: #0f172a; color: #e2e8f0; padding: 12px; }
  .token-box span { color: #94a3b8; font-size: 12px; font-weight: 800; text-transform: uppercase; }
  .token-box small { color: #94a3b8; font-size: 12px; }

  .device-list { display: grid; gap: 10px; margin: 0; padding: 0; list-style: none; }
  .device-list li {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    border-top: 1px solid #f1f5f9;
    padding-top: 10px;
  }
  .device-list strong, .device-list span { display: block; }
  .device-list span, .small-empty { color: #64748b; font-size: 12px; }

  @media (max-width: 1120px) {
    .extension-grid { grid-template-columns: 1fr; }
  }
</style>
