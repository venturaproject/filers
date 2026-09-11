-- Auth-critical tables. RBAC (roles/permissions) and job history stay in-memory
-- for now — see the persistence notes in the README.

CREATE TABLE IF NOT EXISTS users (
    id            UUID PRIMARY KEY,
    name          TEXT        NOT NULL,
    username      TEXT,
    email         TEXT        NOT NULL UNIQUE,
    password_hash TEXT        NOT NULL,
    role          TEXT        NOT NULL,
    role_names    TEXT[]      NOT NULL DEFAULT '{}',
    status        TEXT        NOT NULL DEFAULT 'active',
    api_key       TEXT        NOT NULL UNIQUE,
    permissions   TEXT[]      NOT NULL DEFAULT '{}',
    avatar        TEXT,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS sessions (
    token      TEXT        PRIMARY KEY,
    user_id    UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL
);
CREATE INDEX IF NOT EXISTS sessions_user_id_idx ON sessions(user_id);
CREATE INDEX IF NOT EXISTS sessions_expires_at_idx ON sessions(expires_at);

CREATE TABLE IF NOT EXISTS api_clients (
    id                 UUID PRIMARY KEY,
    name               TEXT        NOT NULL,
    client_id          TEXT        NOT NULL UNIQUE,
    secret_hash        TEXT        NOT NULL,
    scopes             TEXT[]      NOT NULL DEFAULT '{}',
    active             BOOLEAN     NOT NULL DEFAULT TRUE,
    rate_limit_count   INTEGER,
    rate_limit_window  BIGINT,
    monthly_page_quota BIGINT,
    last_used_at       TIMESTAMPTZ,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    usage_period       TEXT        NOT NULL,
    usage_requests     BIGINT      NOT NULL DEFAULT 0,
    usage_pages        BIGINT      NOT NULL DEFAULT 0,
    window_start       TIMESTAMPTZ NOT NULL DEFAULT now(),
    window_count       INTEGER     NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS client_tokens (
    access_hash        TEXT        PRIMARY KEY,
    client_id          UUID        NOT NULL REFERENCES api_clients(id) ON DELETE CASCADE,
    scopes             TEXT[]      NOT NULL DEFAULT '{}',
    refresh_hash       TEXT        NOT NULL,
    access_expires_at  TIMESTAMPTZ NOT NULL,
    refresh_expires_at TIMESTAMPTZ NOT NULL
);
CREATE INDEX IF NOT EXISTS client_tokens_refresh_hash_idx ON client_tokens(refresh_hash);
CREATE INDEX IF NOT EXISTS client_tokens_client_id_idx ON client_tokens(client_id);
