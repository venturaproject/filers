-- Refresh-token reuse detection: keep spent tokens briefly and flag a replay.
ALTER TABLE client_tokens ADD COLUMN IF NOT EXISTS consumed_at TIMESTAMPTZ;
