# Backend

## Runtime and modules

The backend is a Rust 2024 crate (`backend/Cargo.toml`). The `server` binary
starts the HTTP API by default and also exposes maintenance CLI commands. Axum
provides routing and extractors; Tokio runs asynchronous I/O; SQLx is configured
for Postgres; `utoipa` generates the gated OpenAPI document.

```text
backend/src/
├── main.rs                         process entrypoint / CLI dispatch
├── config.rs                       environment parsing and validation
├── bootstrap.rs                    repository + service + router composition
├── state.rs                        shared application state
├── domain/                         entities and repository traits
├── application/                    use cases and processing services
└── infrastructure/
    ├── http/                       router, handlers, middleware, OpenAPI
    └── persistence/
        ├── memory/                 local/test adapters
        └── postgres/               SQL-backed adapters
```

The router and bootstrap are shared by the production binary and integration
tests (`backend/tests/`). This lets tests exercise HTTP behavior without binding
network sockets.

## HTTP API groups

The canonical route declarations are in `backend/src/infrastructure/http/router.rs`.

- `/api/process*` — file parsing and profile/validate/convert/transform/diff/
  pipeline operations; `/api/generate/xlsx` creates workbooks.
- `/api/process/batch`, `/api/jobs/*` — asynchronous batches and result files.
- `/api/pdf/*` — inspect, extract text/forms, split/merge, and optional
  model-assisted extraction.
- `/api/ocr` — optional image OCR through a configured vision model.
- `/api/v1/*` — browser login/session, dashboard, users, roles, permissions,
  API-client administration, and job history.
- `/api/ext/auth/*` — external client token/refresh flow.
- `/health` and `/health/ready` — liveness and readiness probes.

OpenAPI JSON, Scalar, and Swagger routes are disabled in production by default;
`ENABLE_API_DOCS` controls this behavior.

## Processing pipeline

The processing service coordinates format detection, parsing, operations,
timing, and job persistence. Supported spreadsheet inputs include XLSX/XLS/ODS
and CSV. Spreadsheet parsing uses `calamine`; CSV has a native parser; the
count-only XLSX path streams row events with `quick-xml`. XLSX/ODS archive
expansion is bounded before large in-memory structures are built.

Operations can be called independently or ordered in a pipeline. Typical stages
are schema validation, row transformation, profile/statistics calculation, and
serialization to CSV/JSON/NDJSON/XLSX. Batch processing stores job metadata in
the configured repository and artifacts under `BATCH_BASE_DIR`.

PDF processing is implemented with `lopdf` and includes metadata/info, text and
form extraction, splitting and merging. Model-assisted extraction and image OCR
use configured HTTP clients and remain unavailable without an API key.

## Storage and startup

`bootstrap.rs` chooses storage adapters at startup:

- No `DATABASE_URL`: in-memory repositories; state is lost on restart.
- `DATABASE_URL` present: connect to Postgres, run SQLx migrations, seed the
  administrator and RBAC catalogue if missing, and reconcile interrupted jobs.
- `REDIS_URL` present and reachable: shared rate-limit counters. Otherwise,
  limits are process-local.

The configured SQL migrations are in `backend/migrations/`. Retention and job
pruning can be managed with the `server jobs prune` CLI; run `server check` to
inspect effective configuration and startup validation.

## Testing and checks

From the repository root, the Makefile offers `make test`, `make clippy`,
`make fmt`, `make audit`, and `make ci`. The integration test harness is
`backend/tests/common/mod.rs`; sample files live in `backend/tests/fixtures/`.
