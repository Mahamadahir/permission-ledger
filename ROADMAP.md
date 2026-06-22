# Roadmap

This is the build plan for PermissionLedger, split into three releases and fifteen phases.

## Release 1: consent and permission tracking

This is the first complete version of the app.

**Accounts.** Registration, login, Argon2id password hashing, secure cookie sessions, logout, account settings, login rate limiting. Two-factor authentication is out of scope for Release 1.

**Consent records.** Each record can hold a service name, website URL, consent type, category, date given, review date, expiry date if known, status, risk level, notes and a source (manual entry or browser extension).

Status values: `active`, `needs_review`, `expired`, `revoked`.

Risk levels: `low`, `medium`, `high`.

**Categories.** Default categories ship with the app: cookies, marketing, data sharing, app permissions, OAuth permissions, privacy policy, subscription terms, financial services, health data, education platforms, employment platforms, social media, other. Custom categories are a Release 2 feature.

**Dashboard.** Shows total active records, records due for review, expired records, revoked records, high-risk records, recently added records, the most common categories and the services with the most records.

**Browser extension.** Chrome only, Manifest V3, to start. The user clicks the extension icon, the extension reads the current URL and page title, and the user fills in the consent type, category, risk level and review date before saving. Automatic consent banner detection comes later, once manual capture works properly.

**Extension pairing.** The extension never holds the main account password. Pairing creates a limited token tied to that device, which the user can view and revoke from the dashboard.

**Search and filtering.** By service name, website, consent type, category, status, risk level, date given, review date, expiry date and source. Useful saved filters: high-risk, due for review, expired, revoked, added by the extension.

**Export.** CSV and JSON. PDF export is a Release 2 feature.

**Audit log.** Records events such as `user.login`, `user.logout`, `consent.created`, `consent.updated`, `consent.revoked`, `consent.deleted`, `export.created`, `extension.paired` and `extension.revoked`. It never stores passwords, raw tokens or sensitive request data.

### Release 1 scope

- Rust Axum backend
- PostgreSQL database with SQLx migrations
- SvelteKit dashboard
- User registration and login with secure cookie sessions
- Consent record creation, editing, revocation and deletion
- Consent categories and service records
- Dashboard summary
- Search and filtering
- Chrome extension popup with manual capture
- Extension pairing and device revocation
- Audit logging
- CSV and JSON export
- Docker Compose setup
- Basic tests
- README screenshots

## Release 2: privacy policy monitoring

When a record has a privacy policy URL attached, PermissionLedger monitors it and flags meaningful changes. Checking happens once per service, not once per user, and the check frequency backs off for policies that don't change. The full design, including the dedup and polling approach, is in [DOMAIN_MODEL.md](./DOMAIN_MODEL.md).

**Monitoring.** Each tracked policy is checked on its own schedule rather than a flat daily run for everyone: active or recently changed policies get checked daily, stable ones back off to weekly or monthly. A check compares a hash of the extracted text against the last snapshot, so most checks cost a fetch and a hash comparison, nothing more.

**Ranking.** Only a confirmed text change triggers the expensive step: the backend sends the original and updated text to an LLM with a system prompt asking it to identify what changed and assign a category and a risk rating. Each change gets a risk rating: `low`, `medium`, `high` or `critical`. A formatting change is low. A new clause about sharing data with advertising partners is high. Anything touching health data, financial data, location data or children's data is critical.

The summary looks specifically for new data collection wording, new third-party sharing terms, retention period changes, changes to user rights, changes to marketing permissions, changes to tracking or cookies, changes to international transfers, changes to the contact details or legal entity, and changes to deletion or withdrawal processes.

**Critical alerts.** A confirmed change fans out to every user with a record against that service. Critical changes trigger an email per affected user. Everything else shows up on the dashboard only, so the inbox isn't overwhelmed with low-stakes formatting updates.

**SSO.** Google and Microsoft sign-in, alongside the existing password login rather than replacing it.

### Release 2 scope

