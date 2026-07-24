<script lang="ts">
  import { onMount } from 'svelte';
  import { api, loadSession } from '$lib/api';
  import type { Settings } from '$lib/types';

  let displayName = '';
  let timezone = 'UTC';
  let reviewReminderDays = 30;
  let error = '';
  let notice = '';
  let loaded = false;

  onMount(async () => {
    try {
      const settings: Settings = await api('/api/auth/settings');
      applySettings(settings);
    } catch (err) {
      error = err instanceof Error ? err.message : 'Could not load settings';
    } finally {
      loaded = true;
    }
  });

  function applySettings(settings: Settings) {
    displayName = settings.display_name ?? '';
    timezone = settings.timezone;
    reviewReminderDays = settings.review_reminder_days;
  }

  async function save() {
    error = '';
    notice = '';
    try {
      const settings: Settings = await api('/api/auth/settings', {
        method: 'PUT',
        body: JSON.stringify({
          display_name: displayName || null,
          timezone,
          review_reminder_days: reviewReminderDays
        })
      });
      // The server clamps the reminder window, so take back what it stored.
      applySettings(settings);
      await loadSession();
      notice = 'Settings saved';
    } catch (err) {
      error = err instanceof Error ? err.message : 'Could not save settings';
    }
  }
</script>

<section class="page-heading">
  <div><p class="eyebrow">Settings</p><h1>Account settings</h1></div>
</section>

<section class="panel settings-panel">
  <div class="panel-header compact"><div><h2>Profile</h2><p>How your account appears and when reviews are due.</p></div></div>

  {#if loaded}
    <form class="settings-form" on:submit|preventDefault={save}>
      <label>Display name<input bind:value={displayName} autocomplete="name" /></label>
      <label>Timezone<input bind:value={timezone} placeholder="UTC" /></label>
      <label>
        Review reminder days
        <input bind:value={reviewReminderDays} type="number" min="1" max="365" />
        <small>How many days ahead a record counts as due for review. Between 1 and 365.</small>
      </label>
      <div class="settings-actions"><button class="primary">Save settings</button></div>
    </form>
  {/if}

  {#if notice}<p class="message success">{notice}</p>{/if}
  {#if error}<p class="message error">{error}</p>{/if}
</section>

<style>
  .settings-panel { display: grid; gap: 16px; padding: 16px; }
  .settings-form { display: grid; gap: 14px; max-width: 420px; }
  .settings-form small { color: #64748b; font-size: 12px; font-weight: 400; }
  .settings-actions { display: flex; justify-content: flex-start; }
</style>
