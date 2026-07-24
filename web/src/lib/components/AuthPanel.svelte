<script lang="ts">
  import { authenticate } from '$lib/api';

  let mode: 'login' | 'register' = 'login';
  let email = '';
  let password = '';
  let displayName = '';
  let error = '';

  async function submit() {
    error = '';
    try {
      await authenticate(mode, { email, password, displayName });
    } catch (err) {
      error = err instanceof Error ? err.message : 'Authentication failed';
    }
  }
</script>

<main class="auth-shell">
  <section class="auth-copy">
    <p class="eyebrow">PermissionLedger</p>
    <h1>Privacy decisions, kept accountable.</h1>
    <p>Track consent records, app permissions, OAuth connections and review dates from one private dashboard.</p>
  </section>
  <section class="auth-panel panel" aria-label="Authentication">
    <div class="tabs">
      <button class:active={mode === 'login'} on:click={() => (mode = 'login')}>Login</button>
      <button class:active={mode === 'register'} on:click={() => (mode = 'register')}>Register</button>
    </div>
    {#if mode === 'register'}<label>Display name<input bind:value={displayName} autocomplete="name" /></label>{/if}
    <label>Email<input bind:value={email} autocomplete="email" type="email" /></label>
    <label>Password<input bind:value={password} autocomplete="current-password" type="password" /></label>
    <button class="primary full" on:click={submit}>{mode === 'login' ? 'Login' : 'Create account'}</button>
    {#if error}<p class="message error">{error}</p>{/if}
  </section>
</main>

<style>
  .auth-shell {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 420px;
    gap: 48px;
    min-height: 100vh;
    align-items: center;
    max-width: 1120px;
    margin: 0 auto;
    padding: 32px;
  }
  .auth-copy { display: grid; gap: 14px; }
  .auth-copy h1 { max-width: 620px; font-size: 40px; line-height: 48px; }
  .auth-copy p:last-child { max-width: 560px; color: #64748b; font-size: 16px; line-height: 26px; }
  .auth-panel { display: grid; gap: 14px; padding: 18px; }
  .tabs { display: grid; grid-template-columns: 1fr 1fr; gap: 8px; }
  .tabs button { background: #f8fafc; border: 1px solid #e2e8f0; color: #475569; }
  .tabs .active { background: #1e293b; color: #ffffff; }

  @media (max-width: 760px) {
    .auth-shell { grid-template-columns: 1fr; padding: 20px; }
    .auth-copy h1 { font-size: 30px; line-height: 38px; }
  }
</style>
