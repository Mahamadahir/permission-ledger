# Deployment

Release 1 deploys as three pieces: a Rust API, a static SvelteKit dashboard
served by nginx, and PostgreSQL.

## Topology: one origin

The dashboard and the API are served from the **same origin**. nginx serves the
built SPA and proxies `/api` through to the backend (`web/nginx.conf`).

This is deliberate. The session cookie is `SameSite=Lax`, so if the dashboard and
the API were on genuinely different sites the browser would refuse to attach it
to `fetch` requests and every authenticated call would fail, even though login
appeared to succeed. Serving both from one origin keeps the cookie first-party,
removes CORS from the picture entirely, and means the browser never has to be
told to trust a second host.

The consequence is that `VITE_API_BASE` is empty by default and the client calls
relative paths like `/api/records`. Local development mirrors this: the Vite dev
server proxies `/api` to the backend (`web/vite.config.ts`), so development and
production behave the same way rather than only production hitting the proxy
path.

If you ever do need to split them across origins, you must also change the cookie
to `SameSite=None; Secure` in `backend/src/auth/mod.rs` and set `WEB_ORIGIN` to
the dashboard origin so the CORS layer allows credentialed requests.

## Local development

```bash
cp .env.example .env
docker compose up --build
```

Dashboard on `http://localhost:5173`, API on `http://localhost:3000`. The web
container runs the Vite dev server (`web/Dockerfile.dev`).

## Running the production stack locally

Verify the real topology before deploying it:

```bash
cp .env.prod.example .env
docker compose -f docker-compose.prod.yml up --build
```

The dashboard is on `http://localhost:8080` (override with `WEB_PORT`) and is the
only exposed service; the backend is reachable only through the proxy. The whole
end-to-end suite can be pointed at it, which is the best check that the proxy,
the SPA fallback and the cookies all work together:

```bash
cd e2e && WEB_URL=http://localhost:8080 npm test
```

## Environment

See `.env.prod.example`. The ones that matter:

| Variable | Why |
| --- | --- |
| `DATABASE_URL` | PostgreSQL connection string for the backend. |
| `WEB_ORIGIN` | Public origin of the dashboard, used for the CORS allow-list. |
| `COOKIE_SECURE` | **Must be `true` over HTTPS**, or the session cookie is sent without the `Secure` attribute. |
| `BACKEND_BIND_ADDR` | Address the API listens on, usually `0.0.0.0:3000`. |
| `VITE_API_BASE` | Leave empty for the same-origin setup. Only set it to split origins. |

## Migrations

The backend applies migrations at startup (`backend/src/main.rs`). sqlx takes a
Postgres advisory lock while it runs, so several replicas starting at once is
safe rather than racy.

The tradeoff is that a bad migration fails the container on boot. To apply them
as a separate step before rolling out new containers:

```bash
sqlx migrate run --source backend/migrations
```

## Demo data

```bash
node scripts/seed.mjs http://localhost:8080
```

Creates a demo account with a spread of records across categories, risk levels
and statuses so every dashboard metric and filter has something to show. Never
run it against a real deployment.

## Deploying to Azure Container Apps

The target is two Container Apps in one environment: the nginx web app with
public ingress, and the backend with internal ingress only. Both scale to zero,
so an idle app costs essentially nothing; the price is a few-second cold start on
the first request after idle. PostgreSQL is Neon's free tier. Images come from
GHCR. Everything is provisioned by `deploy/main.bicep`.

### 1. Database (Neon)

Create a project at [neon.tech](https://neon.tech) and copy the **direct**
connection string (not the `-pooler` host): the backend uses named prepared
statements, which Neon's transaction-mode pooler breaks. Make sure it ends with
`?sslmode=require`. The backend does TLS out of the box, so nothing else is
needed.

### 2. Build and publish the images

Push a version tag (or run the *Release images* workflow manually):

```bash
git tag v1.0.0 && git push origin v1.0.0
```

The workflow builds both images and pushes them to
`ghcr.io/<owner>/permission-ledger-backend` and `-web`. **Once**, in the repo's
Packages settings, set both packages to **public** so Container Apps can pull
them without a registry credential.

### 3. Provision

```bash
az login
az group create --name permission-ledger --location uksouth

az deployment group create \
  --resource-group permission-ledger \
  --template-file deploy/main.bicep \
  --parameters ghcrOwner=<owner> imageTag=v1.0.0 \
  --parameters databaseUrl='postgres://…?sslmode=require'
```

The `databaseUrl` parameter is `@secure`, so it is not logged. The template sets
`COOKIE_SECURE=true` and derives `WEB_ORIGIN` from the environment's domain, so
there is no post-deploy step. On first boot the backend applies the migrations
against Neon; watch it with:

```bash
az containerapp logs show -n permission-ledger-backend -g permission-ledger --follow
```

The deployment outputs `webUrl` — the public HTTPS address of the dashboard.

### 4. Verify

Open `webUrl`, register, add a record and reload: the session must survive,
which confirms `COOKIE_SECURE` and the same-origin cookie path work over real
HTTPS. Then run the whole suite against it:

```bash
cd e2e && WEB_URL=<webUrl> npm test
```

Populate a demo account with `node scripts/seed.mjs <webUrl>`.

### Redeploying

Push a new tag, then re-run the deployment with the new `imageTag`. Only the two
container images change; the environment and Neon stay put.

### Notes

- Only the web app has public ingress; the backend is reachable only from the web
  app inside the environment.
- A custom domain can be added later with
  `az containerapp hostname add` plus a managed certificate, and updating
  `WEB_ORIGIN`. Nothing else depends on the hostname.
- The API scaling to zero is exactly why the Release 2 privacy-policy checker must
  be a separate always-on worker or a scheduled Container Apps Job: a
  scaled-to-zero app has no process alive to run a nightly check.

## Screenshots

README images are generated from a seeded account rather than captured by hand:

```bash
node scripts/seed.mjs http://localhost:8080
cd e2e && WEB_URL=http://localhost:8080 npm run screenshots
```

They are written to `docs/images/`.
