# Deployment

Release 1 runs as a Rust API, SvelteKit web app, and PostgreSQL database.

## Local Docker Compose

```bash
cp .env.example .env
docker compose up --build
```

The backend listens on `http://localhost:3000` and applies SQLx migrations at startup. The web dashboard runs on `http://localhost:5173`.

Build the Chrome extension bundle with:

```bash
docker compose --profile tools run --rm extension-build
```

Load `extension/dist` as an unpacked extension in Chrome.

## Production Notes

Set `COOKIE_SECURE=true`, use HTTPS-only origins, and set `WEB_ORIGIN` to the deployed dashboard origin. Run migrations before or during backend startup against the production PostgreSQL database.