- Privacy policy URL storage, tracked once per service rather than per user
- Tiered checking with backoff for stable policies
- Policy snapshots and hash-based change detection
- AI-assisted change summaries and risk ranking, triggered only on confirmed changes
- Critical change email alerts
- Google and Microsoft SSO
- PDF export
- Custom categories
- CSV/JSON import
- Automatic consent banner detection

## Release 3: service relationship actions

This release is a stretch goal. Every service handles account deletion, data requests and privacy settings differently: some have clear APIs, most don't. PermissionLedger won't pretend to delete an account automatically on the user's behalf. The realistic goal is to help the user start, record and track that process from one place.

**Action tracking.** Actions link to a service or a consent record. Types include `withdraw_consent`, `delete_account`, `delete_data`, `export_data`, `unsubscribe_marketing` and `review_settings`. Each has a status: `draft`, `sent`, `in_progress`, `completed`, `rejected` or `cancelled`. The user can add notes, attach evidence and log the date they sent the request.

**Request templates.** Plain-language templates for common requests, such as account deletion or withdrawing marketing consent, that the user can copy, edit and send themselves.

**Guided offboarding.** Manually saved links to a service's account deletion page, privacy settings, marketing preferences and support contact. A curated directory of common services is a later addition, not part of this release.

### Release 3 scope

- Service action tracking with statuses and history
- Request templates
- Evidence upload
- Service privacy settings and contact links
- Follow-up reminders
- Audit logging for service actions

## Phased build plan

These dates assume roughly 10 hours a week (evenings and weekends) starting late June 2026, with slack built in for holidays, busy work weeks and the ongoing job search. They are targets, not commitments: the point is a realistic order and pace, not a contract. Release milestones matter more than individual phase dates. The heavier phases (the extension, the policy-monitoring pipeline, SSO) are deliberately given more room because they introduce new surfaces rather than extending existing ones.

Milestones at this pace:
- **Release 1 (consent and permission tracking): target early February 2027** (end of Phase 11)
- **Release 2 (privacy policy monitoring, SSO, alerts): target mid-April 2027** (end of Phase 13)
- **Release 3 (service relationship actions): target end of May 2027** (end of Phase 15)


### Phase 1: project foundation

*Target: 23 Jun – 6 Jul 2026*

Set up the monorepo, the Rust Axum backend, the SvelteKit frontend and PostgreSQL. Add SQLx migrations, Docker Compose, environment variable configuration, a health check endpoint and the initial README.

Outcome: backend, frontend and database run locally through Docker Compose.

### Phase 2: authentication and account system

*Target: 7 Jul – 27 Jul 2026*

Users table, registration, login, Argon2id hashing, secure HTTP-only cookie sessions, logout, an authenticated user endpoint, account settings, login rate limiting.

Outcome: a user can register, log in, stay authenticated and log out securely.

### Phase 3: core consent record system

*Target: 28 Jul – 24 Aug 2026*

Services, consent categories and consent records tables. Endpoints to create, view, edit, revoke and delete records. Default categories seeded on setup.

Outcome: a logged-in user can manage consent records end to end.

### Phase 4: web dashboard

*Target: 25 Aug – 14 Sep 2026*

The dashboard page itself: active, expired, review-due, high-risk and revoked counts, a record table, create and edit forms, revoke and delete actions.

Outcome: a user can manage their records through the dashboard.

### Phase 5: search, filtering and review workflow

*Target: 15 Sep – 28 Sep 2026*

Search by service name, filters for website, category, risk level, status and review date. A needs-review workflow, in-dashboard review reminders, sorting by date, risk and status.

Outcome: high-risk, expired, revoked and review-due records are easy to find.

### Phase 6: browser extension MVP

*Target: 29 Sep – 2 Nov 2026*

The Chrome Manifest V3 extension and its popup UI. Captures the current URL and page title, lets the user set consent type, category, risk level and a note, and sends the record to the backend with a confirmation on save.

