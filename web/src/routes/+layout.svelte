<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import '../app.css';
  import { loadSession } from '$lib/api';
  import AppShell from '$lib/components/AppShell.svelte';
  import AuthPanel from '$lib/components/AuthPanel.svelte';
  import { session } from '$lib/session';

  let ready = false;

  onMount(async () => {
    await loadSession();
    ready = true;
  });
</script>

<svelte:head><title>PermissionLedger Privacy Dashboard</title></svelte:head>

{#if !ready}
  <div class="booting"></div>
{:else if !$session.user}
  <AuthPanel />
{:else}
  <AppShell pathname={$page.url.pathname}><slot /></AppShell>
{/if}

<style>
  .booting { min-height: 100vh; }
</style>
