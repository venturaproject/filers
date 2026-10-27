# Operations and security

## Docker environments

`compose.dev.yml` runs Nginx, the Rust API, Vite, Postgres, and Redis. Source
directories are mounted for hot reload; Nginx exposes `http://localhost:8085`.
Use `make up`, `make down`, and `make logs` for the common development workflow.

`compose.prod.yml` runs the API, Postgres, Redis, and Nginx. Nginx is the only
service publishing ports; it serves the built frontend and reverse-proxies API
requests. The production API image is multi-stage, runs as a non-root user, and
stores batch data in the `uploads` volume. Build/start with `make prod-up`; use
`make prod-down` to stop it.

The production Nginx configuration expects certificate/key files under
`infrastructure/nginx/certs/`. The shipped deployment setup is intended to be
configured with real certificates or placed behind a TLS-terminating load
balancer. Review the Nginx config and `.env.example` before exposure to the
public internet.

## Configuration checklist

Configuration is loaded from environment variables (normally `.env` for local
Compose). `.env.example` and the configuration table in the root README are the
reference lists. Before production:

- Set strong, unique database and administrator credentials and replace default
  service API keys.
- Keep `SEED_DEMO_USERS` disabled and configure `APP_ENV=production`.
- Provide durable Postgres and protect/backup the uploads volume.
- Set `CORS_ALLOWED_ORIGINS` to the application origins actually in use.
- Enable secure session cookies behind HTTPS; preserve correct forwarded
  headers when terminating TLS upstream.
- Decide whether OpenAPI docs should be exposed (`ENABLE_API_DOCS`).
- Configure `MAX_FILE_SIZE_MB`, `MAX_CELLS`, and
  `MAX_UNCOMPRESSED_MB` to fit available memory and workload.
- Treat Redis as shared throttling state, not as durable application storage.

The API's production config validation rejects selected insecure defaults; this
does not replace an environment-specific deployment review.

## Security boundaries

- Service-key and OAuth client-credential APIs are distinct from browser
  session-cookie APIs. API scopes, admin roles, CSRF checks, and rate limiting
  are enforced server-side.
- Nginx and Axum both apply request/rate controls. Redis-backed rate limits are
  shared when configured; the in-process fallback is not shared across replicas.
- File inputs are untrusted. Upload limits, archive expansion checks, cell
  limits, format sniffing, and batch path checks reduce resource and traversal
  risks. Keep Nginx's body-size limit aligned with `MAX_FILE_SIZE_MB`.
- OCR and PDF extraction send submitted content to an external model provider
  when enabled. Do not enable them without an approved provider and data policy.
  Image metadata is stripped before OCR; optional PII redaction applies to text,
  not to sensitive content embedded in image pixels.
- Do not commit `.env`, credentials, user uploads, generated results, or private
  TLS keys/certificates.

## Observability and maintenance

`RUST_LOG` controls tracing filters. `/health` is liveness; `/health/ready`
checks Postgres when persistence is enabled. On startup the API reconciles
unfinished persisted jobs after a restart. The `server jobs prune` command
supports a dry run; use it to preview retention cleanup before deleting records.

Useful checks:

```bash
make check-config
make logs
make test
make ci
```

See the [root README](../README.md) for the full environment variable reference,
API examples, and Compose quick start.
