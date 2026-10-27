# System architecture

## At a glance

Filers is a single Rust HTTP service and a single-page React application. Nginx
is the edge entrypoint in both Docker Compose environments. Postgres provides
durable application data when `DATABASE_URL` is configured; Redis optionally
shares per-IP rate-limit state across API processes.

```text
Browser / external API client
             │
             ▼
       Nginx :80/:443
        ┌────┴───────────────┐
        │                    │
        │ /                  │ /api/*, /health*
        ▼                    ▼
 React SPA / Vite       Axum API (:8000)
                             │
                  ┌──────────┼──────────┐
                  ▼          ▼          ▼
               Postgres    Redis    Local uploads
              (optional) (optional) (batch/results)
```

In development, Nginx proxies `/` to Vite (including HMR) and `/api` and health
routes to the Rust service. In production, Nginx serves the built SPA and
proxies API traffic. Only Nginx publishes host ports in the production Compose
stack.

## Backend boundaries

The backend follows a domain/application/infrastructure separation:

- **Domain** — entities and repository traits for authentication, API clients,
  RBAC, and processing jobs. It defines storage contracts, not database access.
- **Application** — use cases and services: auth, client credentials and
  quotas, spreadsheet processing, PDF operations, OCR, and webhooks.
- **Infrastructure** — Axum routes/handlers/middleware and the in-memory and
  Postgres repository implementations.
- **Bootstrap** — reads configuration, selects adapters, seeds initial records,
  builds services and creates the router. The same wiring is reusable by tests.

Repository interfaces allow in-memory stores when `DATABASE_URL` is absent and
Postgres stores when it is configured. Redis is independently optional; without
it, rate limits are process-local. See [backend design](backend.md).

## Request and file-processing flow

1. Nginx forwards the request to Axum. The API applies body limits, CORS,
   security headers, tracing, rate limiting, and endpoint-specific auth.
2. The handler validates the caller and multipart/file metadata, then invokes
   the application service.
3. The processing service identifies the file format and dispatches CSV or
   spreadsheet input to its parser. Spreadsheet parsing uses `calamine`; the
   streaming count path uses `quick-xml`. Limits guard upload size, uncompressed
   archive size, and maximum cells.
4. Operations can profile, validate, transform, convert, diff, or compose a
   pipeline. Results are returned as JSON or as a generated download.
5. Batch work records job status/history, writes result artifacts below
   `BATCH_BASE_DIR`, and can send a signed completion webhook.

PDF inspection and page operations use `lopdf`. OCR and PDF-to-JSON extraction
are optional external-model integrations: content leaves the service only when
the feature is configured and invoked. The trust boundary is described in
[operations and security](operations.md).

## Identity and access paths

The API supports distinct caller types:

- Static service keys for trusted integrations.
- OAuth2 client-credentials tokens for managed external API clients. Scopes,
  rate limits, page quotas, and AI-token quotas are client-specific.
- Browser sessions for the administration UI. Cookie-authenticated write
  requests use CSRF protection; administrative APIs enforce roles/permissions.

The frontend consumes the admin/session API. It does not replace server-side
authorization: UI route guards are for navigation and usability, while the API
is the security boundary.

## Persistence model

| Data | Adapter without Postgres | Adapter with Postgres |
| --- | --- | --- |
| Users and sessions | Process memory | Durable records |
| API clients and client tokens | Process memory | Durable records |
| Jobs and processing history | Process memory | Durable records and retention |
| Roles and permissions | Seeded in-memory catalogue | Seeded/editable SQL catalogue |
| Rate-limit counters | Process-local | Redis-shared when configured |
| Batch input/results | Filesystem | Filesystem/volume (not stored in SQL) |

SQL migrations live in `backend/migrations/` and run on startup when Postgres is
enabled. A restart in memory mode loses in-memory data. Batch artifacts must be
backed up separately if they need to survive container recreation.

## Design constraints

- API handlers should remain thin; business rules belong in application/domain
  code, and storage access should go through repository traits.
- CPU- and memory-intensive file work must respect configured limits and avoid
  unbounded materialization where streaming is available.
- In-memory mode is useful for local runs and tests, but is not durable and
  does not share state across API replicas.
- External model integrations are opt-in and must be treated as third-party
  data disclosure, not as an internal-only processing step.
