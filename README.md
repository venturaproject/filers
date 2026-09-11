# Filers

Fast spreadsheet processing API in Rust (axum) with a React admin panel.

Upload an `.xlsx` / `.xls` / `.ods` / `.csv` and get structured JSON back — plus
profiling, schema validation, transformation, format conversion, diffing and
multi-step pipelines. A ~1.1M-cell workbook parses in **~0.5 s** of server time;
the same job in a typical PHP stack is ~20 s.

The idea: an external application calls this API and receives the parsed file in
the response, offloading the heavy lifting to Rust.

---

## Contents

- [Features](#features)
- [Architecture](#architecture)
- [Quick start](#quick-start-dev)
- [Configuration](#configuration)
- [Authentication](#authentication)
- [Processing API](#processing-api)
- [PDF](#pdf)
- [Batch jobs](#batch-jobs)
- [Admin API](#admin-api-session-cookie)
- [OpenAPI docs](#openapi-docs)
- [Persistence](#persistence)
- [Security](#security)
- [Development](#development)
- [Production](#production)
- [Project layout](#project-layout)

---

## Features

**Processing**

- Parse `xlsx` / `xls` / `ods` / `csv` → `{ columns, data, stats, errors, timings }`
- Per-request **timing breakdown** (`open` / `read` / `convert` / `parse` wall-clock
  vs `parse_cpu` CPU time — so callers can tell real work from host CPU contention)
- **profile** — per-column type inference, cardinality, min/max/mean, top values
- **validate** — declarative schema (required, type, regex, enum, unique, length,
  range, unexpected/missing columns) → row-level errors
- **convert** — re-serialise to csv / json / ndjson / xlsx
- **transform** — cast → filter → select/drop → rename → offset/limit
- **diff** — key-based row comparison (added / removed / changed field-by-field)
- **pipeline** — ordered `validate` → `transform` → `profile` → `convert` steps
- **generate/xlsx** — build a styled workbook from JSON rows
- **batch** — process a directory of files in the background, with completion
  webhooks (HMAC-signed) and downloadable results

**Platform**

- Two auth models for the API (static service key **or** OAuth2 client-credentials),
  a third (session cookie) for the admin panel
- Per-client scopes, rate limits and monthly page quotas
- Refresh-token rotation with reuse detection (revokes the token family)
- Optional Postgres persistence (users, sessions, API clients, tokens, job
  history, RBAC catalogue); optional Redis-shared rate limiters
- Full job history + a monitoring dashboard (throughput, p95, error rate, 24h timeline)
- PDF: info / text / forms / split / merge (pure Rust). Gated OpenAPI spec + Scalar and Swagger UI
- Liveness / readiness probes, graceful shutdown, startup job reconciliation
- nginx as the single entrypoint (dev and prod)

**Performance**

- xlsx read via calamine's borrowed-range API (shared strings not cloned)
- cell → JSON conversion parallelised with rayon
- batch files fanned out across cores, bounded by a semaphore
- `count_only` parses (batch stats) **stream** `<row>` events with quick-xml —
  the dense matrix is never built; only the header row is decoded
- CSV streams natively; a zip-bomb guard (`MAX_UNCOMPRESSED_MB`) and a cell cap
  (`MAX_CELLS`) reject pathological files before anything is materialised

---

## Architecture

Domain-driven layout, in-memory repositories behind trait boundaries, Postgres
implementations swapped in at startup when `DATABASE_URL` is set.

```
backend/src/
  domain/         entities + repository traits (auth, rbac, api_client, processing)
  application/    services + use-cases (parsers, operations, pdf, notifier)
  infrastructure/
    http/         axum router, handlers, middleware, OpenAPI doc
    persistence/  memory/ + postgres/ implementations of the repo traits
  config.rs       env → Config, with fatal-in-production validation
  bootstrap.rs    Config → AppState → Router (shared by main + tests)
```

**Stack:** Rust 2024 · axum 0.7 · tokio · sqlx 0.8 (Postgres) · redis · calamine
· quick-xml · rayon · argon2 · utoipa. Frontend: React · Vite · TanStack
Query/Table · shadcn/ui · Tailwind · recharts.

---

## Quick start (dev)

Requires Docker.

```bash
cp .env.example .env
make up                       # nginx + api + frontend + postgres + redis, hot reload
```

Everything is behind one entrypoint: **http://localhost:8085**

| Path            | Serves                                   |
| --------------- | ---------------------------------------- |
| `/`             | React admin panel                       |
| `/api/*`        | Rust API                                 |
| `/api/docs` · `/api/swagger` | Scalar / Swagger UI (dev only by default) |
| `/health`       | liveness (process is up)                |
| `/health/ready` | readiness (DB reachable) — `503` when not |

Default admin login: **admin@filers.test** / **admin1234**
Default service key: **dev-key** (sent as `x-api-key`)

First parse:

```bash
curl -s -X POST "http://localhost:8085/api/process?max_rows=5" \
  -H "x-api-key: dev-key" \
  -F "file=@your-file.xlsx" | jq
```

Run without Docker: `cd backend && cargo run` (needs a reachable `DATABASE_URL`,
or leave it unset for the in-memory path) and `cd frontend && pnpm dev`.

---

## Configuration

All via environment (`.env` in dev). See `.env.example` for the annotated list.

| Variable | Default | Notes |
| --- | --- | --- |
| `PORT` | `8000` | API listen port |
| `APP_ENV` | – | `production` makes the config checks below **fatal** instead of warnings |
| `API_KEYS` | `change-me-in-production` | comma-separated service keys (`x-api-key`); empty disables them |
| `MAX_FILE_SIZE_MB` | `100` | upload cap (keep in sync with nginx `client_max_body_size`) |
| `MAX_CELLS` | `64000000` | reject a sheet whose `rows * cols` exceeds this before materialising it |
| `MAX_UNCOMPRESSED_MB` | `1024` | reject an xlsx/ods whose zip members sum to more than this uncompressed (zip-bomb guard) |
| `BATCH_BASE_DIR` | `./uploads` | root for `POST /api/process/batch`; `..` and absolute paths rejected |
| `DATABASE_URL` | derived from `POSTGRES_*` | Postgres for users/sessions/API-clients/tokens/**jobs**. Unset *and* no bundled Postgres → in-memory (lost on restart) |
| `POSTGRES_USER` / `POSTGRES_PASSWORD` / `POSTGRES_DB` | `filers` / `filers` / `filers` | credentials for the bundled Postgres (both compose files); ignored when `DATABASE_URL` is external. **Change the password for real deployments** |
| `REDIS_URL` | derived (bundled Redis) | shares the per-IP rate limiters across replicas; unset → process-local. Fails open if Redis is unreachable |
| `WEBHOOK_URL` / `WEBHOOK_SECRET` | – | default batch-completion webhook; secret enables `X-Filers-Signature: sha256=<hmac>` |
| `CORS_ALLOWED_ORIGINS` | `*` | comma-separated origins; `*` in prod disables credentialed cookie CORS |
| `TRUST_PROXY` | `true` | trust `X-Forwarded-For` / `X-Real-IP` (true behind the bundled nginx) |
| `AUTH_RATE_LIMIT` | `10/60` | per-IP throttle for `/api/v1/auth/*` and `/api/ext/auth/*` (`<n>/<seconds>`) — Redis-backed when `REDIS_URL` is set |
| `API_RATE_LIMIT` | `120/60` | per-IP throttle for every other `/api` route; `off` disables |
| `ENABLE_API_DOCS` | – | serve `/api/openapi.json` + `/api/docs` (Scalar) + `/api/swagger`. Unset → on outside production, off in production. `true`/`false` forces it |
| `APP_NAME` | `Filers` | public name at `GET /api/v1/config` |
| `SESSION_COOKIE_SECURE` | `false` | add `Secure` to the session cookie + emit HSTS (enable behind HTTPS) |
| `SEED_USER_EMAIL` / `_PASSWORD` / `_NAME` / `_API_KEY` | `admin@filers.test` / `admin1234` / `Admin` / first `API_KEYS` | seeded admin account |
| `SEED_DEMO_USERS` | `false` | seed `maria@` / `carlos@` (`demo1234`) — dev only, **fatal in production** |
| `EXT_DEFAULT_RATE_LIMIT` / `EXT_DEFAULT_MONTHLY_PAGE_QUOTA` | – | defaults for API clients that set none |
| `RUST_LOG` | `rust_api=info,tower_http=info` | tracing filter |

**Production startup checks** (fatal when `APP_ENV=production`): weak/default
`SEED_USER_PASSWORD`, a known-default `API_KEYS` entry, `SEED_DEMO_USERS=true`,
missing `DATABASE_URL`. Advisory always: `CORS_ALLOWED_ORIGINS=*`, non-`Secure`
cookie, `ENABLE_API_DOCS` on.

---

## Authentication

| Model | Header | Used by | Endpoints |
| --- | --- | --- | --- |
| **Service key** | `x-api-key: <key>` | internal / CI callers | `/api/process*`, `/api/generate/*`, `/api/jobs/*` |
| **OAuth2 client-credentials** | `Authorization: Bearer <token>` | external partners | same as above |
| **Session cookie** | `Set-Cookie: session=…` (HttpOnly, SameSite=Lax) | the admin panel | `/api/v1/*` |

Passwords are Argon2id. API tokens are stored SHA-256-hashed.

### OAuth2 flow (external clients)

Create a client in the admin panel (or `POST /api/v1/api-clients`), then:

```bash
# exchange credentials for tokens
curl -s -X POST http://localhost:8085/api/ext/auth/token \
  -H "Content-Type: application/json" \
  -d '{"client_id":"...","client_secret":"..."}'
# → { access_token, token_type: "Bearer", expires_in: 3600, refresh_token, scope }

# use it
curl -s -X POST http://localhost:8085/api/process \
  -H "Authorization: Bearer <access_token>" -F "file=@f.xlsx"

# rotate (refresh tokens are single-use; replay revokes the whole family)
curl -s -X POST http://localhost:8085/api/ext/auth/refresh \
  -H "Content-Type: application/json" -d '{"refresh_token":"..."}'
```

Access token lives 1 h, refresh token 30 d. Each client has a **scope**
(`files:read` and/or `files:write`), an optional **rate limit** and an optional
**monthly page quota** (→ `429` when exceeded).

| Scope | Grants |
| --- | --- |
| `files:read` | `profile`, `validate`, `diff`, reading jobs |
| `files:write` | `process`, `convert`, `transform`, `pipeline`, `batch`, `generate/xlsx` |

---

## Processing API

All operations take a **multipart** upload unless noted. Parse options are query
parameters: `sheet` (0-based), `skip_rows`, `has_headers` (default `true`),
`max_rows`, `offset`, `delimiter`.

### `POST /api/process`

Field `file`. Returns the parsed file.

```jsonc
{
  "format": "xlsx",
  "columns": ["Núm. Operación", "Zona", "..."],
  "data": [["1042", "Norte", "..."], ...],
  "stats": { "total_rows": 3104, "returned_rows": 3, "columns": 354, "elapsed_ms": 545 },
  "errors": [],
  "timings": {
    "open_ms": 38, "read_ms": 469, "convert_ms": 38,
    "parse_ms": 545, "parse_cpu_ms": 648, "upload_ms": 5, "total_ms": 550
  }
}
```

> `parse_ms` ≫ `parse_cpu_ms` means the host was starved of CPU — the work itself
> is cheap and the wall time is contention, not processing.

### `POST /api/process/profile`

Field `file`. → `{ report, stats, timings }` — per-column inferred type, nulls,
blanks, distinct count, min/max/mean, top-N values, samples.

### `POST /api/process/validate`

Fields `schema` (JSON string) + `file`.

```jsonc
// schema
{
  "columns": {
    "email":  { "required": true, "type": "string", "regex": "^[^@]+@[^@]+$", "unique": true },
    "age":    { "type": "integer", "min": 0, "max": 120 },
    "status": { "enum": ["active", "inactive"] }
  },
  "allow_extra_columns": false,
  "max_errors": 1000
}
```

→ `{ valid, errors: [{ row, column, rule, value, message }], ... }`

### `POST /api/process/convert?to=csv|json|ndjson|xlsx`

Field `file`. Also `out_delimiter` for csv. Returns the converted file as an
attachment.

### `POST /api/process/transform`

Fields `spec` (JSON string) + `file`. `?to=` returns a file instead of JSON.

```jsonc
{
  "select": ["id", "name", "total"],
  "rename": { "total": "amount" },
  "cast":   { "amount": "float" },
  "filter": { "amount": { "gte": 100 }, "status": { "in": ["paid"] } },
  "offset": 0, "limit": 500
}
```

→ `{ columns, data, stats, matched_rows }`

Filter predicates: `eq ne gt gte lt lte in not_in matches is_null not_null`.

### `POST /api/process/diff?key=id`

Fields `a` + `b`. `key` is one or more comma-separated columns.

→ `{ added, removed, changed: [{ key, changes: { col: { from, to } } }], unchanged, columns_added, columns_removed }` (detail capped at 5000).

### `POST /api/process/pipeline`

Fields `pipeline` (JSON string) + `file`.

```jsonc
{
  "steps": [
    { "op": "validate", "schema": { ... }, "fail_on_error": true },
    { "op": "transform", "spec": { ... } },
    { "op": "convert", "to": "csv" }
  ]
}
```

Runs in order. A failing `validate` (with `fail_on_error`) → `422` + step reports.
`convert` must be last and returns a file; otherwise the response is
`{ columns, data, stats, steps }`.

### `POST /api/generate/xlsx`

JSON body. `rows` as arrays (with `columns`) or objects.

```jsonc
{
  "columns": ["Name", "Total"],
  "rows": [["Acme", 1200], ["Globex", 980]],
  "options": { "sheet_name": "Report", "header_style": true, "freeze_header": true, "auto_filter": true }
}
```

---

## PDF

Pure-Rust (lopdf) — no native dependency. Text extraction is best-effort and
does **not** OCR: a scanned PDF yields little. Same multipart contract, scopes
(`files:read` for info/text/forms, `files:write` for split/merge) and job
tracing as the spreadsheet endpoints.

### `POST /api/pdf/info`

Field `file`. → page count, per-page size (points), metadata (title/author/…),
PDF version, `encrypted`, `has_form`.

### `POST /api/pdf/text?pages=1-3`

Field `file`. `pages` is a `1-3,7` selector (omit for all). →
`{ pages: [{ page, text, chars }], truncated }`.

### `POST /api/pdf/forms`

Field `file`. → `{ has_form, fields: [{ name, kind, value, label }] }` — AcroForm
fields (`kind` = text / button / choice / signature), hierarchical names as
`parent.child`.

### `POST /api/pdf/split?pages=1-3[&each=true]`

Field `file`. Returns the trimmed PDF, or — with `each=true` — a zip of
one-page PDFs.

### `POST /api/pdf/merge`

Repeat the `file` part two or more times → the concatenated PDF.

---

## Batch jobs

### `POST /api/process/batch`

```jsonc
{
  "path": "incoming/2026-02",          // relative to BATCH_BASE_DIR; optional
  "options": { "has_headers": true },
  "webhook_url": "https://you.example/hook",   // optional, overrides WEBHOOK_URL
  "output": { "to": "csv", "transform": { ... } }  // optional: write downloadable results
}
```

→ `{ "job_id": "…" }`. Returns immediately; the files are processed in the
background across cores.

### Polling & results

```bash
GET /api/jobs/:id                    # detail: status, per-file results, timings, outputs
GET /api/jobs/:id/results            # list generated files (only when `output` was set)
GET /api/jobs/:id/results/:name      # download one
```

Jobs are **owner-scoped** — a caller that didn't create a job gets `404` (not
`403`, so the id isn't confirmed). Service keys and admins see everything.

### Completion webhook

On completion the job's final state is `POST`ed as
`{ "event": "job.completed", "job": { … } }` with header `x-filers-event`. If
`WEBHOOK_SECRET` is set, also `X-Filers-Signature: sha256=<hmac-sha256>`.

### Every `POST /api/process*` call is also recorded

as a `kind: sync` job, so the admin has full traceability (filename, rows,
duration, origin, actor).

---

## Admin API (session cookie)

Under `/api/v1/*`, all requiring a logged-in session; management routes require
the `admin` role.

| Endpoint | Purpose |
| --- | --- |
| `POST /api/v1/auth/login` · `logout` · `refresh`, `GET /me`, `POST /me/avatar` | session auth |
| `GET /api/v1/config` · `GET /api/v1/csrf/` | bootstrap for the SPA |
| `GET /api/v1/dashboard` | throughput, p95, error rate, per-operation table, 24h timeline |
| `GET/POST /api/v1/jobs`, `GET /api/v1/jobs/:id`, `.../results[/:name]` | job history + batch trigger |
| `GET/PUT/DELETE /api/v1/users/:id`, `GET/POST /api/v1/users` | user CRUD |
| `.../roles`, `.../permissions` | RBAC catalogue |
| `.../api-clients`, `.../api-clients/:id/{usage,rotate}` | external client management |

The admin panel is **monitoring + management** — it does not run processing
itself; that's the API's job.

---

## OpenAPI docs

When `ENABLE_API_DOCS` is on:

- `GET /api/openapi.json` — the spec (public processing + PDF API; the
  `/api/v1/*` admin routes are excluded)
- `GET /api/docs` — [Scalar](https://scalar.com) UI
- `GET /api/swagger` — Swagger UI (classic "try it out" forms)

Both UIs point at the same `/api/openapi.json`; you can also paste that URL into
Postman / Insomnia / an external Swagger or Redoc instance.

Default: **on** outside production, **off** in production. Set `ENABLE_API_DOCS=true`
to force it on (allowed in production, but the startup log warns).

---

## Persistence

| Store | default (URL unset) | with the backing service |
| --- | --- | --- |
| users, sessions, API clients, client tokens | in-memory (lost on restart) | **Postgres** (`DATABASE_URL`) |
| job / processing history | in-memory (cap 512, oldest evicted) | **Postgres** (`DATABASE_URL`, 30-day retention) |
| roles & permissions | in-memory (seeded catalogue) | **Postgres** (`DATABASE_URL`; seeded once, then editable and durable) |
| per-IP rate-limit counters | process-local (`DashMap`) | **Redis** (`REDIS_URL`; shared across replicas) |

The RBAC catalogue (`resource.action` permissions + the `admin` / `user` roles)
is seeded on first connect when the tables are empty, then persists — so edits
through `/api/v1/roles` and `/api/v1/permissions` survive a restart. The `admin`
role cannot be deleted.

Migrations in `backend/migrations/` run automatically on connect (`sqlx::migrate!`).
Both `compose.dev.yml` and `compose.prod.yml` ship a `postgres:17-alpine` and
point the API at it by default; set `DATABASE_URL` in `.env` to use an external
instance instead.

On startup any job left `pending` / `running` by a previous crash or restart is
reconciled to `failed` ("interrupted by a server restart") — a batch task that
was mid-flight when the process died no longer shows as running forever. The
server also drains in-flight requests on SIGTERM / Ctrl-C before exiting.

---

## Security

- **Refuse-to-boot** in production on weak seed credentials / default keys /
  demo users / missing DB
- Argon2id passwords; opaque HttpOnly / SameSite=Lax session cookies; SHA-256
  hashed API tokens; constant-time key comparison
- Per-IP rate limiting on auth endpoints **and** (app-level) every other `/api`
  route; nginx `limit_req` on top. Shared across replicas via Redis when
  `REDIS_URL` is set, process-local otherwise (fails open if Redis is down)
- Refresh-token single-use + **reuse detection** → revokes the token family
- Suspended / inactive users are locked out and their sessions dropped on
  status/password/role change
- Upload hardening: streaming size guard, `DefaultBodyLimit`, magic-byte sniff
  before the parser, configurable cell cap (`MAX_CELLS`), zip-bomb guard
  (`MAX_UNCOMPRESSED_MB`, read from the central directory), batch file cap (500)
- Batch path-traversal guard (`..` / absolute paths rejected, `Component::Normal`
  only)
- Job reads scoped to the creator (404, no existence oracle)
- Security headers middleware; HSTS when `SESSION_COOKIE_SECURE=true`

---

## Development

```bash
make up          # dev stack (hot reload)
make down
make shell       # shell in the api container
make logs        # tail api logs

make test        # cargo test in the container
make test-local  # cargo test on the host
make ci          # fmt --check + clippy -D warnings + test + audit
make clippy
make fmt
make audit       # cargo audit — RustSec advisory scan (cargo install cargo-audit)
```

`cargo audit` is clean. One advisory (`RUSTSEC-2023-0071`, rsa) is ignored in
`.cargo/audit.toml` with a rationale — `rsa` reaches `Cargo.lock` only through
`sqlx-mysql`, which is not compiled (sqlx is built `postgres`-only).

Frontend checks (in the container):

```bash
docker compose -f compose.dev.yml exec -T frontend sh -c \
  'cd /app && pnpm exec oxlint src && pnpm exec tsc --noEmit && pnpm build'
```

**Tests:** 89 (unit + integration), driven through the router with
`tower::ServiceExt::oneshot` (no sockets). `backend/tests/common/mod.rs` is the
harness; fixtures in `backend/tests/fixtures/`. `backend/tests/perf.rs` is an
`#[ignore]`d timing harness (`cargo test --release --test perf -- --ignored
--nocapture`, reads `backend/input.xlsx` if present).

---

## Maintenance CLI

The `server` binary is also a CLI. With no subcommand it runs the HTTP server
(the default); the subcommands are one-shot tasks.

```bash
server                                  # run the HTTP server
server check                            # print effective config + run startup checks
server jobs prune --days 7              # delete completed/failed jobs older than 7 days
server jobs prune --days 30 --dry-run   # list what would be deleted, delete nothing
```

`jobs prune` keeps running jobs and needs `DATABASE_URL` (the in-memory history
resets on restart anyway). Schedule it from cron for durable retention:

```cron
# 03:15 daily — trim job history to 14 days
15 3 * * *  docker compose -f /srv/filers/compose.prod.yml run --rm --no-deps api jobs prune --days 14
```

In dev: `make prune DAYS=14` (add `DRY=1` to preview), `make check-config`.

---

## Production

```bash
cp .env.example .env
# edit: APP_ENV=production, strong SEED_USER_PASSWORD, real API_KEYS,
#       strong POSTGRES_PASSWORD, CORS_ALLOWED_ORIGINS=https://your.domain,
#       SEED_DEMO_USERS=false
make prod-up      # postgres + redis + api + nginx (80/443), only nginx exposed
```

`compose.prod.yml` runs four services: a bundled **`postgres:17-alpine`** (named
volume `postgres-data`), a bundled **`redis:7-alpine`** (rate-limiter state, no
persistence), the **API** (health-checked, waits for both to be healthy), and
**nginx** — the only one with published ports. Postgres, Redis and the API sit
on the internal network only. The nginx image ships a self-signed cert so HTTPS
works immediately; mount a real one at `/etc/nginx/certs/{fullchain,privkey}.pem`
to replace it. The API runs with `TRUST_PROXY=true`, `SESSION_COOKIE_SECURE=true`
and `BATCH_BASE_DIR=/data/uploads`, and drains in-flight requests on SIGTERM.

`DATABASE_URL` and `REDIS_URL` are derived from `POSTGRES_*` and the `redis`
service. To use an **external** managed Postgres or Redis instead, set the URL
explicitly in `.env` (the bundled service is then unused).

**Scaling to multiple API replicas:** users / sessions / API-clients / tokens /
jobs / RBAC are all in Postgres, and the rate limiters are shared via Redis, so
replicas are stateless. The one exception is batch **result files** on local
disk (`BATCH_BASE_DIR`) — mount shared storage or run batch on a single replica.

---

## Project layout

```
.
├── backend/                    Rust API
│   ├── src/                    see Architecture
│   ├── migrations/             sqlx migrations (auth, refresh-reuse, jobs, rbac)
│   ├── tests/                  integration tests + fixtures + perf harness
│   ├── .cargo/audit.toml       cargo-audit ignore list (with rationale)
│   ├── Cargo.toml / Cargo.lock
│   └── uploads/                batch results (BATCH_BASE_DIR), git-ignored
├── frontend/                   React admin panel (Vite)
├── infrastructure/
│   ├── Dockerfile              API release image (builds ./backend)
│   ├── Dockerfile.dev          API dev image (cargo-watch)
│   ├── frontend/Dockerfile.dev
│   └── nginx/                  nginx.conf (prod, TLS) + nginx.dev.conf + Dockerfile
├── compose.dev.yml             nginx + api + frontend + postgres + redis, hot reload
├── compose.prod.yml            nginx (TLS) + api + postgres + redis, only nginx exposed
├── Makefile
└── .env.example
```