Outcome: a user can save a consent decision from the current website without opening the dashboard.

### Phase 7: extension pairing and device management

*Target: 3 Nov – 16 Nov 2026*

Extension devices table, the pairing flow, a limited access token generated for each device, only a hash of that token stored, a view of paired extensions and the ability to revoke them. Audit logs for pairing and revocation.

Outcome: the extension connects to an account without ever holding the main login.

### Phase 8: audit logging and security hardening

*Target: 17 Nov – 14 Dec 2026*

Audit logs table covering login, logout, record changes and exports. Input validation, CSRF protection for cookie sessions, CORS configuration, safer error responses, a review of extension permissions, encryption for sensitive fields where needed, and production security settings.

Outcome: a clear security layer with a record of important actions.

### Phase 9: export and portability

*Target: 15 Dec 2026 – 4 Jan 2027*

Export endpoint, CSV and JSON formats, an export button in the dashboard, export actions logged to the audit log, and documentation of the export format.

Outcome: a user can download their consent history.

### Phase 10: Release 1 testing

*Target: 5 Jan – 25 Jan 2027*

Backend unit and integration tests covering authentication, consent record permissions and extension token permissions. Tests for search, filtering and export output. Frontend form tests, manual testing of the extension, and a clean-install test of the Docker Compose setup.

Outcome: Release 1 is stable enough to deploy.

### Phase 11: Release 1 deployment

*Target: 26 Jan – 8 Feb 2027*

Production Dockerfile, production environment documentation, a GitHub Actions workflow, a migration workflow, deployment instructions, seed data for demos, and screenshots for the README.

Outcome: Release 1 can be deployed and shown as a working application.

### Phase 12: Release 2 privacy policy monitoring

*Target: 9 Feb – 15 Mar 2027*

Privacy policy URLs tracked per service rather than per user, so the system checks each policy once regardless of how many users have a record against it. Snapshot storage with hash-based change detection, so a check only proceeds to a full diff when the text has actually changed. Tiered check intervals that back off for stable policies and reset to daily after a change. Change summaries and risk ranking, generated only when a change is confirmed. Alerts fanned out to every affected user, and a view of previous versions.

Outcome: users can see when a policy changes and understand what it means for their data, and the checking cost stays proportional to the number of distinct services tracked, not the number of users.

Scheduling and deployment note: the policy check must run as a separate scheduled component, not as an in-process loop inside the request-serving API. The API is intended to scale to zero when idle on Azure Container Apps, and a scaled-to-zero app has no process alive to wake up and run a scheduled check, so an in-process scheduler would silently skip checks on quiet nights, exactly when monitoring still needs to run. Deploy the checker either as an Azure Container Apps Job on a nightly cron schedule (Jobs are built for run-then-stop scheduled work and are the better fit here) or as a separate always-on worker with a minimum of one replica. The worker reads due policies via `next_check_at` / `check_interval`, processes them and writes back snapshots, changes and alerts. The LLM call uses the Batch API, since the nightly schedule has no latency requirement.

### Phase 13: Release 2 notifications and SSO

*Target: 16 Mar – 12 Apr 2027*

Email notification settings, critical change alerts, Google and Microsoft SSO, linking SSO identities to existing accounts, password login kept available, SSO logins recorded in the audit log.

Outcome: SSO login and critical email alerts both work.

### Phase 14: Release 3 service relationship actions

*Target: 13 Apr – 10 May 2027*

Service action records linked to services and consent records, action types and statuses, request templates, evidence upload, service contact methods, settings links, follow-up reminders, and audit logging for actions.

Outcome: users can track what they've done after reviewing a service, particularly after a risky policy change.

### Phase 15: Release 3 guided offboarding

*Target: 11 May – 31 May 2027*

Saved privacy settings and account deletion links per service, saved support contact details, manual notes for service-specific steps, reminders when a service hasn't responded, and an action history on the service detail page.

Outcome: users can manage the process of leaving a service or withdrawing consent without losing track of where they are.
