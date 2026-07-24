<script lang="ts">
  import { logout } from '$lib/api';
  import { serviceInitial } from '$lib/records';
  import { session } from '$lib/session';

  /** Current pathname, passed in so this component stays free of $app/stores. */
  export let pathname = '/';

  const navigation = [
    { label: 'Dashboard', href: '/' },
    { label: 'Records', href: '/records' },
    { label: 'Extension', href: '/extension' },
    { label: 'Exports', href: '/exports' },
    { label: 'Settings', href: '/settings' }
  ];

  $: user = $session.user;
</script>

<main class="app-shell">
  <aside class="sidebar" aria-label="Primary">
    <div class="brand">
      <span class="brand-mark">PL</span>
      <div><strong>PermissionLedger</strong><span>Privacy Dashboard</span></div>
    </div>
    <nav>
      {#each navigation as item}
        <a class:active={pathname === item.href} href={item.href}>{item.label}</a>
      {/each}
    </nav>
  </aside>

  <section class="dashboard">
    <header class="topbar">
      <div class="topbar-actions">
        {#if user}
          <div class="user-chip">
            <span>{serviceInitial(user.email)}</span>
            <div><strong>{user.display_name ?? user.email}</strong><small>{user.email}</small></div>
          </div>
        {/if}
        <button class="secondary" on:click={logout}>Log out</button>
      </div>
    </header>

    <div class="content"><slot /></div>
  </section>
</main>

<style>
  .app-shell { display: grid; grid-template-columns: 260px minmax(0, 1fr); min-height: 100vh; }
  .sidebar {
    position: sticky;
    top: 0;
    height: 100vh;
    border-right: 1px solid #e2e8f0;
    background: #ffffff;
    padding: 20px 16px;
  }
  .brand { display: flex; gap: 12px; align-items: center; padding: 0 4px 24px; }
  .brand-mark {
    display: inline-flex;
    width: 38px;
    height: 38px;
    align-items: center;
    justify-content: center;
    border-radius: 8px;
    background: #2563eb;
    color: #ffffff;
    font-size: 13px;
    font-weight: 800;
  }
  .brand strong, .brand span { display: block; }
  .brand span:not(.brand-mark) { color: #64748b; font-size: 12px; }

  nav { display: grid; gap: 4px; }
  nav a {
    border-radius: 6px;
    color: #475569;
    padding: 10px 12px;
    text-decoration: none;
    font-size: 14px;
    font-weight: 700;
  }
  nav a.active { background: #eff6ff; color: #1d4ed8; }

  .dashboard { min-width: 0; }
  .topbar {
    display: flex;
    min-height: 68px;
    align-items: center;
    justify-content: flex-end;
    gap: 16px;
    border-bottom: 1px solid #e2e8f0;
    background: #ffffff;
    padding: 12px 24px;
  }
  .topbar-actions { display: flex; align-items: center; gap: 8px; }
  .user-chip { display: flex; align-items: center; gap: 10px; min-width: 0; }
  .user-chip > span {
    display: inline-flex;
    width: 34px;
    height: 34px;
    align-items: center;
    justify-content: center;
    border-radius: 50%;
    background: #e0f2fe;
    color: #0369a1;
    font-size: 12px;
    font-weight: 800;
  }
  .user-chip strong, .user-chip small {
    display: block;
    max-width: 220px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .user-chip small { color: #64748b; }

  .content { display: grid; gap: 20px; max-width: 1440px; margin: 0 auto; padding: 24px; }

  @media (max-width: 1120px) {
    .app-shell { grid-template-columns: 1fr; }
    .sidebar { position: static; height: auto; border-right: 0; border-bottom: 1px solid #e2e8f0; }
    nav { grid-template-columns: repeat(5, minmax(0, 1fr)); }
  }
  @media (max-width: 760px) {
    .topbar, .topbar-actions { align-items: stretch; flex-direction: column; padding: 16px; }
    .content { padding: 16px; }
    nav { grid-template-columns: repeat(2, minmax(0, 1fr)); }
    .user-chip strong, .user-chip small { max-width: 100%; }
  }
</style>
