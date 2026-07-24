#!/usr/bin/env node
// Create a demo account with enough spread of records that every dashboard
// metric and filter has something to show.
//
//   node scripts/seed.mjs [baseUrl]
//
// Defaults to http://localhost:5173 (the dev stack). Pass the production
// origin to seed that instead.

const baseUrl = (process.argv[2] ?? process.env.SEED_BASE_URL ?? 'http://localhost:5173').replace(/\/$/, '');
const email = process.env.SEED_EMAIL ?? 'demo@permissionledger.test';
const password = process.env.SEED_PASSWORD ?? 'demopassword123';

let cookie = '';
let csrfToken = '';

async function call(path, options = {}) {
  const headers = { ...(options.headers ?? {}) };
  if (options.body) headers['content-type'] = 'application/json';
  if (cookie) headers.cookie = cookie;
  if (csrfToken) headers['x-csrf-token'] = csrfToken;

  const response = await fetch(`${baseUrl}${path}`, { ...options, headers });
  // Node's fetch has no cookie jar, so carry the session by hand.
  const setCookie = response.headers.getSetCookie?.() ?? [];
  if (setCookie.length) {
    cookie = setCookie.map((c) => c.split(';')[0]).join('; ');
  }
  if (!response.ok) {
    const detail = await response.text();
    throw new Error(`${options.method ?? 'GET'} ${path} failed: ${response.status} ${detail}`);
  }
  return response.status === 204 ? null : response.json();
}

function daysFromNow(days) {
  const date = new Date();
  date.setDate(date.getDate() + days);
  return date.toISOString().slice(0, 10);
}

async function signIn() {
  try {
    const created = await call('/api/auth/register', {
      method: 'POST',
      body: JSON.stringify({ email, password, display_name: 'Demo User' })
    });
    csrfToken = created.csrf_token;
    console.log(`Registered ${email}`);
  } catch {
    cookie = '';
    const session = await call('/api/auth/login', {
      method: 'POST',
      body: JSON.stringify({ email, password })
    });
    csrfToken = session.csrf_token;
    console.log(`Signed in as ${email} (account already existed)`);
  }
}

const records = [
  ['Stripe', 'https://stripe.com', 'financial-services', 'Payment processing', 'high', 'active', 180],
  ['Google Analytics', 'https://analytics.google.com', 'cookies', 'Analytics cookies', 'medium', 'active', 90],
  ['Mailchimp', 'https://mailchimp.com', 'marketing', 'Marketing emails', 'low', 'active', 365],
  ['LinkedIn', 'https://linkedin.com', 'employment-platforms', 'Profile data sharing', 'medium', 'needs_review', -14],
  ['Meta Pixel', 'https://facebook.com', 'data-sharing', 'Ad tracking', 'high', 'needs_review', -30],
  ['Dropbox', 'https://dropbox.com', 'app-permissions', 'File access', 'medium', 'expired', -60],
  ['Old Newsletter', 'https://example.com', 'marketing', 'Newsletter subscription', 'low', 'revoked', -120],
  ['GitHub', 'https://github.com', 'oauth-permissions', 'OAuth repository access', 'high', 'active', 45],
  ['Duolingo', 'https://duolingo.com', 'education-platforms', 'Progress tracking', 'low', 'active', 200]
];

async function seed() {
  await signIn();

  const categories = await call('/api/categories');
  const bySlug = Object.fromEntries(categories.map((c) => [c.slug, c.id]));

  for (const [name, url, slug, consentType, risk, status, reviewInDays] of records) {
    await call('/api/records', {
      method: 'POST',
      body: JSON.stringify({
        service_name: name,
        website_url: url,
        consent_type: consentType,
        category_id: bySlug[slug] ?? categories[0].id,
        date_given: daysFromNow(-Math.abs(reviewInDays) - 30),
        review_date: daysFromNow(reviewInDays),
        expiry_date: status === 'expired' ? daysFromNow(-1) : null,
        status,
        risk_level: risk,
        notes: null
      })
    });
    console.log(`  + ${name} (${status}, ${risk} risk)`);
  }

  await call('/api/extension/pair', {
    method: 'POST',
    body: JSON.stringify({ device_name: 'Demo laptop' })
  });
  console.log('  + paired a demo extension device');

  console.log(`\nSeeded ${records.length} records at ${baseUrl}. Sign in with ${email} / ${password}`);
}

seed().catch((error) => {
  console.error(error.message);
  process.exit(1);
});
