# API

Base URL: `http://localhost:3000` in local development.

## Authentication

- `POST /api/auth/register` creates an account and sets `pl_session` and `pl_csrf` cookies.
- `POST /api/auth/login` starts a session.
- `POST /api/auth/logout` ends the current session.
- `GET /api/auth/me` returns the current user and CSRF token.
- Cookie-authenticated mutating requests must include `x-csrf-token`.

## Consent Records

- `GET /api/categories` lists default Release 1 categories.
- `GET /api/dashboard` returns summary counts and recent records.
- `GET /api/records` supports `q`, `category_id`, `status`, `risk_level`, `source`, `review_due`, `expired`, and `sort`.
- `POST /api/records` creates a manual record.
- `GET /api/records/:id` returns one owned record.
- `PUT /api/records/:id` updates one owned record.
- `POST /api/records/:id/revoke` marks one owned record revoked.
- `DELETE /api/records/:id` deletes one owned record.

Services are shared by normalized domain; consent records remain private to the owning user.

## Extension

- `POST /api/extension/pair` creates a paired device and returns the raw token once.
- `GET /api/extension/devices` lists paired devices.
- `DELETE /api/extension/devices/:id` revokes a device.
- `POST /api/extension/records` creates a record with `Authorization: Bearer <device-token>`.

Only token hashes are stored.

## Export Formats

`GET /api/export.json` returns:

```json
{
  "format": "json",
  "records": []
}
```

`GET /api/export.csv` returns one row per record with these columns:

```text
id,service_name,website_url,category,consent_type,date_given,review_date,expiry_date,status,risk_level,source,notes,created_at,updated_at
```

Each export creates an `export.created` audit event.
