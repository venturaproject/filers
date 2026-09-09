-- Processing job history. The full Job is stored as JSONB; a few columns are
-- lifted out for filtering/sorting and for the owner scoping that `#[serde(skip)]`
-- keeps out of the JSON payload.
CREATE TABLE IF NOT EXISTS jobs (
    id           UUID PRIMARY KEY,
    status       TEXT        NOT NULL,
    kind         TEXT        NOT NULL,
    origin       TEXT        NOT NULL,
    operation    TEXT        NOT NULL,
    owner        TEXT,
    actor        TEXT,
    created_at   TIMESTAMPTZ NOT NULL,
    completed_at TIMESTAMPTZ,
    data         JSONB       NOT NULL
);

CREATE INDEX IF NOT EXISTS jobs_created_at_idx ON jobs (created_at DESC);
CREATE INDEX IF NOT EXISTS jobs_owner_idx      ON jobs (owner);
