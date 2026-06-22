# PermissionLedger

Track, review and manage online consent decisions, app permissions and privacy policy changes from one secure dashboard.

## The problem

I give away digital permissions constantly: cookie banners, marketing sign-ups, app permissions, OAuth connections, privacy policy agreements, subscription terms. Each decision takes a few seconds and is then forgotten. There is no record of what I agreed to, when, or whether it still applies.

PermissionLedger is my answer to one question: what have I agreed to online, and do I still want that permission to remain active?

## What it does

I can create a record manually through the web dashboard, or capture one straight from the current website using the browser extension. Each record holds the service name, the website, the type and category of consent, the date given, a review date, a risk level, a status and notes. The full set of record fields, status values and risk levels is defined in [ROADMAP.md](./ROADMAP.md).

From the dashboard I can search, filter, review, update, revoke and export everything I have recorded.

## Project status

PermissionLedger is in planning. Nothing is built yet. The build plan, including what ships in each release, lives in [ROADMAP.md](./ROADMAP.md). The database schema and entity relationships live in [DOMAIN_MODEL.md](./DOMAIN_MODEL.md).

Release 1 covers consent and permission tracking: accounts, records, the dashboard, the browser extension and audit logging. Release 2 adds privacy policy monitoring, AI-assisted change ranking, SSO and email alerts. Release 3 is a stretch goal that explores helping users act on a service directly, things like consent withdrawal and account deletion requests.

## Tech stack

```text
Backend: Rust, Axum, Tokio, SQLx
Database: PostgreSQL
Authentication: secure cookie sessions for the web app, paired tokens for the extension
Password hashing: Argon2id
Frontend: SvelteKit, TypeScript, Tailwind CSS
Browser extension: Chrome Manifest V3, TypeScript, Svelte popup
DevOps: Docker, Docker Compose, GitHub Actions
```

A few notes on choices that aren't obvious from the list:

**Vite isn't a separate decision.** SvelteKit already uses Vite as its build tool, so there's nothing extra to set up there.

**Privacy policy checks happen once per service, not once per user.** If a thousand users each have a record against the same service, the policy is fetched and checked once a day, not a thousand times. Checks back off to weekly or monthly for policies that don't change, and reset to daily after one does. The full design, including the cost gates around the LLM call, is in [DOMAIN_MODEL.md](./DOMAIN_MODEL.md).

**The policy check job runs as a separate scheduled worker, not inside the API process.** Due-ness lives in the database: each policy carries its own `check_interval` and `last_checked_at`, so the worker only needs to query which policies are due now rather than evaluate cron expressions. It runs as its own deployable, separate from the request-serving backend, because the API is intended to scale to zero when idle on Azure and an in-process scheduler would never wake up on a quiet night. In production this is an always-on worker (minimum one replica) or an Azure Container Apps Job on a nightly schedule; locally it's a background task in the same Docker Compose stack.

**"AI-assisted" means an API call, not a model running inside Rust, and it only runs on confirmed changes.** A check first compares a hash of the extracted policy text against the last snapshot. Only when that hash has changed does the backend send the original and updated text to an LLM provider, with a system prompt asking it to identify what changed and categorise each change by type and risk level. The model never runs in-process, and it never runs on a check that found no change.

**SQLx checks queries against a real database at compile time.** This means local development and CI both need a running PostgreSQL instance with migrations applied before the backend will build. The Docker Compose setup handles this.

## Architecture

```text
        Browser Extension          SvelteKit Web Dashboard
                  \                        /
                   \                      /
                    ↓                    ↓
                  Rust Backend API (Axum)
                            ↓
                  PostgreSQL Database
                            ↑
                  Policy Check Worker (Release 2)
```

Both the extension and the dashboard talk to the Rust backend, which is the only request-serving component that touches PostgreSQL. The backend validates requests, handles authentication, manages extension pairing and exposes the API. PostgreSQL stores everything: users, services, consent records, sessions, extension devices, audit logs and exports. The dashboard is where I review, filter, revoke and export; the extension captures permission decisions while I'm browsing.

The policy check worker added in Release 2 is a separate scheduled component, not part of the request-serving API. It reads due policies from the database, fetches and diffs them, and writes back snapshots, changes and alerts. It's kept separate so the API can scale to zero when idle without the scheduled checks going dark.

## Running locally

This section will be filled in once Phase 1 of the roadmap is done and there's a working Docker Compose setup to document. For now, see [ROADMAP.md](./ROADMAP.md) for what Phase 1 covers. Local development runs entirely through Docker Compose; production is intended to run on Azure once Release 1 is deployable, though the Docker and GitHub Actions setup keeps that target open.

## Security

Passwords are hashed with Argon2id before storage. The web app uses secure, HTTP-only cookie sessions. The browser extension never sees the main account password: it pairs with the account and gets a limited token instead, which I can view and revoke from the dashboard at any time.

An audit log records important actions such as logins, record changes and exports. It never stores passwords, raw tokens or sensitive request bodies.

## Licence

Copyright © 2026 Mahamad Dahir. All rights reserved.

This repository is public for viewing and educational review only. The code may not be copied, modified, distributed, hosted, sold or used in production without prior written permission.
